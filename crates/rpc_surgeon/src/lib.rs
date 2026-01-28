use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

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

pub async fn get_storage_at(rpc_url: &str, address: &str, slot: &str) -> Result<String> {
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

// Find the storage slot for an address in a Solidity mapping
// keccak256(h(k) + p) k -> address, p -> slot position
pub fn derive_mapping_slot(user_address: &str, mapping_slot: u64) -> Result<String> {
    let addr_hex = user_address.trim_start_matches("0x");
    let addr_bytes = hex::decode(addr_hex).context("Failed to decode user address")?;

    let mut buffer = [0u8; 64];

    // 20 bytes address
    buffer[12..32].copy_from_slice(&addr_bytes);
    // 8 bytes slot index
    buffer[56..64].copy_from_slice(&mapping_slot.to_be_bytes());

    let mut hasher = Keccak256::new();
    hasher.update(buffer);
    let result = hasher.finalize();

    Ok(format!("0x{}", hex::encode(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weth_balance_slot() -> Result<()> {
        // Binance Wallet
        let holder = "0xF977814e90dA44bFA03b6295A0616a897441aceC";
        let slot = derive_mapping_slot(holder, 3)?;
        assert_eq!(
            slot,
            "0x9cca97fb08ee88532e0983a3a051466c5df908292b6899f3cdc163eb9c0b22ba"
        );
        Ok(())
    }
}
