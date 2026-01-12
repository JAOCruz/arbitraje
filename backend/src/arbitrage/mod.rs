// src/arbitrage/mod.rs

pub mod detector;

// Re-exportamos para facilitar el uso en main.rs
pub use detector::{ArbitrageDetector, ArbitrageOpportunity};