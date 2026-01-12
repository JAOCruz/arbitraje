use super::{BookUpdate, Exchange, ExchangeConnector};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct HyperliquidConnector {
    tx: Option<mpsc::Sender<BookUpdate>>,
    rx: Option<mpsc::Receiver<BookUpdate>>,
}

impl HyperliquidConnector {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self { tx: Some(tx), rx: Some(rx) }
    }
}

#[async_trait]
impl ExchangeConnector for HyperliquidConnector {
    fn name(&self) -> Exchange {
        Exchange::Hyperliquid
    }

    async fn connect(&mut self, symbols: Vec<String>) -> Result<()> {
        let tx = self.tx.clone().unwrap();
        
        // Hyperliquid usa UNA sola conexión para todo (Multiplexing)
        tokio::spawn(async move {
            let url = "wss://api.hyperliquid.xyz/ws";
            
            match connect_async(url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("✅ Connected to Hyperliquid Mainnet");
                    let (mut write, mut read) = ws_stream.split();

                    // 1. Suscribirse a cada símbolo
                    for symbol in symbols {
                        // FIX: Hyperliquid espera "BTC", no "BTC-USDT"
                        let coin = symbol.split('-').next().unwrap_or(&symbol);
                        
                        let sub_msg = json!({
                            "type": "subscribe",
                            "subscription": {
                                "type": "l2Book",
                                "coin": coin 
                            }
                        });
                        
                        if let Err(e) = write.send(Message::Text(sub_msg.to_string())).await {
                            tracing::error!("❌ Error enviando suscripción HL: {:?}", e);
                        }
                    }

                    // 2. Loop de lectura
                    while let Some(msg) = read.next().await {
                        if let Ok(Message::Text(text)) = msg {
                            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                // "channel": "l2Book", "data": { "levels": [[p, s], ...], "coin": "BTC" }
                                if let Some(data) = json.get("data") {
                                    if let Some(coin) = data.get("coin").and_then(|c| c.as_str()) {
                                        let levels = data.get("levels");
                                        // Lógica simplificada de parsing...
                                        // Debes mapear 'coin' de vuelta a 'BTC-USDT' para tu sistema
                                        let symbol_map = format!("{}-USDT", coin); 
                                        
                                        if let Some(l) = levels {
                                            // Extraer bid/ask del snapshot
                                            // Enviar por tx.send(...)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => tracing::error!("❌ Hyperliquid Connect Error: {:?}", e),
            }
        });

        Ok(())
    }

    fn get_receiver(&mut self) -> mpsc::Receiver<BookUpdate> {
        self.rx.take().expect("Receiver already taken")
    }
}