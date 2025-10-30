//! Unit tests for common types and math

#[cfg(test)]
mod math_tests {
    use crate::math::*;

    #[test]
    fn test_vwap_single_fill() {
        let (qty, notional) = update_vwap(0, 0, 100, 50_000);
        assert_eq!(qty, 100);
        assert_eq!(notional, 5_000_000);
        assert_eq!(calculate_vwap(notional, qty), 50_000);
    }

    #[test]
    fn test_vwap_multiple_fills() {
        let (qty, notional) = update_vwap(0, 0, 100, 50_000);
        let (qty, notional) = update_vwap(qty, notional, 50, 51_000);
        assert_eq!(qty, 150);
        let vwap = calculate_vwap(notional, qty);
        // VWAP = (100*50000 + 50*51000) / 150 = 50,333.33...
        assert!(vwap >= 50_333 && vwap <= 50_334);
    }

    #[test]
    fn test_vwap_zero_qty() {
        let vwap = calculate_vwap(0, 0);
        assert_eq!(vwap, 0);
    }

    #[test]
    fn test_pnl_long_profit() {
        let pnl = calculate_pnl(10, 50_000, 51_000);
        assert_eq!(pnl, 10_000);
    }

    #[test]
    fn test_pnl_long_loss() {
        let pnl = calculate_pnl(10, 50_000, 49_000);
        assert_eq!(pnl, -10_000);
    }

    #[test]
    fn test_pnl_short_profit() {
        let pnl = calculate_pnl(-10, 50_000, 49_000);
        assert_eq!(pnl, 10_000);
    }

    #[test]
    fn test_pnl_short_loss() {
        let pnl = calculate_pnl(-10, 50_000, 51_000);
        assert_eq!(pnl, -10_000);
    }

    #[test]
    fn test_pnl_no_change() {
        let pnl = calculate_pnl(10, 50_000, 50_000);
        assert_eq!(pnl, 0);
    }

    #[test]
    fn test_funding_payment() {
        let payment = calculate_funding_payment(10, 1000, 500);
        assert_eq!(payment, 5000);
    }

    #[test]
    fn test_tick_alignment() {
        assert!(is_tick_aligned(50_000, 1000));
        assert!(is_tick_aligned(50_500, 500));
        assert!(!is_tick_aligned(50_123, 1000));
    }

    #[test]
    fn test_lot_alignment() {
        assert!(is_lot_aligned(100, 10));
        assert!(is_lot_aligned(100, 100));
        assert!(!is_lot_aligned(105, 10));
    }

    #[test]
    fn test_round_to_tick() {
        assert_eq!(round_to_tick(50_123, 1000), 50_000);
        assert_eq!(round_to_tick(50_999, 1000), 50_000);
        assert_eq!(round_to_tick(50_000, 1000), 50_000);
    }

    #[test]
    fn test_round_to_lot() {
        assert_eq!(round_to_lot(105, 10), 100);
        assert_eq!(round_to_lot(109, 10), 100);
        assert_eq!(round_to_lot(100, 10), 100);
    }

    #[test]
    fn test_calculate_im() {
        // qty=10, contract_size=1000, price=50000, imr=500 bps (5%)
        let im = calculate_im(10, 1000, 50_000, 500);
        // Notional = 10 * 1000 = 10,000
        // Value = 10,000 * 50,000 = 500,000,000
        // IM = 500,000,000 * 0.05 = 25,000,000
        assert_eq!(im, 25_000_000);
    }

    #[test]
    fn test_calculate_im_short() {
        // Short position should have same IM as long
        let im_long = calculate_im(10, 1000, 50_000, 500);
        let im_short = calculate_im(-10, 1000, 50_000, 500);
        assert_eq!(im_long, im_short);
    }

    #[test]
    fn test_calculate_mm() {
        let mm = calculate_mm(10, 1000, 50_000, 250);
        // MM = 500,000,000 * 0.025 = 12,500,000
        assert_eq!(mm, 12_500_000);
    }

    #[test]
    fn test_im_mm_relationship() {
        // IM should be >= MM for same position
        let im = calculate_im(10, 1000, 50_000, 500);
        let mm = calculate_mm(10, 1000, 50_000, 250);
        assert!(im >= mm);
    }

    #[test]
    fn test_margin_scales_with_quantity() {
        let im1 = calculate_im(10, 1000, 50_000, 500);
        let im2 = calculate_im(20, 1000, 50_000, 500);
        assert_eq!(im2, im1 * 2);
    }

    #[test]
    fn test_margin_scales_with_price() {
        let im1 = calculate_im(10, 1000, 50_000, 500);
        let im2 = calculate_im(10, 1000, 100_000, 500);
        assert_eq!(im2, im1 * 2);
    }
}

#[cfg(test)]
mod type_tests {
    use crate::types::*;

    #[test]
    fn test_side_default() {
        let side: Side = Default::default();
        assert_eq!(side, Side::Buy);
    }

    #[test]
    fn test_time_in_force_default() {
        let tif: TimeInForce = Default::default();
        assert_eq!(tif, TimeInForce::GTC);
    }

    #[test]
    fn test_maker_class_default() {
        let mc: MakerClass = Default::default();
        assert_eq!(mc, MakerClass::REG);
    }

    #[test]
    fn test_order_state_default() {
        let os: OrderState = Default::default();
        assert_eq!(os, OrderState::LIVE);
    }

    #[test]
    fn test_order_default() {
        let order: Order = Default::default();
        assert_eq!(order.order_id, 0);
        assert_eq!(order.qty, 0);
        assert_eq!(order.price, 0);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.state, OrderState::LIVE);
        assert!(!order.used);
    }

    #[test]
    fn test_position_default() {
        let pos: Position = Default::default();
        assert_eq!(pos.qty, 0);
        assert_eq!(pos.entry_px, 0);
        assert!(!pos.used);
    }
}
