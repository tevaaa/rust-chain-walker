use super::helper;
use anyhow::{Context, Result};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const UNI_V2_POOL: &str = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";
const SUSHI_V2_POOL: &str = "0x397FF1542f962076d0BFE58eA045FfA2d347ACa0";
const SYNC_EVENT: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

#[derive(Default)]
pub struct PriceMonitor {
    uni_reserves: (u128, u128),
    sushi_reserves: (u128, u128),
    sync_count: u64,
}

impl PriceMonitor {
    pub fn new() -> Self {
        Self {
            uni_reserves: (0, 0),
            sushi_reserves: (0, 0),
            sync_count: 0,
        }
    }
    /// Initialize monitor by fetching current reserves
    pub async fn initialize(&mut self, http_rpc: &str) -> Result<()> {
        println!("🔄 Fetching initial reserves...\n");

        let (uni_usdc, uni_weth) = helper::fetch_reserves(http_rpc, UNI_V2_POOL)
            .await
            .context("Failed to fetch Uniswap reserves")?;

        let (sushi_usdc, sushi_weth) = helper::fetch_reserves(http_rpc, SUSHI_V2_POOL)
            .await
            .context("Failed to fetch Sushiswap reserves")?;

        // Calculate initial prices
        let price_uni = Self::calculate_price((uni_usdc, uni_weth));
        let price_sushi = Self::calculate_price((sushi_usdc, sushi_weth));

        // Print formatted output
        println!("✅ Uniswap");
        println!(
            "   Reserves: {} USDC / {} WETH",
            uni_usdc / 1_000_000,
            uni_weth / 1_000_000_000_000_000_000
        );
        println!("   Price: {:.2} USDC/WETH\n", price_uni);

        println!("✅ Sushiswap");
        println!(
            "   Reserves: {} USDC / {} WETH",
            sushi_usdc / 1_000_000,
            sushi_weth / 1_000_000_000_000_000_000
        );
        println!("   Price: {:.2} USDC/WETH", price_sushi);

        // Calculate initial spread
        let spread_abs = (price_uni - price_sushi).abs();
        let spread_bps = (spread_abs / ((price_uni + price_sushi) / 2.0)) * 10_000.0;

        println!(
            "📊 Initial spread: {:.2} USDC ({:.2} bps)\n",
            spread_abs, spread_bps
        );

        self.uni_reserves = (uni_usdc, uni_weth);
        self.sushi_reserves = (sushi_usdc, sushi_weth);
        Ok(())
    }

    fn calculate_price(reserves: (u128, u128)) -> f64 {
        // Adjust decimals: USDC (6) vs WETH (18) = 12 decimals difference
        reserves.0 as f64 / reserves.1 as f64 * 1e12
    }

    /// Handle incoming WebSocket message
    async fn handle_event(&mut self, source: &str, msg: Message) -> Result<()> {
        if let Message::Text(text) = msg {
            let v: serde_json::Value =
                serde_json::from_str(&text).context("Failed to parse JSON-RPC message")?;

            // Skip subscription confirmations
            if v.get("id").is_some() {
                println!("✅ WSS subscription confirmed for {}\n", source);
                return Ok(());
            }

            // Parse Sync event
            if v.get("method") == Some(&serde_json::json!("eth_subscription"))
                && let Some(data_hex) = v["params"]["result"]["data"].as_str()
            {
                let sync = helper::parse_sync_event(data_hex);
                self.update_reserves(source, sync);
            }
        }
        Ok(())
    }

    // Update reserves from Sync event (source of truth)
    fn update_reserves(&mut self, source: &str, sync: helper::SyncReserves) {
        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S%.3f");

        // Get mutable reference to correct pool
        let reserves = match source {
            "uniswap" => &mut self.uni_reserves,
            "sushiswap" => &mut self.sushi_reserves,
            _ => {
                eprintln!("⚠️  Unknown source: {}", source);
                return;
            }
        };

        *reserves = (sync.reserve0, sync.reserve1);

        self.sync_count += 1;

        let price = Self::calculate_price(*reserves);

        println!("{} 💱 [{}] Sync #{}", timestamp, source, self.sync_count);
        println!(
            "   Reserves: {} USDC / {} WETH",
            reserves.0 / 1_000_000,
            reserves.1 / 1_000_000_000_000_000_000
        );
        println!("   Price: {:.2} USDC/WETH", price);

        // Check for arbitrage opportunity
        self.check_arbitrage();
    }

    // Check if arbitrage opportunity exists
    fn check_arbitrage(&self) {
        const THRESHOLD_BPS: f64 = 50.0; // 0.5% minimum spread

        let price_uni = Self::calculate_price(self.uni_reserves);
        let price_sushi = Self::calculate_price(self.sushi_reserves);

        let spread_abs = (price_uni - price_sushi).abs();
        let price_avg = (price_uni + price_sushi) / 2.0;
        let spread_bps = (spread_abs / price_avg) * 10_000.0;
        println!(
            "New spread: ABS: {:.2} | BPS: {:.1}\n",
            spread_abs, spread_bps
        );

        if spread_bps > THRESHOLD_BPS {
            let (buy_dex, buy_price, sell_dex, sell_price) = if price_uni < price_sushi {
                ("Uniswap", price_uni, "Sushiswap", price_sushi)
            } else {
                ("Sushiswap", price_sushi, "Uniswap", price_uni)
            };

            println!("\n🚨 ARBITRAGE DETECTED!");
            println!("   Buy:  {} @ {:.2} USDC/WETH", buy_dex, buy_price);
            println!("   Sell: {} @ {:.2} USDC/WETH", sell_dex, sell_price);
            println!("   Spread: {:.2} USDC ({:.1} bps)", spread_abs, spread_bps);
            println!("   Theoretical profit (1 WETH): ${:.2}\n", spread_abs);
        }
    }

    /// Start listening to WebSocket events
    pub async fn listen(&mut self, wss_url: &str, http_rpc: &str) -> Result<()> {
        Self::initialize(self, http_rpc).await?;

        // Connect to WebSockets
        let (uni_ws, _) = connect_async(wss_url)
            .await
            .context("Failed to connect Uniswap WebSocket")?;
        let (sushi_ws, _) = connect_async(wss_url)
            .await
            .context("Failed to connect Sushiswap WebSocket")?;

        let (mut uni_write, mut uni_read) = uni_ws.split();
        let (mut sushi_write, mut sushi_read) = sushi_ws.split();

        // Subscribe to Sync events
        let sub_uni = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_subscribe",
            "params": ["logs", {
                "address": UNI_V2_POOL,
                "topics": [SYNC_EVENT]
            }]
        });

        let sub_sushi = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "eth_subscribe",
            "params": ["logs", {
                "address": SUSHI_V2_POOL,
                "topics": [SYNC_EVENT]
            }]
        });

        uni_write.send(Message::Text(sub_uni.to_string())).await?;
        sushi_write
            .send(Message::Text(sub_sushi.to_string()))
            .await?;

        println!("👂 Listening for Sync events...\n");

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
