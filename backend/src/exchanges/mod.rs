// 1. REGISTRAR LOS MÓDULOS (ARCHIVOS)
pub mod binance;
pub mod hyperliquid;
pub mod bybit;
pub mod extended;

use async_trait::async_trait;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

// 2. ACTUALIZAR EL ENUM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    Binance,
    Hyperliquid,
    Bybit,
    Extended,
}

impl Exchange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Exchange::Binance => "Binance",
            Exchange::Hyperliquid => "Hyperliquid",
            Exchange::Bybit => "Bybit",
            Exchange::Extended => "Extended",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BookUpdate {
    pub symbol: String,
    pub exchange: Exchange,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub timestamp: u64,
}

#[async_trait]
pub trait ExchangeConnector {
    fn name(&self) -> Exchange;
    async fn connect(&mut self, symbols: Vec<String>) -> anyhow::Result<()>;
    fn get_receiver(&mut self) -> mpsc::Receiver<BookUpdate>;
}