use super::{BookUpdate, Exchange, ExchangeConnector};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest, tungstenite::Message};
use http::HeaderValue;

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
        // La documentación muestra ejemplos como "BTC-USD".
        // Si tu bot usa "BTC-USDT", lo convertimos.
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

            // CONSTRUCCIÓN DE LA URL BASADA EN LA DOCS
            // Formato: wss://api.starknet.extended.exchange/stream.extended.exchange/v1/orderbooks/{market}?depth=1
            let url_str = format!(
                "wss://api.starknet.extended.exchange/stream.extended.exchange/v1/orderbooks/{}?depth=1",
                market
            );

            tokio::spawn(async move {
                // 1. Crear request con User-Agent (Para evitar el WAF 403/404)
                let mut request = match url_str.into_client_request() {
                    Ok(req) => req,
                    Err(e) => {
                        tracing::error!("❌ Error URL Extended: {:?}", e);
                        return;
                    }
                };

                let headers = request.headers_mut();
                headers.insert("User-Agent", HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));
                headers.insert("Origin", HeaderValue::from_static("https://app.extended.exchange"));

                tracing::info!("🔌 Connecting to Extended Stream ({})", safe_symbol);

                match connect_async(request).await {
                    Ok((ws_stream, _)) => {
                        tracing::info!("✅ Connected to Extended for {}", safe_symbol);
                        let (_, mut read) = ws_stream.split();

                        // En este modelo NO enviamos mensaje de suscripción.
                        // La conexión a la URL ES la suscripción.

                        while let Some(msg) = read.next().await {
                            if let Ok(Message::Text(text)) = msg {
                                // Parseo según la documentación proveída:
                                // { "data": { "b": [{"p": "...", "q": "..."}], "a": [...] }, "ts": ... }
                                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                    
                                    // Validamos que tenga data
                                    if let Some(data) = json.get("data") {
                                        // Extraemos timestamp del nivel raíz o usamos actual
                                        let ts = json.get("ts").and_then(|t| t.as_u64()).unwrap_or(0);

                                        let bids = data.get("b"); // Array de bids
                                        let asks = data.get("a"); // Array de asks

                                        // Helper para sacar (precio, cantidad) del primer elemento del array
                                        let get_top = |list: Option<&Value>| -> Option<(f64, f64)> {
                                            let items = list?.as_array()?;
                                            let top = items.get(0)?; // Mejor precio (depth=1)
                                            let p = top.get("p")?.as_str()?.parse::<f64>().ok()?;
                                            let q = top.get("q")?.as_str()?.parse::<f64>().ok()?;
                                            Some((p, q))
                                        };

                                        if let (Some((bid, bid_sz)), Some((ask, ask_sz))) = (get_top(bids), get_top(asks)) {
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
                        tracing::warn!("⚠️ Extended stream ended for {}", safe_symbol);
                    }
                    Err(e) => {
                        tracing::error!("❌ Extended Connection Failed ({}) - Check URL/Symbol: {:?}", safe_symbol, e);
                    }
                }
                // Pausa antes de reconectar si falla
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            });
            
            // Scaled connection start to avoid rate limits
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        Ok(())
    }

    fn get_receiver(&mut self) -> mpsc::Receiver<BookUpdate> {
        self.rx.take().expect("Receiver already taken")
    }
}