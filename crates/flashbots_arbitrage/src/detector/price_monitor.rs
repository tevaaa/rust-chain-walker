use crate::detector::parser;
use anyhow::{Context, Result};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio::select;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const UNI_V2_POOL: &str = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";
const SUSHI_V2_POOL: &str = "0x397FF1542f962076d0BFE58eA045FfA2d347ACa0";
const SWAP_EVENT: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";

pub struct PriceMonitor {
    uni_reserves: (u128, u128),
    sushi_reserves: (u128, u128),
}

impl PriceMonitor {
    pub fn new() -> Self {
        Self {
            uni_reserves: (0, 0),
            sushi_reserves: (0, 0),
        }
    }

    pub async fn handle_event(&mut self, source: &str, msg: Message) -> Result<()> {
        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S%.3f").to_string();

        if let Message::Text(text) = msg {
            // Desrialise
            println!("{} Received {} Method: {}", timestamp, source, v["method"]);
            let v: serde_json::Value =
                serde_json::from_str(&text).context("Failed to parse JSON-RPC message")?;

            // Parser
            if v["method"] == "eth_subscription" {
                if let Some(data_hex) = v["params"]["result"]["data"].as_str() {
                    let amounts = parser::parse_swap_log(data_hex);
                    self.update_price(source, amounts).await;
                }
            } else if v["result"].is_string() {
                println!("Subscription confirmed for {}", source);
            }
        }
        Ok(())
    }

    pub async fn update_price(&mut self, source: &str, amounts: parser::SwapAmounts) {
        let in_0 = amounts.amount0_in as f64 / 1_000_000.0;
        let in_1 = amounts.amount1_in as f64 / 1_000_000_000_000_000_000.0;
        let out_0 = amounts.amount0_out as f64 / 1_000_000.0;
        let out_1 = amounts.amount1_out as f64 / 1_000_000_000_000_000_000.0; // WETH a 18 décimales

        if out_0 > 0.0 {
            println!(
                "💰 [{}] Swap | Sold: {:.2} USDC | Bought: {:.4} WETH",
                source, out_0, in_1
            );
        } else if out_1 > 0.0 {
            println!(
                "💎 [{}] SWAP | Sold: {:.4} WETH | Bought: {:.2} USDC",
                source, out_1, in_0
            );
        }
    }

    pub async fn listen(&mut self, wss_url: &str) -> Result<()> {
        // WebSocket setup
        let (uni_ws, _) = connect_async(wss_url).await.context("Uni WS failed")?;
        let (sushi_ws, _) = connect_async(wss_url).await.context("Sushi WS failed")?;

        let (mut uni_write, mut uni_read) = uni_ws.split();
        let (mut sushi_write, mut sushi_read) = sushi_ws.split();

        // Subscribe to contracts
        let sub_uni = json!({
                "jsonrpc": "2.0", "id": 1, "method": "eth_subscribe",
                "params": ["logs", {"address": UNI_V2_POOL, "topics": [SWAP_EVENT]}]
        });
        uni_write.send(Message::Text(sub_uni.to_string())).await?;

        let sub_sushi = json!({
                "jsonrpc": "2.0", "id": 2, "method": "eth_subscribe",
                "params": ["logs", {"address": SUSHI_V2_POOL, "topics": [SWAP_EVENT]}]
        });
        sushi_write
            .send(Message::Text(sub_sushi.to_string()))
            .await?;

        loop {
            tokio::select! {
                msg = uni_read.next() => {
                    if let Some(Ok(m)) = msg {
                        self.handle_event("uniswap", m).await?;
                    }
                }
                msg = sushi_read.next() => {
                    if let Some(Ok(m)) = msg {
                        self.handle_event("sushiswap", m).await?;
                    }
                }
            }
        }
    }
}
