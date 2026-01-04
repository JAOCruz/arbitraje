use crate::aggregator::PriceAggregator;
use crate::exchanges::Exchange;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub symbol: String,
    pub buy_exchange: Exchange,
    pub buy_price: f64,
    pub sell_exchange: Exchange,
    pub sell_price: f64,
    
    // Spread bruto
    pub spread_bps: f64,
    pub spread_pct: f64,
    
    // Fees
    pub buy_fee_pct: f64,
    pub sell_fee_pct: f64,
    pub total_fees_pct: f64,
    
    // Ganancia neta (después de fees)
    pub net_profit_pct: f64,
    pub net_profit_bps: f64,
    pub profit_potential: f64, // $ ganancia por $1000 invertidos
    
    pub timestamp: u64,
}

#[derive(Clone, Copy)]
pub struct ExchangeFees {
    pub maker: f64,  // En porcentaje (ej: 0.02 = 0.02%)
    pub taker: f64,  // En porcentaje (ej: 0.05 = 0.05%)
}

pub struct FeeConfig {
    fees: HashMap<Exchange, ExchangeFees>,
}

impl FeeConfig {
    pub fn default() -> Self {
        let mut fees = HashMap::new();
        
        // Binance Futures
        fees.insert(Exchange::Binance, ExchangeFees {
            maker: 0.02,
            taker: 0.05,
        });
        
        // Hyperliquid
        fees.insert(Exchange::Hyperliquid, ExchangeFees {
            maker: 0.00,   // Maker rebate en algunos casos
            taker: 0.025,
        });
        
        Self { fees }
    }
    
    pub fn get_taker_fee(&self, exchange: Exchange) -> f64 {
        self.fees.get(&exchange).map(|f| f.taker).unwrap_or(0.05)
    }
    
    pub fn get_maker_fee(&self, exchange: Exchange) -> f64 {
        self.fees.get(&exchange).map(|f| f.maker).unwrap_or(0.02)
    }
}

pub struct ArbitrageDetector {
    aggregator: PriceAggregator,
    min_spread_bps: f64,
    fee_config: FeeConfig,
}

impl ArbitrageDetector {
    pub fn new(aggregator: PriceAggregator, min_spread_bps: f64) -> Self {
        Self {
            aggregator,
            min_spread_bps,
            fee_config: FeeConfig::default(),
        }
    }

    pub fn detect_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let symbols = self.aggregator.get_all_symbols();

        for symbol in symbols {
            // Necesitamos al menos 2 exchanges para arbitraje
            if self.aggregator.get_exchange_count(&symbol) < 2 {
                continue;
            }

            if let Some(prices) = self.aggregator.get_all_prices(&symbol) {
                // Encontrar precio mínimo y máximo
                let (min_exchange, min_price) = prices
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .unwrap();

                let (max_exchange, max_price) = prices
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .unwrap();

                // No arbitraje si es el mismo exchange
                if min_exchange == max_exchange {
                    continue;
                }

                // Calcular spread bruto
                let spread = max_price - min_price;
                let spread_pct = (spread / min_price) * 100.0;
                let spread_bps = spread_pct * 100.0; // 1% = 100 bps

                // Calcular fees (asumiendo taker en ambos lados - worst case)
                let buy_fee_pct = self.fee_config.get_taker_fee(*min_exchange);
                let sell_fee_pct = self.fee_config.get_taker_fee(*max_exchange);
                let total_fees_pct = buy_fee_pct + sell_fee_pct;

                // Ganancia neta = spread bruto - fees
                let net_profit_pct = spread_pct - total_fees_pct;
                let net_profit_bps = net_profit_pct * 100.0;

                // Solo reportar si la ganancia NETA supera el threshold
                if net_profit_bps >= self.min_spread_bps {
                    // Calcular profit potencial por $1000 invertidos
                    let profit_potential = (net_profit_pct / 100.0) * 1000.0;

                    opportunities.push(ArbitrageOpportunity {
                        symbol: symbol.clone(),
                        buy_exchange: *min_exchange,
                        buy_price: *min_price,
                        sell_exchange: *max_exchange,
                        sell_price: *max_price,
                        spread_bps,
                        spread_pct,
                        buy_fee_pct,
                        sell_fee_pct,
                        total_fees_pct,
                        net_profit_pct,
                        net_profit_bps,
                        profit_potential,
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    });
                }
            }
        }

        // Ordenar por ganancia NETA (mayor a menor)
        opportunities.sort_by(|a, b| b.net_profit_bps.partial_cmp(&a.net_profit_bps).unwrap());

        opportunities
    }
}