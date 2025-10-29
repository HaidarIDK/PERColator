//! v0 Funding Rate Tests
//!
//! Comprehensive E2E tests for the funding rate system implementation.
//! Tests verify properties F1-F5 proven with Kani in model_safety::funding.

use pinocchio::pubkey::Pubkey;

#[cfg(test)]
mod funding_rate_tests {
    use super::*;

    /// F-01: Test funding index accumulation in SlabHeader
    ///
    /// Scenario: Call update_funding_index multiple times and verify cumulative index grows
    #[test]
    fn test_f01_funding_index_accumulation() {
        use percolator_common::SlabHeader;
        use model_safety::funding::{MarketFunding, update_funding_index};

        let mut header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            50_000_000_000, // mark_px
            20,             // taker_fee_bps
            255,            // bump
        );

        // Initial state
        assert_eq!(header.cum_funding, 0);
        assert_eq!(header.last_funding_ts, 0);
        assert_eq!(header.funding_rate, 0);

        // Scenario: Mark price 1% above oracle ($50,500 vs $50,000)
        // With sensitivity 800 (8 bps/hour), expect funding rate = 8 bps/hour
        let mark_price = 50_500_000_000i64; // $50,500
        let oracle_price = 50_000_000_000i64; // $50,000
        let sensitivity = 800i64; // 8 bps per hour
        let dt_seconds = 3600u64; // 1 hour

        // First update
        let mut market = MarketFunding {
            cumulative_funding_index: header.cum_funding,
        };
        update_funding_index(&mut market, mark_price, oracle_price, sensitivity, dt_seconds)
            .expect("update_funding_index should succeed");

        header.cum_funding = market.cumulative_funding_index;
        header.last_funding_ts = dt_seconds;

        // After 1 hour at 1% premium, cumulative funding should be positive
        assert!(header.cum_funding > 0, "Cumulative funding should grow with mark > oracle");

        let first_cum_funding = header.cum_funding;

        // Second update (another hour)
        let mut market = MarketFunding {
            cumulative_funding_index: header.cum_funding,
        };
        update_funding_index(&mut market, mark_price, oracle_price, sensitivity, dt_seconds)
            .expect("update_funding_index should succeed");

        header.cum_funding = market.cumulative_funding_index;

        // Cumulative funding should continue growing
        assert!(header.cum_funding > first_cum_funding, "Cumulative funding should accumulate");
        assert_eq!(
            header.cum_funding,
            first_cum_funding * 2,
            "With constant rate, funding should accumulate linearly"
        );

