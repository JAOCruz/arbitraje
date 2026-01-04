use super::{Exchange, ExchangeConnector, Price};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct BinanceConnector {
    price_tx: Option<mpsc::Sender<Price>>,
    price_rx: Option<mpsc::Receiver<Price>>,
}

impl BinanceConnector {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self {
            price_tx: Some(tx),
            price_rx: Some(rx),
        }
    }

    fn normalize_symbol(symbol: &str) -> String {
        // Convertir BTC-USDT -> btcusdt
        symbol.replace("-", "").to_lowercase()
    }
}

#[async_trait]
impl ExchangeConnector for BinanceConnector {
    fn name(&self) -> Exchange {
        Exchange::Binance
    }

    async fn connect(&mut self, symbols: Vec<String>) -> Result<()> {
        let url = "wss://fstream.binance.com/ws";

        tracing::info!("🔌 Connecting to Binance Futures: {}", url);

        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        tracing::info!("✅ Connected to Binance Futures");

        // Subscribirse a todos los símbolos
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@aggTrade", Self::normalize_symbol(s)))
            .collect();

        tracing::info!("📡 Subscribing to {} streams: {:?}", streams.len(), streams);

        // Enviar mensaje de suscripción
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": 1
        });

        write
            .send(Message::Text(subscribe_msg.to_string()))
            .await?;

        let tx = self.price_tx.clone().unwrap();

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        tracing::debug!("📥 Binance raw message: {}", text);

                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            // Check if it's a subscription response
                            if data.get("result").is_some() || data.get("id").is_some() {
                                tracing::info!("📩 Binance subscription response: {}", text);
                                continue;
                            }

                            // Binance Futures aggTrade format
                            if let (Some(symbol), Some(price_str), Some(timestamp)) = (
                                data["s"].as_str(),
                                data["p"].as_str(),
                                data["T"].as_u64(),
                            ) {
                                if let Ok(price) = price_str.parse::<f64>() {
                                    // Normalizar símbolo: BTCUSDT -> BTC-USDT
                                    let normalized_symbol = symbol.replace("USDT", "-USDT");
                                    tracing::info!("💰 Binance price: {} @ ${}", normalized_symbol, price);

                                    let price_update = Price {
                                        symbol: normalized_symbol,
                                        exchange: Exchange::Binance,
                                        price,
                                        timestamp,
                                    };

                                    let _ = tx.send(price_update).await;
                                }
                            } else {
                                tracing::warn!("⚠️  Binance unexpected message format: {}", text);
                            }
                        } else {
                            tracing::warn!("⚠️  Binance invalid JSON: {}", text);
                        }
                    }
                    Ok(other) => {
                        tracing::debug!("📥 Binance non-text message: {:?}", other);
                    }
                    Err(e) => {
                        tracing::error!("❌ Binance WebSocket error: {:?}", e);
                        break;
                    }
                }
            }

            tracing::warn!("⚠️  Binance connection closed");
        });

        Ok(())
    }

    fn get_price_receiver(&mut self) -> mpsc::Receiver<Price> {
        self.price_rx.take().expect("Receiver already taken")
    }
}
