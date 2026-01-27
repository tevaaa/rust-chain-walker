use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha3::{Keccak256, Digest};
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u32,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    result: String,
}

async fn get_storage_at(rpc_url: &str, address: &str, slot: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let payload = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "eth_getStorageAt".to_string(),
        params: serde_json::json!([address, slot, "latest"]),
        id: 1,
    };

    let response = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to send RPC request")?;

    let parsed: JsonRpcResponse = response
        .json()
        .await
        .context("Failed to parse RPC response")?;

    Ok(parsed.result)
}

fn derive_mapping_slot(user_address: &str, mapping_slot: u64) -> Result<String> {
    let addr_hex = user_address.trim_start_matches("0x");
    let addr_bytes = hex::decode(addr_hex).context("Failed to decode user address")?;

    let mut buffer = [0u8; 64];

    buffer[12..32].copy_from_slice(&addr_bytes);
    buffer[56..64].copy_from_slice(&mapping_slot.to_be_bytes());

    let mut hasher = Keccak256::new();
    hasher.update(buffer);
    let result = hasher.finalize();

    Ok(format!("0x{}", hex::encode(result)))

    
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let rpc_url = std::env::var("RPC_URL").context("RPC_URL must be defined")?;

    let eth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    let binance_holder = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
    let mapping_slot = 3;

    let target_slot = derive_mapping_slot(binance_holder, mapping_slot)?;

    println!("--- SURGERY RESULT ---");
    println!("Contract: {}", eth_address);
    println!("Taret Slot for Balance: {}", target_slot);

    let raw_balance = get_storage_at(&rpc_url, eth_address, &target_slot).await?;
    println!("Raw Balance: {}", raw_balance);

    Ok(())
}
