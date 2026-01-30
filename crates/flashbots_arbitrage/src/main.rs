use anyhow::Result;
use flashbots_arbitrage::detector::price_monitor::PriceMonitor;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    let wss_url = std::env::var("WSS_URL").expect("❌ WSS_URL not found in .env file");

    println!("🔍 Starting Price Monitor...");
    println!("📡 WebSocket: {}", wss_url);
    println!("👀 Watching Uniswap V2 & Sushiswap...\n");

    let mut monitor = PriceMonitor::new();

    monitor.listen(&wss_url).await?;

    Ok(())
}
