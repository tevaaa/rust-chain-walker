use crate::detector::parser;
use anyhow::{Context, Result};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio::select;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const UNI_V3_POOL: &str = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640";
const SUSHI_V3_POOL: &str = "0xf3Eb87C1F6020982173C908E7eB31aA66c1f0296";
const SWAP_EVENT: &str = "0xc42021b1a9404827050d212870743a014596206b006a619c961917f69906646b";

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
        println!("{} Received {}", timestamp, source);

        let raw_text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
            Message::Ping(_) => {
                println!("PING");
                // println!("{} [PING] from {}", timestamp, source); // Trop de bruit
                return Ok(());
            }
            Message::Pong(_) => return Ok(()),
            _ => return Ok(()),
        };
        // if let Message::Text(text) = msg {
        // Desrialise
        let v: serde_json::Value =
            serde_json::from_str(&raw_text).context("Failed to parse JSON-RPC message")?;
        println!("{}", v);

        // Parser
        if v["method"] == "eth_subscription" {
            if let Some(data_hex) = v["params"]["result"]["data"].as_str() {
                let amounts = parser::parse_swap_log(data_hex);
                self.update_price(source, amounts).await;
            }
        } else if v["result"].is_string() {
            println!("Subscription confirmed for {}", source);
        }
        //}
        Ok(())
    }

    pub async fn update_price(&mut self, source: &str, amounts: parser::SwapAmounts) {
        println!("You are here");
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
                "params": ["logs", {"address": UNI_V3_POOL, "topics": [SWAP_EVENT]}]
        });
        uni_write.send(Message::Text(sub_uni.to_string())).await?;

        let sub_sushi = json!({
                "jsonrpc": "2.0", "id": 2, "method": "eth_subscribe",
                "params": ["logs", {"address": SUSHI_V3_POOL, "topics": [SWAP_EVENT]}]
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
