// backend/src/main.rs

mod aggregator;
mod arbitrage;
mod exchanges;
mod execution;

use aggregator::{PriceAggregator, MarketBook};
use arbitrage::{ArbitrageDetector, ArbitrageOpportunity};
use exchanges::Exchange;
use exchanges::{binance::BinanceConnector, hyperliquid::HyperliquidConnector, bybit::BybitConnector, extended::ExtendedConnector, ExchangeConnector};
use tokio::sync::broadcast;
use warp::Filter;
use tracing::info;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};

const WS_PORT: u16 = 3030;

#[derive(Serialize, Clone)]
struct SimStats {
    total_usd: f64,
    binance_usd: f64,
    bybit_usd: f64,
    hyperliquid_usd: f64,
    extended_usd: f64,
    trade_count: u32,
    last_action: String,
}

#[derive(Serialize, Clone)]
struct DashboardPayload {
    opportunities: Vec<ArbitrageOpportunity>,
    stats: SimStats,
    recent_trades: Vec<TradeLog>,    
}

#[derive(Serialize, Clone)] // Se requiere Clone para el historial en memoria
struct TradeLog {
    timestamp: String,
    symbol: String,
    buy_exchange: String,
    sell_exchange: String,
    buy_price: f64,
    sell_price: f64,
    profit_usd: f64,
    balance_after: f64,
    note: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("🤖 FLASH-ARB: Motor v2.0 Online (Multi-Balance + VWAP)");

    init_csv();

    let (tx, _rx) = broadcast::channel::<DashboardPayload>(100);
    let tx_clone = tx.clone();

