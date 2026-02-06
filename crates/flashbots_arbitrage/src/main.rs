use anyhow::Result;
use flashbots_arbitrage::detector::price_monitor::PriceMonitor;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    let wss_rpc = std::env::var("WSS_URL").expect("❌ WSS_URL not found in .env file");
    let http_rpc = std::env::var("HTTP_URL").expect("❌ HTTP_URL not found in .env file");

    println!("=============================");
    println!("|🔍 Starting Price Monitor...|");
    println!("=============================\n");
    println!("👀 Watching Uniswap V2 & Sushiswap...");

    let mut monitor = PriceMonitor::new();

    monitor.listen(&wss_rpc, &http_rpc).await?;

    Ok(())
}
