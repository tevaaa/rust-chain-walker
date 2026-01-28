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
    pub amount: u128,
}

pub fn process_raw_message(text: &str) -> Option<TransferEvent> {
    let notification: LogNotification = serde_json::from_str(text).ok()?;

    let result = notification.params.result;

    if result.topics.len() < 3 {
        return None;
    }
    let from = format!("0x{}", &result.topics[1][26..]);
    let to = format!("0x{}", &result.topics[2][26..]);

    let amount = decode_hex_to_u128(&result.data);
    Some(TransferEvent { from, to, amount })
}
