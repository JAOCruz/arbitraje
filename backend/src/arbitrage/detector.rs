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
    
    // Datos Financieros
    pub spread_pct: f64,
    pub total_fees_pct: f64,      // <--- CAMPO RE-AGREGADO
    pub net_profit_pct: f64,
    pub net_profit_usd: f64,
    
    // Liquidez
    pub max_tradeable_qty: f64,   // <--- CAMPO RE-AGREGADO (Cantidad de monedas)
    pub max_tradeable_usd: f64,   // (Cantidad de dólares)
    pub liquidity_bottleneck: Exchange,
    
    pub data_age_ms: u64,
    pub timestamp: u64,

    pub created_at: u64, // Timestamp en milisegundos
}

#[derive(Clone, Copy)]
pub struct ExchangeFees {
    pub maker: f64,
    pub taker: f64,
}

pub struct FeeConfig {
    fees: HashMap<Exchange, ExchangeFees>,
}

impl FeeConfig {
    pub fn default() -> Self {
        let mut fees = HashMap::new();
        // Fees conservadores (Taker)
        fees.insert(Exchange::Binance, ExchangeFees { maker: 0.02, taker: 0.05 }); 
        fees.insert(Exchange::Hyperliquid, ExchangeFees { maker: 0.00, taker: 0.025 });
        fees.insert(Exchange::Bybit, ExchangeFees { maker: 0.02, taker: 0.06 });
        fees.insert(Exchange::Extended, ExchangeFees { maker: 0.05, taker: 0.05 }); 
        Self { fees }
    }
    
    pub fn get_taker_fee(&self, exchange: Exchange) -> f64 {
        self.fees.get(&exchange).map(|f| f.taker).unwrap_or(0.06)
    }
}

pub struct ArbitrageDetector {
    aggregator: PriceAggregator,
    // Eliminamos el campo min_net_profit_bps de la struct ya que usamos logica interna
    fee_config: FeeConfig,
}

impl ArbitrageDetector {
    pub fn new(aggregator: PriceAggregator, _unused_threshold: f64) -> Self {
        Self {
            aggregator,
            fee_config: FeeConfig::default(),
        }
    }

    pub fn detect_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let symbols = self.aggregator.get_all_symbols();
        let now = chrono::Utc::now().timestamp_millis() as u64;
        
        let max_age_ms = 5000; // Permitimos hasta 2s de latencia
        let min_usd_profit = 0.001; // Bajamos un poco la vara para ver si funciona todo bien ($0.02)

        for symbol in symbols {
            if let Some(books) = self.aggregator.get_books(&symbol) {
                for (exchange_buy, book_buy) in &books {
                    for (exchange_sell, book_sell) in &books {
                        if exchange_buy == exchange_sell { continue; }

                        // 1. Latencia
                        let age_buy = now.saturating_sub(book_buy.timestamp);
                        let age_sell = now.saturating_sub(book_sell.timestamp);
                        let max_age = std::cmp::max(age_buy, age_sell);
                        if max_age > max_age_ms { continue; }

                        // 2. Precios
                        let buy_price = book_buy.ask;
                        let sell_price = book_sell.bid;

                        if sell_price > buy_price {
                            // 3. Liquidez Real (Bottleneck)
                            let max_qty_buy = book_buy.ask_size;
                            let max_qty_sell = book_sell.bid_size;
                            let tradeable_qty = f64::min(max_qty_buy, max_qty_sell);
                            let tradeable_usd = tradeable_qty * buy_price;

                            if tradeable_usd < 10.0 { continue; } 

                            // 4. Fees y Ganancia USD
                            let fee_buy_pct = self.fee_config.get_taker_fee(*exchange_buy);
                            let fee_sell_pct = self.fee_config.get_taker_fee(*exchange_sell);
                            
                            let fee_buy = fee_buy_pct / 100.0;
                            let fee_sell = fee_sell_pct / 100.0;
                            
                            let cost = tradeable_usd * (1.0 + fee_buy);
                            let revenue = (tradeable_qty * sell_price) * (1.0 - fee_sell);
                            
                            let net_profit_usd = revenue - cost;
                            let net_profit_pct = ((revenue - cost) / cost) * 100.0;
                            let total_fees_pct = fee_buy_pct + fee_sell_pct;

                            if net_profit_usd > min_usd_profit {
                                opportunities.push(ArbitrageOpportunity {
                                    symbol: symbol.clone(),
                                    buy_exchange: *exchange_buy,
                                    buy_price,
                                    sell_exchange: *exchange_sell,
                                    sell_price,
                                    spread_pct: ((sell_price - buy_price)/buy_price)*100.0,
                                    total_fees_pct,
                                    net_profit_pct,
                                    net_profit_usd,
                                    max_tradeable_qty: tradeable_qty, // RELLENAMOS EL CAMPO
                                    max_tradeable_usd: tradeable_usd,
                                    liquidity_bottleneck: if max_qty_buy < max_qty_sell { *exchange_buy } else { *exchange_sell },
                                    data_age_ms: max_age,
                                    timestamp: now,
                                    created_at: now,
                                });
                            }
                        }
                    }
                }
            }
        }
        opportunities.sort_by(|a, b| b.net_profit_usd.partial_cmp(&a.net_profit_usd).unwrap());
        opportunities
    }
}