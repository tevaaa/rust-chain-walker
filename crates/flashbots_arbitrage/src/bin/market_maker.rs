use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use flashbots_arbitrage::detector::helper::fetch_reserves;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;

const ANVIL_RPC: &str = "http://localhost:8545";
const WHALE: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"; // Uniswap v3

const UNI_V2_POOL: &str = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";
const SUSHI_V2_POOL: &str = "0x397FF1542f962076d0BFE58eA045FfA2d347ACa0";

const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

async fn unlock_whale() -> Result<()> {
    let req = json!({
      "jsonrpc": "2.0",
      "method": "anvil_impersonateAccount",
      "params": [WHALE],
      "id": 1
    });

    send_transaction(req).await?;

    Ok(())
}

async fn fund_account_with_eth(address: &str) -> Result<()> {
    let req = json!({
        "jsonrpc": "2.0",
        "method": "anvil_setBalance",
        "params": [address, "0x3635C9ADC5DEA00000"], // 1000 ETH
        "id": 1
    });

    send_transaction(req).await?;
    Ok(())
}

fn calculate_usdc_out(weth_in: u128, reserve_usdc: u128, reserve_weth: u128) -> u128 {
    let weth_in_with_fee = weth_in * 997;

    (reserve_usdc * weth_in_with_fee) / (reserve_weth * 1000 + weth_in_with_fee)
}

fn calculate_weth_out(usdc_in: u128, reserve_usdc: u128, reserve_weth: u128) -> u128 {
    let usdc_in_with_fee = usdc_in * 997;
    (reserve_weth * usdc_in_with_fee) / (reserve_usdc * 1000 + usdc_in_with_fee)
}

#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    error: Option<JsonRpcError>,
}

async fn send_transaction(req: serde_json::Value) -> Result<()> {
    let client = reqwest::Client::new();

    let response = client.post(ANVIL_RPC).json(&req).send().await?;

    let text = response
        .text()
        .await
        .context("Failed to get response text")?;

    let parsed: JsonRpcResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Serde error: {} | Raw response: {}", e, text))?;

    if let Some(err) = parsed.error {
        anyhow::bail!("RPC Error: {} (code: {})", err.message, err.code);
    }
    Ok(())
}

fn uint256_to_bytes(value: u128) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[16..32].copy_from_slice(&value.to_be_bytes());
    bytes
}

fn address_to_bytes(addr: &str) -> Result<[u8; 32]> {
    let clean = addr.trim_start_matches("0x");
    let addr_bytes = hex::decode(clean).context("Invalid address")?;

    if addr_bytes.len() != 20 {
        anyhow::bail!("Address must be 20 bytes");
    }

    let mut bytes = [0u8; 32];
    bytes[12..32].copy_from_slice(&addr_bytes);
    Ok(bytes)
}

fn encode_swap_call(to: &str, usdc_out: u128, weth_out: u128) -> Result<Vec<u8>> {
    // Function selector: keccak256("swap(uint256,uint256,address,bytes)")[:4]
    // 0x022c0d9f
    const SWAP_SELECTOR: [u8; 4] = [0x02, 0x2c, 0x0d, 0x9f];

    let mut calldata = Vec::new();

    // Selector (4 bytes)
    calldata.extend_from_slice(&SWAP_SELECTOR);

    if usdc_out > 0 {
        // amount0Out (USDC out) - 32 bytes
        calldata.extend_from_slice(&uint256_to_bytes(usdc_out));
        // amount1Out (WETH out = 0) - 32 bytes
        calldata.extend_from_slice(&uint256_to_bytes(0));
    } else {
        // WETH transfer
        // amount0Out (USDC out = 0) - 32 bytes
        calldata.extend_from_slice(&uint256_to_bytes(0));
        // amount1Out (WETH out) - 32 bytes
        calldata.extend_from_slice(&uint256_to_bytes(weth_out));
    }

    // to (recipient) - 32 bytes
    calldata.extend_from_slice(&address_to_bytes(to)?);

    // data offset (0x80 = 128 bytes) - 32 bytes
    calldata.extend_from_slice(&uint256_to_bytes(0x80));

    // data length (0) - 32 bytes
    calldata.extend_from_slice(&uint256_to_bytes(0));

    Ok(calldata)
}

