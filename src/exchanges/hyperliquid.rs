use super::{Exchange, ExchangeConnector, Price};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct HyperliquidConnector {
    price_tx: Option<mpsc::Sender<Price>>,
    price_rx: Option<mpsc::Receiver<Price>>,
}

impl HyperliquidConnector {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self {
            price_tx: Some(tx),
            price_rx: Some(rx),
        }
    }

    fn normalize_symbol(symbol: &str) -> String {
        // Convertir BTC-USDT -> BTC
        symbol.split('-').next().unwrap_or(symbol).to_string()
    }
}

#[async_trait]
impl ExchangeConnector for HyperliquidConnector {
    fn name(&self) -> Exchange {
        Exchange::Hyperliquid
    }

    async fn connect(&mut self, symbols: Vec<String>) -> Result<()> {
        let url = "wss://api.hyperliquid.xyz/ws";

        tracing::info!("🔌 Connecting to Hyperliquid: {}", url);

        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        tracing::info!("✅ Connected to Hyperliquid");

        // Subscribe to allMids (todos los precios mid)
        let subscribe_msg = json!({
            "method": "subscribe",
            "subscription": {
                "type": "allMids"
            }
        });

        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await?;

        tracing::info!("📡 Subscribed to Hyperliquid allMids");

        let tx = self.price_tx.clone().unwrap();
        let tracked_symbols: Vec<String> = symbols
            .iter()
            .map(|s| Self::normalize_symbol(s))
            .collect();

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            // Hyperliquid allMids format: { "channel": "allMids", "data": { "mids": { "BTC": "91234.5", ... } } }
                            if let Some(mids) = data["data"]["mids"].as_object() {
                                let timestamp = chrono::Utc::now().timestamp_millis() as u64;

                                for (symbol, price_value) in mids {
                                    // Solo procesar símbolos que nos interesan
                                    if tracked_symbols.contains(symbol) {
                                        if let Some(price_str) = price_value.as_str() {
                                            if let Ok(price) = price_str.parse::<f64>() {
                                                let price_update = Price {
                                                    symbol: format!("{}-USDT", symbol), // Normalizar a formato común
                                                    exchange: Exchange::Hyperliquid,
                                                    price,
                                                    timestamp,
                                                };

                                                let _ = tx.send(price_update).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("❌ Hyperliquid WebSocket error: {:?}", e);
                        break;
                    }
                }
            }

            tracing::warn!("⚠️  Hyperliquid connection closed");
        });

        Ok(())
    }

    fn get_price_receiver(&mut self) -> mpsc::Receiver<Price> {
        self.price_rx.take().expect("Receiver already taken")
    }
}
