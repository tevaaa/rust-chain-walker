use anyhow::{Context, Result};
use event_horizon::process_raw_message;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let wss_url = std::env::var("WSS_URL").context("WSS_URL must be set")?;

    println!("Connecting to {}", wss_url);

    let (ws_stream, _) = connect_async(wss_url)
        .await
        .context("Failed to connect to WebSocket")?;

    println!("Handshake successful!");
    let (mut write, mut read) = ws_stream.split();
    let usdc_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["logs", {"address": usdc_address}]
    });

    write.send(Message::Text(subscribe_msg.to_string())).await?;
    println!("Subscription request sent...");

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Some(transfer) = process_raw_message(&text) {
                    let amount = transfer.amount as f64 / 1_000_000.0;
                    println!(
                        "💸 {} -> {} | {:.2} USDC",
                        transfer.from, transfer.to, amount
                    );
                }
            }
            Ok(_) => (),
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        }
    }
    Ok(())
}
