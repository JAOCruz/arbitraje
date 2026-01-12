use super::{BookUpdate, Exchange, ExchangeConnector};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::Value;
// CORRECCIÓN: Importaciones separadas para mpsc y broadcast
use tokio::sync::{mpsc, broadcast}; 
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest, tungstenite::Message};
use http::HeaderValue;
use tracing::{info, warn, error};
use std::time::Duration;

pub struct ExtendedConnector {
    tx: Option<mpsc::Sender<BookUpdate>>,
    rx: Option<mpsc::Receiver<BookUpdate>>,
}

impl ExtendedConnector {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self { tx: Some(tx), rx: Some(rx) }
    }

    fn normalize_symbol(symbol: &str) -> String {
        if symbol.ends_with("USDT") {
            symbol.replace("USDT", "USD")
        } else {
            symbol.to_string()
        }
    }
}

#[async_trait]
impl ExchangeConnector for ExtendedConnector {
    fn name(&self) -> Exchange {
        Exchange::Extended
    }

    async fn connect(&mut self, symbols: Vec<String>) -> Result<()> {
        let tx_base = self.tx.clone().unwrap();

        for symbol in symbols {
            let market = Self::normalize_symbol(&symbol);
            let tx = tx_base.clone();
            let safe_symbol = symbol.clone();

            let url_str = format!(
                "wss://api.starknet.extended.exchange/stream.extended.exchange/v1/orderbooks/{}?depth=1",
                market
            );

            // Bucle de reconexión dentro del spawn
            tokio::spawn(async move {
                loop {
                    let mut request = match url_str.clone().into_client_request() {
                        Ok(req) => req,
                        Err(_) => break,
                    };

                    request.headers_mut().insert("User-Agent", HeaderValue::from_static("Mozilla/5.0..."));
                    
                    info!("🔌 Connecting to Extended: {}", safe_symbol);

                    if let Ok((ws_stream, _)) = connect_async(request).await {
                        let (_, mut read) = ws_stream.split();
                        while let Some(msg) = read.next().await {
                            if let Ok(Message::Text(text)) = msg {
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    if let Some(data) = json.get("data") {
                                        let ts = json.get("ts").and_then(|t| t.as_u64()).unwrap_or(0);
                                        let bids = data.get("b");
                                        let asks = data.get("a");
                                    
                                        // Función interna para extraer el mejor precio
                                        let get_top = |list: Option<&Value>| -> Option<(f64, f64)> {
                                            let items = list?.as_array()?;
                                            let top = items.get(0)?;
                                            let p = top.get("p")?.as_str()?.parse::<f64>().ok()?;
                                            let q = top.get("q")?.as_str()?.parse::<f64>().ok()?;
                                            Some((p, q))
                                        };
                                    
                                        if let (Some((bid, bid_sz)), Some((ask, ask_sz))) = (get_top(bids), get_top(asks)) {
                                            // ESTA ES LA LÍNEA QUE FALTA O ESTÁ FALLANDO:
                                            let _ = tx.send(BookUpdate {
                                                symbol: safe_symbol.clone(),
                                                exchange: Exchange::Extended,
                                                bid,
                                                ask,
                                                bid_size: bid_sz,
                                                ask_size: ask_sz,
                                                timestamp: ts,
                                            }).await; 
                                        }
                                    }
                                }
                            }
                        }
                        warn!("⚠️ Connection lost for {}. Retrying...", safe_symbol);
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });
        }
        Ok(())
    }

    fn get_receiver(&mut self) -> mpsc::Receiver<BookUpdate> {
        self.rx.take().expect("Receiver already taken")
    }
}