    let routes = warp::path("ws").and(warp::ws()).map(move |ws: warp::ws::Ws| {
        let rx = tx_clone.subscribe();
        ws.on_upgrade(move |socket| handle_socket(socket, rx))
    });

    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], WS_PORT)).await;
    });

    // --- BALANCES INICIALES ---
    let mut b_bal = 5000.0;
    let mut by_bal = 5000.0;
    let mut hl_bal = 5000.0;
    let mut ex_bal = 5000.0;
    let mut sim_balance = 20000.0;
    let mut trade_count = 0;
    let mut last_trade_log = "Sistema Iniciado".to_string();
    
    // Lista para el historial en el Dashboard
    let mut recent_trades_list = load_history_from_csv();

    let all_symbols = vec![
        // Hyperliquid & Ecosystem Leaders
        "HYPE-USDT".into(), "PURR-USDT".into(), "POL-USDT".into(), "SOL-USDT".into(),
        // High-Volatility Memes (Lo mejor para Tokio)
        "PEPE-USDT".into(), "WIF-USDT".into(), "BONK-USDT".into(), 
        "POPCAT-USDT".into(), "FLOKI-USDT".into(), "DOGE-USDT".into(),
        // AI & Infrastructure (Movimientos bruscos)
        "FET-USDT".into(), "RENDER-USDT".into(), "TAO-USDT".into(),
        "NEAR-USDT".into(), "LDO-USDT".into(), "ENA-USDT".into(),
        // Fast L1s/L2s
        "SUI-USDT".into(), "APT-USDT".into(), "AVAX-USDT".into(),
        "SEI-USDT".into(), "TIA-USDT".into(), "ARB-USDT".into(),
        "OP-USDT".into(), "STRK-USDT".into(), "ETH-USDT".into(),
        // High-Cap Alts
        "LINK-USDT".into(), "PYTH-USDT".into(), "JUP-USDT".into(),
        "INJ-USDT".into(), "STX-USDT".into(), "ORDI-USDT".into(),
        "BTC-USDT".into(),
    ];
    
    let aggregator = PriceAggregator::new();

    // Conectores
    let mut binance = BinanceConnector::new();
    if let Ok(_) = binance.connect(all_symbols.clone()).await {
        let mut rx = binance.get_receiver(); let agg = aggregator.clone();
        tokio::spawn(async move { while let Some(u) = rx.recv().await { agg.update(u.symbol.clone(), u.exchange, MarketBook::from(u)); }});
    }
    let mut hl = HyperliquidConnector::new();
    if let Ok(_) = hl.connect(all_symbols.clone()).await {
        let mut rx = hl.get_receiver(); let agg = aggregator.clone();
        tokio::spawn(async move { while let Some(u) = rx.recv().await { agg.update(u.symbol.clone(), u.exchange, MarketBook::from(u)); }});
    }
    let mut bybit = BybitConnector::new();
    if let Ok(_) = bybit.connect(all_symbols.clone()).await {
        let mut rx = bybit.get_receiver(); let agg = aggregator.clone();
        tokio::spawn(async move { while let Some(u) = rx.recv().await { agg.update(u.symbol.clone(), u.exchange, MarketBook::from(u)); }});
    }
    let mut extended = ExtendedConnector::new();
    if let Ok(_) = extended.connect(all_symbols.clone()).await {
        let mut rx = extended.get_receiver(); let agg = aggregator.clone();
        tokio::spawn(async move { while let Some(u) = rx.recv().await { agg.update(u.symbol.clone(), u.exchange, MarketBook::from(u)); }});
    }

    let detector = ArbitrageDetector::new(aggregator.clone(), 0.0);
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    loop {
        let mut opportunities = detector.detect_opportunities();
        
        let slippage_bps = 0.5; 
        let slippage_factor = slippage_bps / 10000.0; 
        
        // Simulación de fricción para todas las oportunidades en el feed
        for op in opportunities.iter_mut() {
            let cap_sim = f64::min(2000.0, op.max_tradeable_usd);
            let liq_impact_sim = (cap_sim / op.max_tradeable_usd) * 0.0003;
            let total_fric_sim = slippage_factor + liq_impact_sim;
            
            // Usamos data_age_ms para pasar temporalmente este valor al Dashboard 
            // o simplemente deja que el backend lo use para filtrar visualmente.
            op.total_fees_pct = (total_fric_sim * 100.0) + 0.06; // Fricción + Fee estimado
        }

        if !opportunities.is_empty() {
            let best_op = &opportunities[0];
            let base_fee_pct = 0.0006;
        
            let trade_capital = f64::min(2000.0, best_op.max_tradeable_usd);
            let liquidity_impact = (trade_capital / best_op.max_tradeable_usd) * 0.0003;
        
            let total_friction = slippage_factor + liquidity_impact;
            let final_buy_price = best_op.buy_price * (1.0 + total_friction);
            let final_sell_price = best_op.sell_price * (1.0 - total_friction);
        
            if trade_capital > 10.0 {
                let has_funds = match best_op.buy_exchange {
                    Exchange::Binance => b_bal >= trade_capital,
                    Exchange::Bybit => by_bal >= trade_capital,
                    Exchange::Hyperliquid => hl_bal >= trade_capital,
                    Exchange::Extended => ex_bal >= trade_capital,
                };

                if has_funds {
                    let trade_qty = trade_capital / final_buy_price;
                    let cost_real = (trade_qty * final_buy_price) * (1.0 + base_fee_pct);
                    let revenue_real = (trade_qty * final_sell_price) * (1.0 - base_fee_pct);
                    let profit_net_real = revenue_real - cost_real;

                    if profit_net_real > 0.0001 {
                        // Gestión de Balances
                        match best_op.buy_exchange {
                            Exchange::Binance => b_bal -= trade_capital,
                            Exchange::Bybit => by_bal -= trade_capital,
                            Exchange::Hyperliquid => hl_bal -= trade_capital,
                            Exchange::Extended => ex_bal -= trade_capital,
                        }

                        match best_op.sell_exchange {
                            Exchange::Binance => b_bal += trade_capital + profit_net_real,
                            Exchange::Bybit => by_bal += trade_capital + profit_net_real,
                            Exchange::Hyperliquid => hl_bal += trade_capital + profit_net_real,
                            Exchange::Extended => ex_bal += trade_capital + profit_net_real,
                        }

                        sim_balance = b_bal + by_bal + hl_bal + ex_bal;
                        trade_count += 1;
                        last_trade_log = format!("WIN: {} (+${:.4})", best_op.symbol, profit_net_real);
                        
                        let new_trade_entry = TradeLog {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            symbol: best_op.symbol.clone(),
                            buy_exchange: format!("{:?}", best_op.buy_exchange),
                            sell_exchange: format!("{:?}", best_op.sell_exchange),
                            buy_price: final_buy_price,
                            sell_price: final_sell_price,
                            profit_usd: profit_net_real,
                            balance_after: sim_balance,
                            note: format!("Tokio Sim (Fric: {:.2}bps)", total_friction * 10000.0),
                        };

                        // Actualizar historial para el Frontend
                        recent_trades_list.insert(0, new_trade_entry.clone());
                        recent_trades_list.truncate(10);

                        info!("💰 TRADE #{}: +${:.4} en {} (Fricción: {:.4}%)", 
                            trade_count, profit_net_real, best_op.symbol, total_friction * 100.0);
                            
                        log_trade_to_csv(new_trade_entry);
                    }
                }
            }
        }

        // --- CONSTRUIR Y ENVIAR PAYLOAD ---
        let payload = DashboardPayload {
            opportunities, 
            stats: SimStats {
                total_usd: sim_balance,
                binance_usd: b_bal,
                bybit_usd: by_bal,
                hyperliquid_usd: hl_bal,
                extended_usd: ex_bal,
                trade_count,
                last_action: last_trade_log.clone(),
            },
            recent_trades: recent_trades_list.clone(),
        };

        let _ = tx.send(payload);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

async fn handle_socket(ws: warp::ws::WebSocket, mut rx: broadcast::Receiver<DashboardPayload>) {
    let (mut sender, _) = ws.split();
    while let Ok(payload) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&payload) {
            if sender.send(warp::ws::Message::text(json)).await.is_err() { break; }
        }
    }
}

