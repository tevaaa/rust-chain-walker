use anyhow::{Context, Result};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

fn decode_hex_to_u128(raw_hex: &str) -> u128 {
    let clean = raw_hex.trim_start_matches("0x");
    u128::from_str_radix(clean, 16).unwrap_or(0)
}

#[derive(serde::Deserialize)]
struct LogNotification {
    params: LogParams,
}

#[derive(serde::Deserialize)]
struct LogParams {
    result: LogResult,
}

#[derive(serde::Deserialize)]
struct LogResult {
    data: String,
    topics: Vec<String>,
}

pub struct TransferEvent {
    pub from: String,
    pub to: String,
    pub amount_raw: u128,
}

impl TransferEvent {
    pub fn amount_formatted(&self, decimals: u32) -> f64 {
        self.amount_raw as f64 / 10f64.powi(decimals as i32)
    }
}

fn process_raw_message(text: &str) -> Option<TransferEvent> {
    let notification: LogNotification = serde_json::from_str(text).ok()?;

    let result = notification.params.result;

    if result.topics.len() < 3 {
        return None;
    }
    let from = format!("0x{}", &result.topics[1][26..]);
    let to = format!("0x{}", &result.topics[2][26..]);

    let amount_raw = decode_hex_to_u128(&result.data);
    Some(TransferEvent {
        from,
        to,
        amount_raw,
    })
}

pub async fn run_indexer(wss_url: &str, target: &str) -> Result<()> {
    let (ws_stream, _) = connect_async(wss_url)
        .await
        .context("Failed to connect to WebSocket")?;
    println!("Handshake successful!");
    let (mut write, mut read) = ws_stream.split();

    // Find decimals
    let decimals_req = json!({
        "jsonrpc": "2.0", "id": 2, "method": "eth_call",
        "params": [{"to": target, "data": "0x313ce567"}, "latest"]
    });
    write.send(Message::Text(decimals_req.to_string())).await?;
    let decimals = if let Some(Ok(Message::Text(res))) = read.next().await {
        let v: serde_json::Value = serde_json::from_str(&res)?;
        let hex = v["result"].as_str().unwrap_or("0x12");
        decode_hex_to_u128(hex) as u32
    } else {
        18
    };

    // Subscribing
    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["logs", {"address": target}]
    });
    println!("Subscription request sent...");
    write.send(Message::Text(subscribe_msg.to_string())).await?;
    println!("< Monitoring started >");

    while let Some(message) = read.next().await {
        let message = message.context("Network error")?;
        if let Message::Text(text) = message
            && let Some(transfer) = process_raw_message(&text)
        {
            let precision = if decimals <= 6 { 2 } else { 8 };
            let amount = transfer.amount_formatted(decimals);
            println!(
                "💸 {} -> {} | {:.*} 🪙",
                transfer.from, transfer.to, precision, amount
            );
        }
    }
    Err(anyhow::anyhow!("Stream closed"))
}

pub struct TokenMetadata {
    pub symbol: String,
    pub decimals: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transfer_event() -> Result<()> {
        let mock_json = r#"{
            "params": {
                "result": {
                    "data": "0x0000000000000000000000000000000000000000000000000000000005f5e100",
                    "topics": [
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                        "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                        "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec"
                    ]
                }
            }
        }"#;

        let event = process_raw_message(mock_json).context("JSON parsing failed")?;
        assert_eq!(event.amount_raw, 100_000_000); // 100 USDC
        Ok(())
    }

    #[test]
    fn test_decode_hex_to_u128_valid() {
        assert_eq!(decode_hex_to_u128("0x64"), 100);
        assert_eq!(decode_hex_to_u128("0xFF"), 255);
        assert_eq!(decode_hex_to_u128("0x0"), 0);
        assert_eq!(decode_hex_to_u128("0x"), 0);
    }

    #[test]
    fn test_parse_transfer_event_insufficient_topics() {
        let mock_json = r#"{
            "params": {
                "result": {
                    "data": "0x0000000000000000000000000000000000000000000000000000000005f5e100",
                    "topics": [
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
                    ]
                }
            }
        }"#;

        let event = process_raw_message(mock_json);
        assert!(event.is_none(), "Should return None when topics.len() < 3");
    }

    #[test]
    fn test_parse_transfer_event_empty_topics() {
        let mock_json = r#"{
            "params": {
                "result": {
                    "data": "0x0000000000000000000000000000000000000000000000000000000005f5e100",
                    "topics": []
                }
            }
        }"#;

        let event = process_raw_message(mock_json);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_transfer_event_malformed_json() {
        let malformed = r#"{ "params": { "result": { "data": invalid } } }"#;
        let event = process_raw_message(malformed);
        assert!(event.is_none());
    }

    #[test]
    fn test_parse_transfer_event_missing_fields() {
        let incomplete_json = r#"{
            "params": {
                "result": {
                    "topics": [
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                        "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                        "0x000000000000000000000000f977814e90da44bfa03b6295a0616a897441acec"
                    ]
                }
            }
        }"#;

        let event = process_raw_message(incomplete_json);
        assert!(
            event.is_none(),
            "Should return None when 'data' field is missing"
        );
    }
    #[test]
    fn test_transfer_event_addresses_lowercase() {
        let mock_json = r#"{
            "params": {
                "result": {
                    "data": "0x0000000000000000000000000000000000000000000000000000000005f5e100",
                    "topics": [
                        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                        "0x000000000000000000000000aabbccddaabbccddaabbccddaabbccddaabbccdd",
                        "0x000000000000000000000000eeff00aaeeff00aaeeff00aaeeff00aaeeff00aa"
                    ]
                }
            }
        }"#;

        let event = process_raw_message(mock_json).expect("Should parse");
        assert_eq!(
            event.from.to_lowercase(),
            "0xaabbccddaabbccddaabbccddaabbccddaabbccdd"
        );
        assert_eq!(
            event.to.to_lowercase(),
            "0xeeff00aaeeff00aaeeff00aaeeff00aaeeff00aa"
        );
    }
}
