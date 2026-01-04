mod aggregator;
mod arbitrage;
mod exchanges;

use aggregator::PriceAggregator;
use arbitrage::ArbitrageDetector;
use exchanges::{binance::BinanceConnector, hyperliquid::HyperliquidConnector, ExchangeConnector};
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() {
    // Inicializar logging
    tracing_subscriber::fmt::init();

    tracing::info!("🤖 Starting Arbitrage Bot - MVP (Binance + Hyperliquid)");

    // 1. Símbolos a monitorear
    let symbols = vec![
        "BTC-USDT".to_string(),
        "ETH-USDT".to_string(),
        "SOL-USDT".to_string(),
    ];

    tracing::info!("📊 Monitoring {} symbols: {:?}", symbols.len(), symbols);

    // 2. Crear agregador de precios
    let aggregator = PriceAggregator::new();

    // 3. Conectar a Binance
    let mut binance = BinanceConnector::new();
    binance
        .connect(symbols.clone())
        .await
        .expect("Failed to connect to Binance");

    let mut binance_rx = binance.get_price_receiver();
    let binance_agg = aggregator.clone();

    tokio::spawn(async move {
        while let Some(price) = binance_rx.recv().await {
            tracing::debug!("📈 Binance: {} = ${}", price.symbol, price.price);
            binance_agg.update(price);
        }
    });

    // 4. Conectar a Hyperliquid
    let mut hyperliquid = HyperliquidConnector::new();
    hyperliquid
        .connect(symbols.clone())
        .await
        .expect("Failed to connect to Hyperliquid");

    let mut hyperliquid_rx = hyperliquid.get_price_receiver();
    let hyperliquid_agg = aggregator.clone();

    tokio::spawn(async move {
        while let Some(price) = hyperliquid_rx.recv().await {
            tracing::debug!("📊 Hyperliquid: {} = ${}", price.symbol, price.price);
            hyperliquid_agg.update(price);
        }
    });

    // 5. Crear detector de arbitraje (10 bps = 0.1% ganancia NETA mínima)
    let detector = ArbitrageDetector::new(aggregator.clone(), 10.0);

    // 6. Esperar 5 segundos para que se acumulen precios
    tracing::info!("⏳ Waiting 5 seconds for initial price data...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // 7. Loop principal: Detectar oportunidades cada 2 segundos
    tracing::info!("🔍 Starting arbitrage detection loop...");
    tracing::info!("💡 Note: Showing opportunities with NET profit ≥ 0.1% (after fees)");

    let mut tick = interval(Duration::from_secs(2));
    let mut iteration = 0;

    loop {
        tick.tick().await;
        iteration += 1;

        let opportunities = detector.detect_opportunities();

        if !opportunities.is_empty() {
            println!("\n╔══════════════════════════════════════════════════════════════════╗");
            println!("║ 🚨 {} Oportunidades de Arbitraje (Iteration {})                  ", opportunities.len(), iteration);
            println!("╚══════════════════════════════════════════════════════════════════╝");

            for (i, opp) in opportunities.iter().enumerate() {
                println!(
                    "\n{}) {} | Comprar: {} @ ${:.2} | Vender: {} @ ${:.2}",
                    i + 1,
                    opp.symbol,
                    opp.buy_exchange.as_str(),
                    opp.buy_price,
                    opp.sell_exchange.as_str(),
                    opp.sell_price
                );
                println!(
                    "   📊 Spread Bruto: {:.3}% ({:.0} bps)",
                    opp.spread_pct, opp.spread_bps
                );
                println!(
                    "   💸 Fees: Buy {:.3}% + Sell {:.3}% = {:.3}% total",
                    opp.buy_fee_pct, opp.sell_fee_pct, opp.total_fees_pct
                );
                println!(
                    "   💰 Ganancia NETA: {:.3}% ({:.0} bps) | ${:.2} por $1000",
                    opp.net_profit_pct, opp.net_profit_bps, opp.profit_potential
                );
            }

            println!("\n════════════════════════════════════════════════════════════════════\n");
        } else {
            tracing::info!("✅ Iteration {}: No profitable opportunities (net profit < 0.1%)", iteration);
        }

        // Mostrar estado del agregador cada 5 iteraciones
        if iteration % 5 == 0 {
            println!("\n📊 Price Aggregator Status:");
            for symbol in &symbols {
                let count = aggregator.get_exchange_count(symbol);
                if let Some(prices) = aggregator.get_all_prices(symbol) {
                    println!("  {} ({} exchanges):", symbol, count);
                    for (exchange, price) in prices {
                        println!("    - {}: ${:.2}", exchange.as_str(), price);
                    }
                }
            }
            println!();
        }
    }
}