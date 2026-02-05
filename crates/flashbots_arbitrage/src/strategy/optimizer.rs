pub struct ArbitrageOpportunity {
    pub optimal_amount_weth: f64,
    pub gross_profit_usdc: f64,
    pub net_profit_usdc: f64,
    pub gas_cost_usdc: f64,
    pub buy_pool: &'static str,
    pub sell_pool: &'static str,
}

pub fn calculate_optimal_arbitrage(
    reserves_buy: (u128, u128),
    reserves_sell: (u128, u128),
    b_pool: &'static str,
    s_pool: &'static str,
) -> Option<ArbitrageOpportunity> {
    // Normalize
    let r1_u = reserves_buy.0 as f64 / 1e6;
    let r1_w = reserves_buy.1 as f64 / 1e18;
    let r2_u = reserves_sell.0 as f64 / 1e6;
    let r2_w = reserves_sell.1 as f64 / 1e18;

    const FEE: f64 = 0.997;
    const GAS_COST: f64 = 10.0; // Estimation ATM

    //  USDC -> WETH -> USDC
    let numerator = f64::sqrt(r1_u * r1_w * r2_u * r2_w * FEE.powi(2)) - (r1_u * r2_w);
    let denominator = (FEE * r1_w) + r2_w;

    if numerator <= 0.0 {
        return None;
    }

    let a_optimal_usdc_in = numerator / denominator;

    // Buy on b_pool
    let weth_bought = (a_optimal_usdc_in * FEE * r1_w) / (r1_u + (a_optimal_usdc_in * FEE));

    // Sell on s_pool
    let usdc_out = (weth_bought * FEE * r2_u) / (r2_w + (weth_bought * FEE));

    let gross_profit = usdc_out - a_optimal_usdc_in;
    let net_profit = gross_profit - GAS_COST;

    if net_profit > 1.0 {
        Some(ArbitrageOpportunity {
            optimal_amount_weth: weth_bought,
            gross_profit_usdc: gross_profit,
            net_profit_usdc: net_profit,
            gas_cost_usdc: GAS_COST,
            buy_pool: b_pool,
            sell_pool: s_pool,
        })
    } else {
        None
    }
}
