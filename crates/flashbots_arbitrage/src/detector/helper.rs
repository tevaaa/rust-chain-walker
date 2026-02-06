use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;

pub struct SyncReserves {
    pub reserve0: u128,
    pub reserve1: u128,
}

pub fn parse_sync_event(data: &str) -> SyncReserves {
    let data = data.trim_start_matches("0x");

    SyncReserves {
        reserve0: u128::from_str_radix(&data[0..64], 16).unwrap_or(0),
        reserve1: u128::from_str_radix(&data[64..128], 16).unwrap_or(0),
    }
}

#[derive(Deserialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    result: Option<String>,
    error: Option<JsonRpcError>,
}

pub async fn fetch_reserves(rpc_url: &str, pool_address: &str) -> Result<(u128, u128)> {
    const GET_RESERVE: &str = "0x0902f1ac";
    let client = reqwest::Client::new();

    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [
            { "to": pool_address, "data": GET_RESERVE },
            "latest"
        ]
    });

    let response = client.post(rpc_url).json(&req).send().await?;

    let text = response
        .text()
        .await
        .context("Failed to get response text")?;

    let parsed: JsonRpcResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Serde error: {} | Raw response: {}", e, text))?;

    if let Some(err) = parsed.error {
        anyhow::bail!("RPC Error: {} (code: {})", err.message, err.code);
    }

    let result_str = parsed
        .result
        .context("No result and no error in RPC response")?;
    let hex = result_str.trim_start_matches("0x");

    if hex.len() < 128 {
        anyhow::bail!("Invalid reserves data length: {}", hex.len());
    }

    let r0 = u128::from_str_radix(&hex[0..64], 16).context("Failed to parse reserve0")?;
    let r1 = u128::from_str_radix(&hex[64..128], 16).context("Failed to parse reserve1")?;

    Ok((r0, r1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_uni_reserves() -> Result<()> {
        dotenvy::dotenv().ok();

        let http_rpc = std::env::var("WSS_URL")
            .expect("WSS_URL")
            .replace("wss://", "https://")
            .replace("ws://", "http://");

        let uni_pool = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc";

        let (r0, r1) = fetch_reserves(&http_rpc, uni_pool).await?;

        assert!(r0 > 0);
        // r1 -> USDC
        assert!(r1 > r0);

        Ok(())
    }
}
