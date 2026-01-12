// src/execution.rs

use crate::exchanges::Exchange;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy)]
pub enum Side {
    Buy,
    Sell,
}

#[async_trait]
pub trait Executor {
    async fn place_order(
        &self, 
        symbol: &str, 
        side: Side, 
        amount: f64, 
        price: Option<f64>
    ) -> anyhow::Result<String>;
    
    async fn get_balance(&self, asset: &str) -> anyhow::Result<f64>;
}

// Estructura vacía por ahora para que compile si decidimos usarla luego
pub struct MockExecutor;

#[async_trait]
impl Executor for MockExecutor {
    async fn place_order(&self, _symbol: &str, _side: Side, _amount: f64, _price: Option<f64>) -> anyhow::Result<String> {
        Ok("mock_order_id".to_string())
    }
    
    async fn get_balance(&self, _asset: &str) -> anyhow::Result<f64> {
        Ok(1000.0)
    }
}