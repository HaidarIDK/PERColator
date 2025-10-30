//! Fixed-point math utilities

/// Fixed-point precision (6 decimals)
pub const PRICE_DECIMALS: u32 = 6;
pub const PRICE_MULTIPLIER: u64 = 1_000_000;

/// Multiply two u64 values and return u128
#[inline]
pub fn mul_u64(a: u64, b: u64) -> u128 {
    (a as u128) * (b as u128)
}

/// Multiply u64 by u128
#[inline]
pub fn mul_u64_u128(a: u64, b: u128) -> u128 {
    (a as u128) * b
}

/// Divide u128 by u64, rounding up
#[inline]
pub fn div_ceil_u128(numerator: u128, denominator: u64) -> u128 {
    let denom = denominator as u128;
    (numerator + denom - 1) / denom
}

/// Divide u128 by u64, rounding down
#[inline]
pub fn div_floor_u128(numerator: u128, denominator: u64) -> u128 {
    numerator / (denominator as u128)
}

/// Calculate VWAP: (total_notional / total_qty)
#[inline]
pub fn calculate_vwap(total_notional: u128, total_qty: u64) -> u64 {
    if total_qty == 0 {
        return 0;
    }
    (total_notional / (total_qty as u128)) as u64
}

/// Update VWAP with new fill
/// Returns new (total_qty, total_notional)
#[inline]
pub fn update_vwap(
    current_qty: u64,
    current_notional: u128,
    fill_qty: u64,
    fill_price: u64,
) -> (u64, u128) {
    let new_qty = current_qty + fill_qty;
    let new_notional = current_notional + mul_u64(fill_qty, fill_price);
    (new_qty, new_notional)
}

/// Calculate position PnL
/// PnL = qty * (current_price - entry_price)
#[inline]
pub fn calculate_pnl(qty: i64, entry_price: u64, current_price: u64) -> i128 {
    let qty_i128 = qty as i128;
    let entry_i128 = entry_price as i128;
    let current_i128 = current_price as i128;
    qty_i128 * (current_i128 - entry_i128)
}

/// Calculate funding payment
/// Payment = qty * (cum_funding_current - cum_funding_entry)
#[inline]
pub fn calculate_funding_payment(qty: i64, cum_funding_current: i128, cum_funding_entry: i128) -> i128 {
    let qty_i128 = qty as i128;
    qty_i128 * (cum_funding_current - cum_funding_entry)
}

/// Check if price is within tick alignment
#[inline]
pub fn is_tick_aligned(price: u64, tick: u64) -> bool {
    price % tick == 0
}

/// Check if quantity is within lot alignment
#[inline]
pub fn is_lot_aligned(qty: u64, lot: u64) -> bool {
    qty % lot == 0
}

/// Round price to tick
#[inline]
pub fn round_to_tick(price: u64, tick: u64) -> u64 {
    (price / tick) * tick
}

/// Round quantity to lot
#[inline]
pub fn round_to_lot(qty: u64, lot: u64) -> u64 {
    (qty / lot) * lot
}

/// Calculate IM requirement: |qty| * contract_size * mark_price * imr
#[inline]
pub fn calculate_im(qty: i64, contract_size: u64, mark_price: u64, imr_bps: u64) -> u128 {
    let abs_qty = qty.abs() as u64;
    let notional = mul_u64(abs_qty, contract_size);
    let notional_value = mul_u64_u128(mark_price, notional);
    // imr_bps is in basis points (1 bp = 0.01%)
    (notional_value * (imr_bps as u128)) / 10_000
}

/// Calculate MM requirement: |qty| * contract_size * mark_price * mmr
#[inline]
pub fn calculate_mm(qty: i64, contract_size: u64, mark_price: u64, mmr_bps: u64) -> u128 {
    let abs_qty = qty.abs() as u64;
    let notional = mul_u64(abs_qty, contract_size);
    let notional_value = mul_u64_u128(mark_price, notional);
    // mmr_bps is in basis points (1 bp = 0.01%)
    (notional_value * (mmr_bps as u128)) / 10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_calculation() {
        let (qty, notional) = update_vwap(0, 0, 100, 50_000);
        assert_eq!(qty, 100);
        assert_eq!(notional, 5_000_000);
        assert_eq!(calculate_vwap(notional, qty), 50_000);

        let (qty, notional) = update_vwap(qty, notional, 50, 51_000);
        assert_eq!(qty, 150);
        let vwap = calculate_vwap(notional, qty);
        // VWAP should be (100*50000 + 50*51000) / 150 = 50333.33...
        assert!(vwap >= 50_333 && vwap <= 50_334);
    }

    #[test]
    fn test_pnl_calculation() {
        // Long position profit
        let pnl = calculate_pnl(10, 50_000, 51_000);
        assert_eq!(pnl, 10_000);

        // Long position loss
        let pnl = calculate_pnl(10, 50_000, 49_000);
        assert_eq!(pnl, -10_000);

        // Short position profit
        let pnl = calculate_pnl(-10, 50_000, 49_000);
        assert_eq!(pnl, 10_000);

        // Short position loss
        let pnl = calculate_pnl(-10, 50_000, 51_000);
        assert_eq!(pnl, -10_000);
    }
}
