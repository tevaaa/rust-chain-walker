use anyhow::{Context, Result};
use clap::Parser;
use event_horizon::{USDC_ADDRESS, run_indexer};
use tokio::time::{Duration, sleep};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = USDC_ADDRESS)]
    target: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let args = Args::parse();
    let wss_url = std::env::var("WSS_URL").context("WSS_URL must be set")?;

    println!("Connecting to {}", wss_url);

    let mut sec = 1;
    loop {
        if let Err(e) = run_indexer(&wss_url, &args.target).await {
            eprintln!("Connection lost: {}. Retrying in {} seconds...", e, sec);
        }
        sleep(Duration::from_secs(sec)).await;
        if sec < 20 { sec += 1 };
    }
}