fn encode_transfer_call(to: &str, amount: u128) -> Result<Vec<u8>> {
    // a9059cbb
    const TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];

    let mut calldata = Vec::new();

    // Selector (4 bytes)
    calldata.extend_from_slice(&TRANSFER_SELECTOR);

    // to
    calldata.extend_from_slice(&address_to_bytes(to)?);

    // amount
    calldata.extend_from_slice(&uint256_to_bytes(amount));

    Ok(calldata)
}

async fn send_swap_bundle(
    pool: &str,
    contract: &str,
    amount_in: u128,
    usdc_out: u128,
    weth_out: u128,
) -> Result<()> {
    let transfer_data = format!("0x{}", hex::encode(encode_transfer_call(pool, amount_in)?));
    let req_transfer = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "eth_sendTransaction",
        "params": [{
            "from": WHALE,
            "to": contract,
            "data": transfer_data
        }]
    });
    send_transaction(req_transfer).await?;

    let swap_data = format!(
        "0x{}",
        hex::encode(encode_swap_call(WHALE, usdc_out, weth_out)?)
    );
    let req_swap = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "eth_sendTransaction",
        "params": [{
            "from": WHALE,
            "to": pool,
            "data": swap_data
        }]
    });
    send_transaction(req_swap).await?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Market Maker Simulator for MEV Testing")]
struct Args {
    #[arg(short, long, value_enum, default_value_t = MarketMode::Real)]
    mode: MarketMode,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MarketMode {
    Real,
    Volatile,
    Extreme,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mode = args.mode;

    let mut rng = rand::rng();

    println!("🎲 Starting Market Maker in {:?} mode 🎲", mode);

    unlock_whale().await?;
    fund_account_with_eth(WHALE).await?;

    loop {
        let is_weth = rng.random_bool(0.5);
        let pool_to_hit = if rng.random_bool(0.7) {
            UNI_V2_POOL
        } else {
            SUSHI_V2_POOL
        };

        let (wait_time, amount) = match mode {
            MarketMode::Real => {
                let s = rng.random_range(2..10);
                let amt = if is_weth {
                    rng.random_range(1e17 as u128..1e18 as u128)
                } else {
                    rng.random_range(200e6 as u128..2000e6 as u128)
                };
                (s, amt)
            }
            MarketMode::Volatile => {
                let s = rng.random_range(10..20);
                let amt = if is_weth {
                    rng.random_range(5e18 as u128..15e18 as u128)
                } else {
                    rng.random_range(10000e6 as u128..30000e6 as u128)
                };
                (s, amt)
            }
            MarketMode::Extreme => {
                let s = rng.random_range(0..2);
                let amt = if is_weth {
                    rng.random_range(1e17 as u128..2e18 as u128)
                } else {
                    rng.random_range(500e6 as u128..5000e6 as u128)
                };
                (s, amt)
            }
        };

        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S%.3f");

        // --- EXECUTION ---
        let display_amount = if is_weth {
            amount as f64 / 1e18
        } else {
            amount as f64 / 1e6
        };
        let token_name = if is_weth { "WETH" } else { "USDC" };
        let pool_name = if pool_to_hit == UNI_V2_POOL {
            "Uniswap"
        } else {
            "Sushiswap"
        };

        let (u, w) = fetch_reserves(ANVIL_RPC, pool_to_hit).await?;

        if is_weth {
            let out = calculate_usdc_out(amount, u, w);
            send_swap_bundle(pool_to_hit, WETH_ADDRESS, amount, out, 0).await?;
        } else {
            let out = calculate_weth_out(amount, u, w);
            send_swap_bundle(pool_to_hit, USDC_ADDRESS, amount, 0, out).await?;
        }

        println!(
            "✨ [{}] {:>5.4} {}  ➡️  {:<10}",
            timestamp, display_amount, token_name, pool_name
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
    }
}
