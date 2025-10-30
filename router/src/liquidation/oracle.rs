//! Oracle alignment validation for liquidations

use pinocchio::msg;

/// Validate if slab mark price is aligned with oracle price
///
/// Returns true if the difference between slab mark and oracle price
/// is within the tolerance threshold defined in the registry.
///
/// # Arguments
/// * `slab_mark` - Mark price from the slab (1e6 scale)
/// * `oracle_price` - Oracle price (1e6 scale)
/// * `tolerance_bps` - Tolerance in basis points (e.g., 50 = 0.5%)
///
/// # Returns
/// * `true` if aligned (within tolerance)
/// * `false` if misaligned (exceeds tolerance)
pub fn validate_oracle_alignment(
    slab_mark: i64,
    oracle_price: i64,
    tolerance_bps: u64,
) -> bool {
    if oracle_price == 0 {
        msg!("Oracle: Zero oracle price, rejecting alignment");
        return false;
    }

    let diff = (slab_mark - oracle_price).abs();
    let threshold = ((oracle_price.abs() as u128 * tolerance_bps as u128) / 10_000) as i64;

    let aligned = diff <= threshold;

    if !aligned {
        msg!("Oracle: Slab mark price misaligned with oracle");
    }

    aligned
}

/// Calculate price band for liquidation orders
///
/// Returns (lower_bound, upper_bound) for limit prices based on
/// oracle price and band width in basis points.
///
/// For selling (reducing long): use lower_bound as limit price
/// For buying (reducing short): use upper_bound as limit price
///
/// # Arguments
/// * `oracle_price` - Oracle price (1e6 scale)
/// * `band_bps` - Price band width in basis points (e.g., 200 = 2%)
///
/// # Returns
/// * `(lower_bound, upper_bound)` - Price band limits
pub fn calculate_price_band(oracle_price: i64, band_bps: u64) -> (i64, i64) {
    if oracle_price == 0 {
        return (0, 0);
    }

    // Calculate band as percentage of oracle price
    let band_amount = ((oracle_price.abs() as u128 * band_bps as u128) / 10_000) as i64;

    if oracle_price > 0 {
        let lower = oracle_price.saturating_sub(band_amount);
        let upper = oracle_price.saturating_add(band_amount);
        (lower, upper)
    } else {
        // Handle negative prices (shouldn't happen in practice)
        let lower = oracle_price.saturating_add(band_amount);
        let upper = oracle_price.saturating_sub(band_amount);
        (lower, upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_alignment_within_tolerance() {
        let oracle_price = 1_000_000; // $1.00
        let slab_mark = 1_004_000;    // $1.004 (0.4% diff)
        let tolerance_bps = 50;        // 0.5%

        assert!(validate_oracle_alignment(slab_mark, oracle_price, tolerance_bps));
    }

    #[test]
    fn test_oracle_alignment_exceeds_tolerance() {
        let oracle_price = 1_000_000; // $1.00
        let slab_mark = 1_006_000;    // $1.006 (0.6% diff)
        let tolerance_bps = 50;        // 0.5%

        assert!(!validate_oracle_alignment(slab_mark, oracle_price, tolerance_bps));
    }

    #[test]
    fn test_oracle_alignment_exact_threshold() {
        let oracle_price = 1_000_000; // $1.00
        let slab_mark = 1_005_000;    // $1.005 (0.5% diff exactly)
        let tolerance_bps = 50;        // 0.5%

        assert!(validate_oracle_alignment(slab_mark, oracle_price, tolerance_bps));
    }

    #[test]
    fn test_oracle_alignment_negative_diff() {
        let oracle_price = 1_000_000; // $1.00
        let slab_mark = 996_000;      // $0.996 (0.4% diff)
        let tolerance_bps = 50;        // 0.5%

        assert!(validate_oracle_alignment(slab_mark, oracle_price, tolerance_bps));
    }

    #[test]
    fn test_oracle_alignment_zero_oracle() {
        let oracle_price = 0;
        let slab_mark = 1_000_000;
        let tolerance_bps = 50;

        assert!(!validate_oracle_alignment(slab_mark, oracle_price, tolerance_bps));
    }

    #[test]
    fn test_price_band_calculation() {
        let oracle_price = 1_000_000; // $1.00
        let band_bps = 200;            // 2%

        let (lower, upper) = calculate_price_band(oracle_price, band_bps);

        assert_eq!(lower, 980_000);  // $0.98
        assert_eq!(upper, 1_020_000); // $1.02
    }

    #[test]
    fn test_price_band_tight() {
        let oracle_price = 1_000_000; // $1.00
        let band_bps = 100;            // 1%

        let (lower, upper) = calculate_price_band(oracle_price, band_bps);

        assert_eq!(lower, 990_000);  // $0.99
        assert_eq!(upper, 1_010_000); // $1.01
    }

    #[test]
    fn test_price_band_zero_oracle() {
        let oracle_price = 0;
        let band_bps = 200;

        let (lower, upper) = calculate_price_band(oracle_price, band_bps);

        assert_eq!(lower, 0);
        assert_eq!(upper, 0);
    }

    #[test]
    fn test_price_band_high_price() {
        let oracle_price = 50_000_000; // $50.00
        let band_bps = 200;             // 2%

        let (lower, upper) = calculate_price_band(oracle_price, band_bps);

        assert_eq!(lower, 49_000_000);  // $49.00
        assert_eq!(upper, 51_000_000);  // $51.00
    }
}
