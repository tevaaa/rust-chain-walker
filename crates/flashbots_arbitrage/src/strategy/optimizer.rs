pub struct ArbitrageOpportunity {
    pub optimal_amount_weth: f64,
    pub gross_profit_usdc: f64,
    pub net_profit_usdc: f64,
    pub gas_cost_usdc: f64,
    pub buy_pool: &'static str,
    pub sell_pool: &'static str,
}

/// Simulates a single arbitrage execution
fn simulate_arbitrage(usdc_in: f64, r1_usdc: f64, r1_weth: f64, r2_usdc: f64, r2_weth: f64) -> f64 {
    const FEE: f64 = 0.997;

    // Step 1: Buy WETH with USDC on cheap pool
    let weth_bought = (usdc_in * FEE * r1_weth) / (r1_usdc + (usdc_in * FEE));

    // Step 2: Sell WETH for USDC on expensive pool
    let usdc_received = (weth_bought * FEE * r2_usdc) / (r2_weth + (weth_bought * FEE));

    // Return net profit (before gas)
    usdc_received - usdc_in
}

/// Finds optimal arbitrage amount using binary search
pub fn calculate_optimal_arbitrage(
    reserves_buy: (u128, u128),
    reserves_sell: (u128, u128),
    b_pool: &'static str,
    s_pool: &'static str,
) -> Option<ArbitrageOpportunity> {
    // Normalize to human units
    let r1_usdc = reserves_buy.0 as f64 / 1e6;
    let r1_weth = reserves_buy.1 as f64 / 1e18;
    let r2_usdc = reserves_sell.0 as f64 / 1e6;
    let r2_weth = reserves_sell.1 as f64 / 1e18;

    const GAS_COST: f64 = 15.0;

    // Calculate prices
    let p_buy = r1_usdc / r1_weth;
    let p_sell = r2_usdc / r2_weth;

    // Sanity check: must have price difference
    if p_buy >= p_sell {
        return None;
    }

    // Binary search bounds
    // Start with very small amount, max 10% of smaller pool's USDC
    let min_pool_usdc = r1_usdc.min(r2_usdc);
    let mut low = 10.0; // Start at $10
    let mut high = min_pool_usdc * 0.1; // Max 10% of pool

    let mut best_amount = 0.0;
    let mut best_profit = 0.0;

    // Binary search for maximum profit
    for _ in 0..100 {
        // 100 iterations = very precise
        let mid1 = low + (high - low) / 3.0;
        let mid2 = high - (high - low) / 3.0;

        let profit1 = simulate_arbitrage(mid1, r1_usdc, r1_weth, r2_usdc, r2_weth);
        let profit2 = simulate_arbitrage(mid2, r1_usdc, r1_weth, r2_usdc, r2_weth);

        if profit1 > best_profit {
            best_profit = profit1;
            best_amount = mid1;
        }
        if profit2 > best_profit {
            best_profit = profit2;
            best_amount = mid2;
        }

        // Ternary search logic
        if profit1 < profit2 {
            low = mid1;
        } else {
            high = mid2;
        }
    }

    // Calculate final amounts with best input
    const FEE: f64 = 0.997;
    let weth_bought = (best_amount * FEE * r1_weth) / (r1_usdc + (best_amount * FEE));
    let usdc_received = (weth_bought * FEE * r2_usdc) / (r2_weth + (weth_bought * FEE));

    let gross_profit = usdc_received - best_amount;
    let net_profit = gross_profit - GAS_COST;

    // Debug output
    if gross_profit > 0.0 {
        eprintln!("\n=== ARBITRAGE ANALYSIS ===");
        eprintln!("Buy: {} @ ${:.2}", b_pool, p_buy);
        eprintln!("Sell: {} @ ${:.2}", s_pool, p_sell);
        eprintln!(
            "Spread: ${:.2} ({:.2}%)",
            p_sell - p_buy,
            (p_sell - p_buy) / p_buy * 100.0
        );
        eprintln!("Optimal USDC: ${:.2}", best_amount);
        eprintln!("WETH traded: {:.4}", weth_bought);
        eprintln!("Gross profit: ${:.2}", gross_profit);
        eprintln!("Gas cost: ${:.2}", GAS_COST);
        eprintln!("Net profit: ${:.2}", net_profit);
        eprintln!("=========================\n");
    }

    // Only return if profitable after gas
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