fn init_csv() {
    let path = "trades_log.csv";
    if std::fs::metadata(path).is_err() {
        let mut wtr = csv::Writer::from_path(path).unwrap();
        wtr.write_record(&["Timestamp", "Symbol", "BuyEx", "SellEx", "BuyPrice", "SellPrice", "Profit", "Balance", "Note"]).unwrap();
        wtr.flush().unwrap();
    }
}

fn load_history_from_csv() -> Vec<TradeLog> {
    let path = "trades_log.csv";
    let mut history = Vec::new();
    
    if let Ok(file) = std::fs::File::open(path) {
        let reader = BufReader::new(file);
        // Saltamos la cabecera y leemos las líneas
        for line in reader.lines().skip(1) {
            if let Ok(l) = line {
                let v: Vec<&str> = l.split(',').collect();
                if v.len() >= 8 {
                    history.push(TradeLog {
                        timestamp: v[0].to_string(),
                        symbol: v[1].to_string(),
                        buy_exchange: v[2].to_string(),
                        sell_exchange: v[3].to_string(),
                        buy_price: v[4].parse().unwrap_or(0.0),
                        sell_price: v[5].parse().unwrap_or(0.0),
                        profit_usd: v[6].parse().unwrap_or(0.0),
                        balance_after: v[7].parse().unwrap_or(0.0),
                        note: v.get(8).unwrap_or(&"").to_string(),
                    });
                }
            }
        }
    }
    // Invertimos para que los más nuevos estén arriba y limitamos a 10
    history.reverse();
    history.truncate(10);
    history
}

fn log_trade_to_csv(trade: TradeLog) {
    let file = OpenOptions::new().write(true).append(true).open("trades_log.csv").unwrap();
    let mut wtr = csv::WriterBuilder::new().has_headers(false).from_writer(file);
    wtr.serialize(trade).unwrap();
    wtr.flush().unwrap();
}

impl From<exchanges::BookUpdate> for MarketBook {
    fn from(u: exchanges::BookUpdate) -> Self {
        MarketBook { bid: u.bid, ask: u.ask, bid_size: u.bid_size, ask_size: u.ask_size, timestamp: u.timestamp }
    }
}