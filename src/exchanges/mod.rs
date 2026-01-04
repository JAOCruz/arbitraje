pub mod binance;
pub mod hyperliquid;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub symbol: String,
    pub exchange: Exchange,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Exchange {
    Binance,
    Hyperliquid,
}

impl Exchange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Exchange::Binance => "Binance",
            Exchange::Hyperliquid => "Hyperliquid",
        }
    }
}

#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    fn name(&self) -> Exchange;
    async fn connect(&mut self, symbols: Vec<String>) -> Result<(), anyhow::Error>;
    fn get_price_receiver(&mut self) -> mpsc::Receiver<Price>;
}
