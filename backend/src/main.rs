// backend/src/main.rs

mod aggregator;
mod arbitrage;
mod exchanges;
mod execution;

use aggregator::{PriceAggregator, MarketBook};
use arbitrage::{ArbitrageDetector, ArbitrageOpportunity};
use exchanges::{binance::BinanceConnector, hyperliquid::HyperliquidConnector, bybit::BybitConnector, extended::ExtendedConnector, ExchangeConnector};
use tokio::sync::broadcast;
use warp::Filter;
use tracing::info;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;

const WS_PORT: u16 = 3030;

// --- NUEVA ESTRUCTURA DE DATOS PARA EL FRONTEND ---
#[derive(Serialize, Clone)]
struct DashboardPayload {
    opportunities: Vec<ArbitrageOpportunity>,
    stats: SimStats,
}

#[derive(Serialize, Clone)]
struct SimStats {
    balance: f64,       // Dinero total (Empieza en 10k)
    total_profit: f64,  // Ganancia acumulada
    trade_count: u32,   // Cantidad de trades simulados
    last_trade: String, // Última acción realizada
}
// --------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("🤖 FLASH-ARB: Iniciando Motor de Paper Trading...");

    // 1. Canal de Broadcast (Ahora envía DashboardPayload en lugar de Vec<Opportunity>)
    let (tx, _rx) = broadcast::channel::<DashboardPayload>(100);
    let tx_clone = tx.clone();

    // 2. Servidor WebSocket
    let routes = warp::path("ws").and(warp::ws()).map(move |ws: warp::ws::Ws| {
        let rx = tx_clone.subscribe();
        ws.on_upgrade(move |socket| handle_socket(socket, rx))
    });

    tokio::spawn(async move {
        info!("🌍 Dashboard Server: ws://127.0.0.1:{}/ws", WS_PORT);
        warp::serve(routes).run(([127, 0, 0, 1], WS_PORT)).await;
    });

    // 3. Inicialización de Exchanges
    let all_symbols = vec![
        "BTC-USDT".to_string(), "ETH-USDT".to_string(), "SOL-USDT".to_string(),
        "DOGE-USDT".to_string(), "PEPE-USDT".to_string(), "WIF-USDT".to_string(),
        "SUI-USDT".to_string(), "APT-USDT".to_string(), "AVAX-USDT".to_string(),
    ];
    let aggregator = PriceAggregator::new();

    // Conectores (Igual que antes)
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

    // 4. ESTADO DE LA SIMULACIÓN
    let mut sim_balance = 10_000.0; // Empezamos con $10,000 USD
    let mut total_profit = 0.0;
    let mut trade_count = 0;
    let mut last_trade_log = "Sistema Iniciado".to_string();

    let detector = ArbitrageDetector::new(aggregator.clone(), 0.0);
    info!("⏳ Calentando mercados (5s)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    info!("🚀 PAPER TRADING ACTIVO. Simulando ejecuciones...");

    loop {
        let opportunities = detector.detect_opportunities();
        
        // --- LÓGICA DE EJECUCIÓN SIMULADA ---
        if !opportunities.is_empty() {
            // Tomamos la mejor oportunidad
            let best_op = &opportunities[0];
            
            // Regla de trading: Solo "ejecutar" si la ganancia es > $0.05 y ROI > 0.01%
            let min_profit_usd = 0.05;
            
            // Simulamos que ejecutamos con $2000 o lo que permita la liquidez
            let trade_amount = f64::min(sim_balance * 0.20, best_op.max_tradeable_usd); // Usamos 20% del balance por trade
            
            if best_op.net_profit_usd > min_profit_usd && trade_amount > 10.0 {
                // Calcular ganancia real proporcional al monto invertido
                // net_profit_usd del detector asume max_tradeable_usd. Ajustamos a trade_amount.
                let real_profit = (trade_amount / best_op.max_tradeable_usd) * best_op.net_profit_usd;
                
                sim_balance += real_profit;
                total_profit += real_profit;
                trade_count += 1;
                last_trade_log = format!("WIN: {} (+${:.4})", best_op.symbol, real_profit);
                
                info!("💰 TRADE #{}: Ganamos ${:.4} en {}", trade_count, real_profit, best_op.symbol);
            }
        }

        // Preparamos el paquete de datos
        let payload = DashboardPayload {
            opportunities: opportunities, // Enviamos todas para verlas
            stats: SimStats {
                balance: sim_balance,
                total_profit,
                trade_count,
                last_trade: last_trade_log.clone(),
            }
        };

        // Enviar al Frontend
        let _ = tx.send(payload);
        
        // Pausa de 1 segundo para no hacer spam de trades simulados
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }
}

async fn handle_socket(ws: warp::ws::WebSocket, mut rx: broadcast::Receiver<DashboardPayload>) {
    let (mut sender, _) = ws.split();
    while let Ok(payload) = rx.recv().await {
        let json = serde_json::to_string(&payload).unwrap_or_default();
        if sender.send(warp::ws::Message::text(json)).await.is_err() { break; }
    }
}

impl From<exchanges::BookUpdate> for MarketBook {
    fn from(u: exchanges::BookUpdate) -> Self {
        MarketBook { bid: u.bid, ask: u.ask, bid_size: u.bid_size, ask_size: u.ask_size, timestamp: u.timestamp }
    }
}