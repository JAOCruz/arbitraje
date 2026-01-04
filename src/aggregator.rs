use crate::exchanges::{Exchange, Price};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct PriceAggregator {
    // symbol -> (exchange -> price)
    prices: Arc<DashMap<String, DashMap<Exchange, f64>>>,
    timestamps: Arc<DashMap<String, DashMap<Exchange, u64>>>,
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            prices: Arc::new(DashMap::new()),
            timestamps: Arc::new(DashMap::new()),
        }
    }

    pub fn update(&self, price: Price) {
        // Actualizar precio
        self.prices
            .entry(price.symbol.clone())
            .or_insert_with(DashMap::new)
            .insert(price.exchange, price.price);

        // Actualizar timestamp
        self.timestamps
            .entry(price.symbol)
            .or_insert_with(DashMap::new)
            .insert(price.exchange, price.timestamp);
    }

    pub fn get_all_prices(&self, symbol: &str) -> Option<Vec<(Exchange, f64)>> {
        self.prices.get(symbol).map(|map| {
            map.iter()
                .map(|entry| (*entry.key(), *entry.value()))
                .collect()
        })
    }

    pub fn get_all_symbols(&self) -> Vec<String> {
        self.prices.iter().map(|entry| entry.key().clone()).collect()
    }

    pub fn get_exchange_count(&self, symbol: &str) -> usize {
        self.prices
            .get(symbol)
            .map(|map| map.len())
            .unwrap_or(0)
    }
}