        println!("✅ F-01 PASSED: Funding index accumulation verified");
        println!("   Initial cum_funding: 0");
        println!("   After 1 hour: {}", first_cum_funding);
        println!("   After 2 hours: {}", header.cum_funding);
    }

    /// F-02: Test funding sign correctness (F5 property)
    ///
    /// Scenario: Verify longs pay when mark > oracle, shorts pay when mark < oracle
    #[test]
    fn test_f02_funding_sign_correctness() {
        use model_safety::funding::{Position, MarketFunding, apply_funding};

        // Setup: Long position of +1 BTC
        let mut long_position = Position {
            base_size: 1_000_000, // +1 BTC (long)
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        // Scenario 1: Mark > Oracle → Longs pay, shorts receive
        // Positive funding index means longs paid
        let market_positive = MarketFunding {
            cumulative_funding_index: 100_000, // Positive funding accrued
        };

        apply_funding(&mut long_position, &market_positive);

        // Long position should have PAID (negative PnL change)
        assert!(
            long_position.realized_pnl < 0,
            "Longs should pay when mark > oracle (F5 property)"
        );
        assert_eq!(long_position.funding_index_offset, 100_000);

        let long_payment = long_position.realized_pnl;

        // Setup: Short position of -1 BTC
        let mut short_position = Position {
            base_size: -1_000_000, // -1 BTC (short)
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        apply_funding(&mut short_position, &market_positive);

        // Short position should have RECEIVED (positive PnL change)
        assert!(
            short_position.realized_pnl > 0,
            "Shorts should receive when mark > oracle (F5 property)"
        );

        // Verify conservation: long pays exactly what short receives (F1 property)
        assert_eq!(
            long_payment,
            -short_position.realized_pnl,
            "Conservation: long payment should equal short receipt (F1)"
        );

        println!("✅ F-02 PASSED: Funding sign correctness verified (F5 property)");
        println!("   Long position (mark > oracle): {} (paid)", long_payment);
        println!("   Short position (mark > oracle): {} (received)", short_position.realized_pnl);
    }

    /// F-03: Test funding idempotence (F3 property)
    ///
    /// Scenario: Applying funding twice with same index = applying once
    #[test]
    fn test_f03_funding_idempotence() {
        use model_safety::funding::{Position, MarketFunding, apply_funding};

        // Setup: Position with +1 BTC
        let mut position_once = Position {
            base_size: 1_000_000,
            realized_pnl: 1_000_000_000, // Existing $1,000 PnL
            funding_index_offset: 0,
        };

        let mut position_twice = position_once.clone();

        let market = MarketFunding {
            cumulative_funding_index: 50_000,
        };

        // Apply once
        apply_funding(&mut position_once, &market);
        let pnl_after_once = position_once.realized_pnl;
        let offset_after_once = position_once.funding_index_offset;

        // Apply twice with same index
        apply_funding(&mut position_twice, &market);
        apply_funding(&mut position_twice, &market);
        let pnl_after_twice = position_twice.realized_pnl;
        let offset_after_twice = position_twice.funding_index_offset;

        // F3 property: Applying twice = applying once
        assert_eq!(
            pnl_after_once,
            pnl_after_twice,
            "Idempotence: PnL should be same (F3 property)"
        );
        assert_eq!(
            offset_after_once,
            offset_after_twice,
            "Idempotence: Offset should be same (F3 property)"
        );

        println!("✅ F-03 PASSED: Funding idempotence verified (F3 property)");
        println!("   PnL after applying once: {}", pnl_after_once);
        println!("   PnL after applying twice: {}", pnl_after_twice);
        println!("   Both are identical ✓");
    }

    /// F-04: Test funding conservation (F1 property)
    ///
    /// Scenario: Equal and opposite positions should net to zero funding
    #[test]
    fn test_f04_funding_conservation() {
        use model_safety::funding::{Position, MarketFunding, apply_funding, net_funding_pnl};

        // Setup: Two equal and opposite positions
        let mut long_position = Position {
            base_size: 5_000_000, // +5 BTC
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        let mut short_position = Position {
            base_size: -5_000_000, // -5 BTC
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        let market = MarketFunding {
            cumulative_funding_index: 250_000,
        };

        // Apply funding to both positions
        apply_funding(&mut long_position, &market);
        apply_funding(&mut short_position, &market);

        // F1 property: Net funding should be zero
        let positions = [long_position, short_position];
        let net_funding = net_funding_pnl(&positions);

        assert_eq!(
            net_funding,
            0,
            "Conservation: Net funding across equal/opposite positions must be zero (F1 property)"
        );

        // Additional check: Long payment = short receipt
        assert_eq!(
            long_position.realized_pnl,
            -short_position.realized_pnl,
            "Long payment should equal negative short receipt"
        );

        println!("✅ F-04 PASSED: Funding conservation verified (F1 property)");
        println!("   Long position PnL: {}", long_position.realized_pnl);
        println!("   Short position PnL: {}", short_position.realized_pnl);
        println!("   Net funding: {} ✓", net_funding);
    }

    /// F-05: Test funding proportionality (F2 property)
    ///
    /// Scenario: Funding payment should be proportional to position size
    #[test]
    fn test_f05_funding_proportionality() {
        use model_safety::funding::{Position, MarketFunding, apply_funding};

        let market = MarketFunding {
            cumulative_funding_index: 100_000,
        };

        // Position 1: 1 BTC
        let mut position_1btc = Position {
            base_size: 1_000_000,
            realized_pnl: 0,
            funding_index_offset: 0,
        };
        apply_funding(&mut position_1btc, &market);
        let payment_1btc = position_1btc.realized_pnl;

        // Position 2: 2 BTC (2x size)
        let mut position_2btc = Position {
            base_size: 2_000_000,
            realized_pnl: 0,
            funding_index_offset: 0,
        };
        apply_funding(&mut position_2btc, &market);
        let payment_2btc = position_2btc.realized_pnl;

        // Position 3: 5 BTC (5x size)
        let mut position_5btc = Position {
            base_size: 5_000_000,
            realized_pnl: 0,
            funding_index_offset: 0,
        };
        apply_funding(&mut position_5btc, &market);
        let payment_5btc = position_5btc.realized_pnl;

        // F2 property: Payment proportional to size
        assert_eq!(
            payment_2btc,
            payment_1btc * 2,
            "2 BTC position should pay 2x (F2 property)"
        );
        assert_eq!(
            payment_5btc,
            payment_1btc * 5,
            "5 BTC position should pay 5x (F2 property)"
        );

        println!("✅ F-05 PASSED: Funding proportionality verified (F2 property)");
        println!("   1 BTC payment: {}", payment_1btc);
        println!("   2 BTC payment: {} (2x)", payment_2btc);
        println!("   5 BTC payment: {} (5x)", payment_5btc);
    }

    /// F-06: Test Portfolio funding offset tracking
    ///
    /// Scenario: Verify Portfolio correctly stores and retrieves funding offsets
    #[test]
    fn test_f06_portfolio_funding_offsets() {
        use percolator_router::state::Portfolio;

        // Create portfolio
        let mut portfolio = Portfolio::new();

        // Initially, all funding offsets should be zero
        assert_eq!(portfolio.get_funding_offset(0, 0), 0);
        assert_eq!(portfolio.get_funding_offset(1, 0), 0);

        // Set funding offset for slab 0, instrument 0
        portfolio.set_funding_offset(0, 0, 123_456);
        assert_eq!(portfolio.get_funding_offset(0, 0), 123_456);

        // Set funding offset for slab 1, instrument 0
        portfolio.set_funding_offset(1, 0, 789_012);
        assert_eq!(portfolio.get_funding_offset(1, 0), 789_012);

        // Verify first offset unchanged
        assert_eq!(portfolio.get_funding_offset(0, 0), 123_456);

        // Update existing offset
        portfolio.set_funding_offset(0, 0, 999_999);
        assert_eq!(portfolio.get_funding_offset(0, 0), 999_999);

        println!("✅ F-06 PASSED: Portfolio funding offset tracking verified");
    }

    /// F-07: Test bridge function apply_funding_to_position_verified
    ///
    /// Scenario: Verify bridge correctly applies funding from SlabHeader to Portfolio
    #[test]
    fn test_f07_bridge_funding_application() {
        use percolator_router::state::{Portfolio, model_bridge::apply_funding_to_position_verified};

        let mut portfolio = Portfolio::new();

        // Setup: Portfolio has exposure on slab 0, instrument 0
        portfolio.update_exposure(0, 0, 2_000_000); // +2 BTC long
        portfolio.pnl = 5_000_000_000; // Existing $5,000 PnL

        // Simulate SlabHeader with cumulative funding
        let market_cumulative_index = 150_000i128;

        // Apply funding through bridge
        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);

        // Verify funding was applied
        assert_eq!(
            portfolio.get_funding_offset(0, 0),
            market_cumulative_index,
            "Funding offset should be updated to market index"
        );

        // Funding payment = base_size * (cum_funding - offset) = 2_000_000 * 150_000
        let expected_payment = 2_000_000i128 * 150_000i128;
        let expected_pnl = 5_000_000_000i128 + expected_payment;
        assert_eq!(
            portfolio.pnl,
            expected_pnl,
            "PnL should include funding payment"
        );

        // Apply again with same index (idempotence test)
        let pnl_before_second = portfolio.pnl;
        apply_funding_to_position_verified(&mut portfolio, 0, 0, market_cumulative_index);
        assert_eq!(
            portfolio.pnl,
            pnl_before_second,
            "Second application with same index should be no-op (F3 idempotence)"
        );

        println!("✅ F-07 PASSED: Bridge funding application verified");
        println!("   Initial PnL: 5000000000");
        println!("   Funding payment: {}", expected_payment);
        println!("   Final PnL: {}", portfolio.pnl);
    }

    /// F-08: Test funding with mark below oracle (negative rate)
    ///
    /// Scenario: When mark < oracle, shorts pay and longs receive
    #[test]
    fn test_f08_negative_funding_rate() {
        use model_safety::funding::{MarketFunding, update_funding_index};

        // Scenario: Mark price 1% below oracle ($49,500 vs $50,000)
        // Funding should be negative → shorts pay, longs receive
        let mark_price = 49_500_000_000i64; // $49,500
        let oracle_price = 50_000_000_000i64; // $50,000
        let sensitivity = 800i64;
        let dt_seconds = 3600u64;

        let mut market = MarketFunding {
            cumulative_funding_index: 0,
        };

        update_funding_index(&mut market, mark_price, oracle_price, sensitivity, dt_seconds)
            .expect("update_funding_index should succeed");

        // With mark < oracle, cumulative funding should decrease (or be negative)
        assert!(
            market.cumulative_funding_index < 0,
            "Negative funding when mark < oracle"
        );

        // Test with positions
        use model_safety::funding::{Position, apply_funding};

        let mut long_position = Position {
            base_size: 1_000_000,
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        let mut short_position = Position {
            base_size: -1_000_000,
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        apply_funding(&mut long_position, &market);
        apply_funding(&mut short_position, &market);

        // When mark < oracle: longs receive (positive PnL), shorts pay (negative PnL)
        assert!(
            long_position.realized_pnl > 0,
            "Longs should receive when mark < oracle"
        );
        assert!(
            short_position.realized_pnl < 0,
            "Shorts should pay when mark < oracle"
        );

        // Conservation still holds
        assert_eq!(
            long_position.realized_pnl,
            -short_position.realized_pnl,
            "Conservation holds for negative funding"
        );

        println!("✅ F-08 PASSED: Negative funding rate verified");
        println!("   Cumulative funding: {}", market.cumulative_funding_index);
        println!("   Long position PnL: {} (received)", long_position.realized_pnl);
        println!("   Short position PnL: {} (paid)", short_position.realized_pnl);
    }

    /// F-09: Test incremental funding updates
    ///
    /// Scenario: Multiple small funding updates accumulate correctly
    #[test]
    fn test_f09_incremental_funding_updates() {
        use model_safety::funding::{Position, MarketFunding, apply_funding};

        let mut position = Position {
            base_size: 1_000_000,
            realized_pnl: 0,
            funding_index_offset: 0,
        };

        // Apply funding in 4 increments of 10,000 each
        for i in 1..=4 {
            let market = MarketFunding {
                cumulative_funding_index: 10_000i128 * i,
            };
            apply_funding(&mut position, &market);

            // After each application, offset should match current index
            assert_eq!(position.funding_index_offset, 10_000i128 * i);
        }

        // Final PnL should equal total cumulative funding * base_size
        // 40,000 * 1,000,000 = 40,000,000,000
        let expected_pnl = 40_000i128 * 1_000_000i128;
        assert_eq!(position.realized_pnl, expected_pnl);

        println!("✅ F-09 PASSED: Incremental funding updates verified");
        println!("   Final cumulative index: 40000");
        println!("   Position base size: 1000000");
        println!("   Final PnL: {}", position.realized_pnl);
    }

    /// F-10: Test funding with existing PnL
    ///
    /// Scenario: Funding payments correctly add to existing PnL
    #[test]
    fn test_f10_funding_with_existing_pnl() {
        use model_safety::funding::{Position, MarketFunding, apply_funding};

        let existing_pnl = 10_000_000_000i128; // $10,000 existing PnL
        let mut position = Position {
            base_size: 2_000_000,
            realized_pnl: existing_pnl,
            funding_index_offset: 0,
        };

        let market = MarketFunding {
            cumulative_funding_index: 50_000,
        };

        apply_funding(&mut position, &market);

        // Funding payment = 2,000,000 * 50,000 = 100,000,000,000
        let funding_payment = 2_000_000i128 * 50_000i128;
        let expected_final_pnl = existing_pnl + funding_payment;

        assert_eq!(
            position.realized_pnl,
            expected_final_pnl,
            "Funding should add to existing PnL"
        );

        println!("✅ F-10 PASSED: Funding with existing PnL verified");
        println!("   Existing PnL: {}", existing_pnl);
        println!("   Funding payment: {}", funding_payment);
        println!("   Final PnL: {}", position.realized_pnl);
    }
}
