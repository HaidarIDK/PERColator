//! Constant product AMM math (x·y=k)

use percolator_common::PercolatorError;

/// Scaling factor (1e6)
pub const SCALE: i64 = 1_000_000;

/// Basis points scale (10,000 bps = 100%)
const BPS_SCALE: i64 = 10_000;

/// Quote result with VWAP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuoteResult {
    /// Amount of quote currency (in/out depending on side)
    pub quote_amount: i64,

    /// Volume-weighted average price (scaled by SCALE)
    pub vwap_px: i64,

    /// New x reserve after trade
    pub new_x: i64,

    /// New y reserve after trade
    pub new_y: i64,
}

/// Calculate quote for buying base (Router wants +Δx, provides quote y)
///
/// With fee on input:
/// - x1 = x0 - Δx_out
/// - Invariant: x0·y0 = x1·y1
/// - y1 = (x0·y0) / x1
/// - Δy_gross = y1 - y0
/// - Δy_in = Δy_gross / (1 - fee)
/// - VWAP = Δy_in / Δx_out
pub fn quote_buy(
    x_reserve: i64,
    y_reserve: i64,
    fee_bps: i64,
    dx_out: i64,
    min_liquidity: i64,
) -> Result<QuoteResult, PercolatorError> {
    // Validate inputs
    if x_reserve <= 0 || y_reserve <= 0 {
        return Err(PercolatorError::InvalidAccount);
    }
    if dx_out <= 0 {
        return Err(PercolatorError::InvalidInstruction);
    }
    if dx_out >= x_reserve - min_liquidity {
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // Calculate using i128 to avoid overflow
    let x0 = x_reserve as i128;
    let y0 = y_reserve as i128;
    let dx = dx_out as i128;

    // x1 = x0 - dx
    let x1 = x0 - dx;
    if x1 <= 0 {
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // y1 = (x0 * y0) / x1
    let k = x0 * y0;
    let y1 = k / x1;

    // Δy_gross = y1 - y0
    let dy_gross = y1 - y0;
    if dy_gross <= 0 {
        return Err(PercolatorError::InvalidInstruction);
    }

    // Apply fee: Δy_in = Δy_gross / (1 - fee)
    // = Δy_gross * BPS_SCALE / (BPS_SCALE - fee_bps)
    let fee_multiplier = BPS_SCALE as i128;
    let fee_divisor = fee_multiplier - fee_bps as i128;
    let dy_in = (dy_gross * fee_multiplier) / fee_divisor;

    // VWAP = Δy_in / Δx_out (both scaled, so result is scaled)
    let vwap_px = (dy_in * SCALE as i128) / dx;

    // New reserves: x1, y0 + dy_in
    let new_y = y0 + dy_in;

    Ok(QuoteResult {
        quote_amount: dy_in as i64,
        vwap_px: vwap_px as i64,
        new_x: x1 as i64,
        new_y: new_y as i64,
    })
}

/// Calculate quote for selling base (Router provides Δx, receives quote y)
///
/// With fee on input:
/// - Δx_net = Δx_in * (1 - fee)
/// - x1 = x0 + Δx_net
/// - Invariant: x0·y0 = x1·y1
/// - y1 = (x0·y0) / x1
/// - Δy_out = y0 - y1
/// - VWAP = Δy_out / Δx_in
pub fn quote_sell(
    x_reserve: i64,
    y_reserve: i64,
    fee_bps: i64,
    dx_in: i64,
    min_liquidity: i64,
) -> Result<QuoteResult, PercolatorError> {
    // Validate inputs
    if x_reserve <= 0 || y_reserve <= 0 {
        return Err(PercolatorError::InvalidAccount);
    }
    if dx_in <= 0 {
        return Err(PercolatorError::InvalidInstruction);
    }

    // Calculate using i128 to avoid overflow
    let x0 = x_reserve as i128;
    let y0 = y_reserve as i128;
    let dx = dx_in as i128;

    // Apply fee to input: dx_net = dx * (1 - fee/BPS_SCALE)
    let fee_multiplier = (BPS_SCALE - fee_bps) as i128;
    let dx_net = (dx * fee_multiplier) / BPS_SCALE as i128;

    // x1 = x0 + dx_net
    let x1 = x0 + dx_net;

    // y1 = (x0 * y0) / x1
    let k = x0 * y0;
    let y1 = k / x1;

    // Δy_out = y0 - y1
    let dy_out = y0 - y1;
    if dy_out <= 0 {
        return Err(PercolatorError::InvalidInstruction);
    }
    if dy_out >= (y_reserve - min_liquidity) as i128 {
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // VWAP = Δy_out / Δx_in
    let vwap_px = (dy_out * SCALE as i128) / dx;

    // New reserves: x1, y1
    let new_y = y1;

    Ok(QuoteResult {
        quote_amount: dy_out as i64,
        vwap_px: vwap_px as i64,
        new_x: x1 as i64,
        new_y: new_y as i64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SCALE: i64 = SCALE;

    #[test]
    fn test_quote_buy_small() {
        // x=1000 contracts, y=60M quote units (spot ~60k), fee=5bps
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE; // 60M quote units scaled

        // Buy 1 contract
        let result = quote_buy(x, y, 5, 1 * TEST_SCALE, 1000).unwrap();

        // Should cost slightly more than spot due to slippage + fee
        // Spot = 60M/1000 = 60k per contract
        // Expected VWAP should be close to spot with small slippage
        assert!(result.vwap_px > 60_000 * TEST_SCALE);
        assert!(result.vwap_px < 61_000 * TEST_SCALE); // Should be less than 1.67% slippage+fee

        // Reserves should update correctly
        assert_eq!(result.new_x, x - 1 * TEST_SCALE);
        assert!(result.new_y > y); // Y increases when buying
    }

    #[test]
    fn test_quote_sell_small() {
        // x=1000 contracts, y=60M quote units, fee=5bps
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Sell 1 contract
        let result = quote_sell(x, y, 5, 1 * TEST_SCALE, 1000).unwrap();

        // VWAP should be slightly less than spot due to slippage + fee
        assert!(result.vwap_px < 60_000 * TEST_SCALE);
        assert!(result.vwap_px > 59_000 * TEST_SCALE);

        // Reserves should update correctly
        assert!(result.new_x > x); // X increases when selling
        assert!(result.new_y < y); // Y decreases
    }

    #[test]
    fn test_insufficient_liquidity() {
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Try to buy too much (>= x_reserve)
        let result = quote_buy(x, y, 5, 1000 * TEST_SCALE, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_fee_accounting() {
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Quote with fee
        let with_fee = quote_buy(x, y, 5, 10 * TEST_SCALE, 1000).unwrap();

        // Quote without fee
        let no_fee = quote_buy(x, y, 0, 10 * TEST_SCALE, 1000).unwrap();

        // With fee should cost more
        assert!(with_fee.quote_amount > no_fee.quote_amount);
        assert!(with_fee.vwap_px > no_fee.vwap_px);
    }

    #[test]
    fn test_quote_buy_large() {
        // Test large trade with significant slippage
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Buy 10% of reserves (100 contracts)
        let result = quote_buy(x, y, 5, 100 * TEST_SCALE, 1000).unwrap();

        // Should have significant price impact
        // Spot = 60k, but large trade should cost more
        assert!(result.vwap_px > 65_000 * TEST_SCALE); // At least 8.3% price impact

        // Reserves updated correctly
        assert_eq!(result.new_x, x - 100 * TEST_SCALE);
        assert!(result.new_y > y);
    }

    #[test]
    fn test_quote_sell_large() {
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Sell 10% relative to reserves
        let result = quote_sell(x, y, 5, 100 * TEST_SCALE, 1000).unwrap();

        // Should have significant negative price impact
        assert!(result.vwap_px < 55_000 * TEST_SCALE);

        // Reserves updated correctly
        assert!(result.new_x > x);
        assert!(result.new_y < y);
    }

    #[test]
    fn test_invariant_preservation_buy() {
        // Test that x*y invariant is preserved (accounting for fees)
        let x0 = 1000 * TEST_SCALE;
        let y0 = 60_000_000 * TEST_SCALE;
        let k = (x0 as i128) * (y0 as i128);

        let result = quote_buy(x0, y0, 5, 50 * TEST_SCALE, 1000).unwrap();

        // After buy: x decreases, y increases (by more than quote due to fee)
        // New invariant: x1 * y1 should equal k (before fee is added to y)
        let x1 = result.new_x as i128;
        let y1 = result.new_y as i128;

        // The pool receives more than k requires due to fee
        assert!(x1 * y1 > k, "Invariant should increase due to fees");
    }

    #[test]
    fn test_invariant_preservation_sell() {
        let x0 = 1000 * TEST_SCALE;
        let y0 = 60_000_000 * TEST_SCALE;
        let k = (x0 as i128) * (y0 as i128);

        let result = quote_sell(x0, y0, 5, 50 * TEST_SCALE, 1000).unwrap();

        // After sell: x increases (by less than input due to fee), y decreases
        let x1 = result.new_x as i128;
        let y1 = result.new_y as i128;

        // Pool value stays approximately equal or increases slightly due to fees
        // Allow for small rounding differences
        let k_diff_bps = ((x1 * y1 - k) * 10_000) / k;
        assert!(k_diff_bps >= -10, "Invariant should not decrease significantly (diff: {}bps)", k_diff_bps);
    }

    #[test]
    fn test_round_trip_loses_to_fees() {
        // Buy then sell should lose money due to fees
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;
        let amount = 10 * TEST_SCALE;

        // Buy 10 contracts
        let buy_result = quote_buy(x, y, 5, amount, 1000).unwrap();
        let cost = buy_result.quote_amount;

        // Sell 10 contracts back (using new reserves)
        let sell_result = quote_sell(buy_result.new_x, buy_result.new_y, 5, amount, 1000).unwrap();
        let proceeds = sell_result.quote_amount;

        // Should lose money due to fees and slippage
        assert!(proceeds < cost, "Round-trip should lose to fees (cost={}, proceeds={})", cost, proceeds);

        // Loss should be noticeable (fees + slippage)
        let loss_bps = ((cost - proceeds) as i128 * 10_000) / cost as i128;
        assert!(loss_bps >= 5, "Should lose at least 5bps to fees/slippage (actual: {}bps)", loss_bps);
    }

    #[test]
    fn test_sequential_buys() {
        // Multiple sequential buys should have increasing price impact
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // First buy
        let result1 = quote_buy(x, y, 5, 10 * TEST_SCALE, 1000).unwrap();
        let vwap1 = result1.vwap_px;

        // Second buy (after first)
        let result2 = quote_buy(result1.new_x, result1.new_y, 5, 10 * TEST_SCALE, 1000).unwrap();
        let vwap2 = result2.vwap_px;

        // Second buy should be more expensive
        assert!(vwap2 > vwap1, "Sequential buys should have increasing prices");
    }

    #[test]
    fn test_sequential_sells() {
        // Multiple sequential sells should have decreasing prices
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // First sell
        let result1 = quote_sell(x, y, 5, 10 * TEST_SCALE, 1000).unwrap();
        let vwap1 = result1.vwap_px;

        // Second sell (after first)
        let result2 = quote_sell(result1.new_x, result1.new_y, 5, 10 * TEST_SCALE, 1000).unwrap();
        let vwap2 = result2.vwap_px;

        // Second sell should get worse price
        assert!(vwap2 < vwap1, "Sequential sells should have decreasing prices");
    }

    #[test]
    fn test_zero_amount_error() {
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Zero amount should fail
        assert!(quote_buy(x, y, 5, 0, 1000).is_err());
        assert!(quote_sell(x, y, 5, 0, 1000).is_err());
    }

    #[test]
    fn test_negative_amount_error() {
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        // Negative amount should fail
        assert!(quote_buy(x, y, 5, -10, 1000).is_err());
        assert!(quote_sell(x, y, 5, -10, 1000).is_err());
    }

    #[test]
    fn test_min_liquidity_floor() {
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;
        let min_liq = 1000;

        // Try to buy almost all reserves (leaving less than min_liquidity)
        let result = quote_buy(x, y, 5, x - min_liq + 1, min_liq);
        assert!(result.is_err(), "Should fail when approaching min_liquidity");

        // Should succeed when leaving exactly min_liquidity
        let result = quote_buy(x, y, 5, x - min_liq - 1, min_liq);
        assert!(result.is_ok(), "Should succeed when respecting min_liquidity");
    }

    #[test]
    fn test_high_fee() {
        // Test with very high fee (50% = 5000 bps)
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;

        let high_fee_result = quote_buy(x, y, 5000, 10 * TEST_SCALE, 1000).unwrap();
        let low_fee_result = quote_buy(x, y, 5, 10 * TEST_SCALE, 1000).unwrap();

        // High fee should cost significantly more (at least 50% more)
        let price_ratio = (high_fee_result.vwap_px as i128 * 100) / low_fee_result.vwap_px as i128;
        assert!(price_ratio > 150, "High fee (50%) should cost at least 50% more (actual ratio: {}%)", price_ratio);
    }

    #[test]
    fn test_price_impact_percentage() {
        // Test that price impact scales with trade size
        let x = 1000 * TEST_SCALE;
        let y = 60_000_000 * TEST_SCALE;
        let spot = (y as i128 * TEST_SCALE as i128 / x as i128) as i64;

        // Small trade (1%)
        let small = quote_buy(x, y, 5, 10 * TEST_SCALE, 1000).unwrap();
        let small_impact = ((small.vwap_px - spot) as i128 * 10_000) / spot as i128;

        // Large trade (10%)
        let large = quote_buy(x, y, 5, 100 * TEST_SCALE, 1000).unwrap();
        let large_impact = ((large.vwap_px - spot) as i128 * 10_000) / spot as i128;

        // Larger trade should have much higher price impact
        assert!(large_impact > small_impact * 5);
    }
}
