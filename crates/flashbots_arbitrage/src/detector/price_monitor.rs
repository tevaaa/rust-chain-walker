use crate::detector::parser;
use anyhow::{Context, Result};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio::select;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const UNI_V2_POOL: &str = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";
const SUSHI_V2_POOL: &str = "0x397FFBe3f3752eA848cF7f2861f944E390273234";
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
        if let Message::Text(text) = msg {
            // Desrialise
            let v: serde_json::Value =
                serde_json::from_str(&text).context("Failed to parse JSON-RPC message")?;

            // Parser
            if let Some(data_hex) = v["params"]["result"]["data"].as_str() {
                let amounts = parser::parse_swap_log(data_hex);
                self.update_price(source, amounts).await;
            }
        }
        Ok(())
    }

    pub async fn update_price(&mut self, source: &str, amounts: parser::SwapAmounts) {
        println!("Update received from {}", source);
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
                "jsonrpc": "2.0", "id": 1, "method": "eth_subscribe",
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
