pub struct SwapAmounts {
    pub amount0_in: u128,
    pub amount1_in: u128,
    pub amount0_out: u128,
    pub amount1_out: u128,
}

pub fn parse_swap_log(data: &str) -> SwapAmounts {
    let data = data.trim_start_matches("0x");

    SwapAmounts {
        amount0_in: u128::from_str_radix(&data[0..64], 16).unwrap_or(0),
        amount1_in: u128::from_str_radix(&data[64..128], 16).unwrap_or(0),
        amount0_out: u128::from_str_radix(&data[128..192], 16).unwrap_or(0),
        amount1_out: u128::from_str_radix(&data[192..256], 16).unwrap_or(0),
    }
}
