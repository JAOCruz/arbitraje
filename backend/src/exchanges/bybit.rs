use super::{BookUpdate, Exchange, ExchangeConnector};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct BybitConnector {
    tx: Option<mpsc::Sender<BookUpdate>>,
    rx: Option<mpsc::Receiver<BookUpdate>>,
}

impl BybitConnector {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self { tx: Some(tx), rx: Some(rx) }
    }

    fn normalize_symbol(symbol: &str) -> String {
        symbol.replace("-", "")
    }

    fn denormalize_symbol(bybit_symbol: &str) -> String {
        if bybit_symbol.ends_with("USDT") {
            let base = &bybit_symbol[0..bybit_symbol.len() - 4];
            format!("{}-USDT", base)
        } else {
            bybit_symbol.to_string()
        }
    }
}

#[async_trait]
impl ExchangeConnector for BybitConnector {
    fn name(&self) -> Exchange {
        Exchange::Bybit
    }

    async fn connect(&mut self, symbols: Vec<String>) -> Result<()> {
        let url = "wss://stream.bybit.com/v5/public/linear";
        tracing::info!("🔌 Connecting to Bybit V5 (Linear)...");

        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        let args: Vec<String> = symbols
            .iter()
            .map(|s| format!("orderbook.1.{}", Self::normalize_symbol(s)))
            .collect();

        let subscribe_msg = json!({
            "op": "subscribe",
            "args": args
        });

        write.send(Message::Text(subscribe_msg.to_string())).await?;
        tracing::info!("📡 Subscribed to Bybit Orderbooks");

        let tx = self.tx.clone().unwrap();

        tokio::spawn(async move {
            // Ping cada 20 segundos para mantener la conexión viva
            let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(20));

            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        let ping_msg = json!({"op": "ping"});
                        if let Err(e) = write.send(Message::Text(ping_msg.to_string())).await {
                            tracing::error!("Fallo al enviar Ping a Bybit: {:?}", e);
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    // Ignorar pong y confirmaciones
                                    if json.get("op").map(|s| s == "pong").unwrap_or(false) { continue; }
                                    if json.get("op").is_some() || json.get("success").is_some() { continue; }

                                    let ts = json.get("ts").and_then(|t| t.as_u64()).unwrap_or_else(|| chrono::Utc::now().timestamp_millis() as u64);

                                    if let Some(data) = json.get("data") {
                                        if let Some(s) = data["s"].as_str() {
                                            let bids = &data["b"];
                                            let asks = &data["a"];
                                            let get_data = |list: &Value| -> Option<(f64, f64)> {
                                                let item = list.get(0)?; 
                                                let p = item.get(0)?.as_str()?.parse().ok()?;
                                                let sz = item.get(1)?.as_str()?.parse().ok()?;
                                                Some((p, sz))
                                            };
                                            if let (Some((bid, bid_sz)), Some((ask, ask_sz))) = (get_data(bids), get_data(asks)) {
                                                let update = BookUpdate {
                                                    symbol: Self::denormalize_symbol(s),
                                                    exchange: Exchange::Bybit,
                                                    bid, ask, bid_size: bid_sz, ask_size: ask_sz, timestamp: ts,
                                                };
                                                let _ = tx.send(update).await;
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("❌ Bybit WS Error: {:?}", e);
                                break;
                            }
                            None => {
                                tracing::warn!("⚠️ Bybit connection closed");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn get_receiver(&mut self) -> mpsc::Receiver<BookUpdate> {
        self.rx.take().expect("Receiver already taken")
    }
}