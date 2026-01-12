use crate::exchanges::Exchange;
use dashmap::DashMap;
use std::sync::Arc;

// 1. Definimos la estructura del Libro (Bid y Ask)
#[derive(Debug, Clone, Copy)]
pub struct MarketBook {
    pub bid: f64,      // El precio más alto al que alguien quiere COMPRAR (tú vendes aquí)
    pub ask: f64,      // El precio más bajo al que alguien quiere VENDER (tú compras aquí)
    pub bid_size: f64,
    pub ask_size: f64,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct PriceAggregator {
    // 2. Ahora guardamos Libros enteros, no solo precios sueltos
    // Map: Symbol -> (Exchange -> MarketBook)
    books: Arc<DashMap<String, DashMap<Exchange, MarketBook>>>,
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            // Inicializamos el mapa de libros
            books: Arc::new(DashMap::new()),
        }
    }

    // Actualizamos con Bid y Ask
    pub fn update(&self, symbol: String, exchange: Exchange, book: MarketBook) {
        self.books
            .entry(symbol)
            .or_insert_with(DashMap::new)
            .insert(exchange, book);
    }

    // Obtener todos los libros de un símbolo para compararlos
    pub fn get_books(&self, symbol: &str) -> Option<Vec<(Exchange, MarketBook)>> {
        self.books.get(symbol).map(|map| {
            map.iter()
                .map(|entry| (*entry.key(), *entry.value()))
                .collect()
        })
    }

    pub fn get_all_symbols(&self) -> Vec<String> {
        self.books.iter().map(|entry| entry.key().clone()).collect()
    }
    
    pub fn get_exchange_count(&self, symbol: &str) -> usize {
        self.books
            .get(symbol)
            .map(|map| map.len())
            .unwrap_or(0)
    }
}