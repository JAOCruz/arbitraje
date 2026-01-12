use super::{BookUpdate, Exchange, ExchangeConnector};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct BinanceConnector {
    tx: Option<mpsc::Sender<BookUpdate>>,
    rx: Option<mpsc::Receiver<BookUpdate>>,
}

impl BinanceConnector {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self { tx: Some(tx), rx: Some(rx) }
    }
}

#[async_trait]
impl ExchangeConnector for BinanceConnector {
    fn name(&self) -> Exchange {
        Exchange::Binance
    }

    async fn connect(&mut self, symbols: Vec<String>) -> Result<()> {
        let tx = self.tx.clone().unwrap();
        
        // Usamos la URL base limpia. La suscripción se hace via JSON después.
        let url = "wss://fstream.binance.com/ws";

        tokio::spawn(async move {
            loop {
                tracing::info!("🔌 Connecting to Binance Futures...");
                
                match connect_async(url).await {
                    Ok((ws_stream, _)) => {
                        tracing::info!("✅ Connected to Binance WS");
                        let (mut write, mut read) = ws_stream.split();

                        // 1. Convertir símbolos (BTC-USDT -> btcusdt)
                        let params: Vec<String> = symbols.iter()
                            .map(|s| format!("{}@bookTicker", s.replace("-", "").to_lowercase()))
                            .collect();

                        // 2. Enviar Suscripción Inmediata
                        let id = chrono::Utc::now().timestamp_millis();
                        let subscribe_msg = json!({
                            "method": "SUBSCRIBE",
                            "params": params,
                            "id": id
                        });

                        if let Err(e) = write.send(Message::Text(subscribe_msg.to_string())).await {
                            tracing::error!("❌ Failed to subscribe Binance: {:?}", e);
                            // Si falla enviar suscripción, forzamos reconexión
                        } else {
                            tracing::info!("📡 Subscribed to {} symbols on Binance", params.len());
                            
                            // 3. Loop de lectura
                            while let Some(msg) = read.next().await {
                                match msg {
                                    Ok(Message::Text(text)) => {
                                        if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                            // Ignorar respuestas de control (id, null result)
                                            if json.get("id").is_some() { continue; }

                                            // Parsear BookTicker
                                            if let (Some(s), Some(bid), Some(ask), Some(bid_qty), Some(ask_qty)) = (
                                                json.get("s").and_then(|v| v.as_str()),
                                                json.get("b").and_then(|v| v.as_str()),
                                                json.get("a").and_then(|v| v.as_str()),
                                                json.get("B").and_then(|v| v.as_str()),
                                                json.get("A").and_then(|v| v.as_str())
                                            ) {
                                                // Convertir btcusdt -> BTC-USDT
                                                let symbol_std = format!("{}-{}", &s[0..s.len()-4], "USDT");
                                                
                                                if let (Ok(bid_p), Ok(ask_p), Ok(bid_sz), Ok(ask_sz)) = (
                                                    bid.parse::<f64>(), 
                                                    ask.parse::<f64>(),
                                                    bid_qty.parse::<f64>(),
                                                    ask_qty.parse::<f64>()
                                                ) {
                                                    let _ = tx.send(BookUpdate {
                                                        symbol: symbol_std,
                                                        exchange: Exchange::Binance,
                                                        bid: bid_p,
                                                        ask: ask_p,
                                                        bid_size: bid_sz,
                                                        ask_size: ask_sz,
                                                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                                    }).await;
                                                }
                                            }
                                        }
                                    }
                                    Ok(Message::Ping(payload)) => {
                                        // Responder Pongs es vital para no ser desconectado
                                        let _ = write.send(Message::Pong(payload)).await;
                                    }
                                    Ok(Message::Close(_)) => {
                                        tracing::warn!("⚠️ Binance connection closed by server");
                                        break;
                                    }
                                    Err(_) => break, // Error de socket
                                    _ => {}
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ Binance Connection Failed: {:?}", e);
                    }
                }
                
                tracing::warn!("🔄 Reconnecting to Binance in 2s...");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });

        Ok(())
    }

    fn get_receiver(&mut self) -> mpsc::Receiver<BookUpdate> {
        self.rx.take().expect("Receiver already taken")
    }
}