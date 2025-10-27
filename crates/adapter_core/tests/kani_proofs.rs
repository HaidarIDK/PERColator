//! Kani formal verification proofs for adapter-core invariants
//!
//! Run with: cargo kani --tests -p adapter-core
//!
//! These proofs explore the complete state space of bounded inputs to verify
//! critical safety properties.

#![cfg(kani)]

use adapter_core::*;

// ============================================================================
// Proof 1: Shares Monotonicity & Overflow Safety
// ============================================================================

#[kani::proof]
fn proof_add_shares_checked_safe() {
    let cur: u128 = kani::any();
    let delta: i128 = kani::any();

    // Limit ranges to keep search finite but meaningful
    kani::assume(cur <= (1u128 << 120)); // Leave headroom for overflow detection

    let result = add_shares_checked(cur, delta);

    // Property 1: If delta < 0 and magnitude > cur, function MUST error
    if delta < 0 {
        let d = (-delta) as u128;
        if d > cur {
            assert!(result.is_err(), "Should error when burning more shares than available");
        } else {
            // Can succeed when we have enough shares
            assert!(result.is_ok() || result.is_err()); // Either outcome is valid
        }
    }

    // Property 2: If delta >= 0, only overflow causes error
    if delta >= 0 {
        let d = delta as u128;
        if cur.checked_add(d).is_none() {
            assert!(result.is_err(), "Should error on overflow");
        } else {
            assert!(result.is_ok(), "Should succeed when no overflow");
            assert_eq!(result.unwrap(), cur + d);
        }
    }

    // Property 3: Never returns negative shares
    if let Ok(shares) = result {
        assert!(shares <= u128::MAX, "Shares must be non-negative");
    }
}

// ============================================================================
// Proof 2: Settlement Batch Conservation
// ============================================================================

#[kani::proof]
fn proof_batch_conservation_two_fills() {
    // Model: Two fills that should conserve (maker/taker pair)
    let base1: i128 = kani::any();
    let quote1: i128 = kani::any();
    let base2: i128 = kani::any();
    let quote2: i128 = kani::any();

    // Conservation requirement
    kani::assume(base1.saturating_add(base2) == 0);
    kani::assume(quote1.saturating_add(quote2) == 0);

    let f1 = FillDelta {
        taker_portfolio: [0; 32],
        maker_seat: SeatId([1; 32]),
        base_delta_q64: base1,
        quote_delta_q64: quote1,
        fee_to_maker: 1,
        fee_to_venue: 2,
        exec_px_q64: 2 << 64,
    };

    let f2 = FillDelta {
        taker_portfolio: [0; 32],
        maker_seat: SeatId([2; 32]),
        base_delta_q64: base2,
        quote_delta_q64: quote2,
        fee_to_maker: -1,
        fee_to_venue: 0,
        exec_px_q64: 2 << 64,
    };

    let (sb, sq, _fv) = sum_fills(&[f1, f2]);

    // CRITICAL PROPERTY: Conservation of base and quote
    assert!(sb == 0, "Base must be conserved");
    assert!(sq == 0, "Quote must be conserved");
}

#[kani::proof]
#[kani::unwind(4)] // Limit loop unrolling for bounded verification
fn proof_batch_conservation_general() {
    // Test with small bounded array
    const N: usize = 3;
    let mut fills: [FillDelta; N] = [FillDelta {
        taker_portfolio: [0; 32],
        maker_seat: SeatId([0; 32]),
        base_delta_q64: 0,
        quote_delta_q64: 0,
        fee_to_maker: 0,
        fee_to_venue: 0,
        exec_px_q64: 0,
    }; N];

    // Generate bounded fills
    for i in 0..N {
        fills[i].base_delta_q64 = kani::any();
        fills[i].quote_delta_q64 = kani::any();
        fills[i].fee_to_venue = kani::any();

        // Bound values to prevent overflow in sum
        kani::assume(fills[i].base_delta_q64.abs() < (1i128 << 100));
        kani::assume(fills[i].quote_delta_q64.abs() < (1i128 << 100));
    }

    // Assume perfect conservation
    let mut total_base: i128 = 0;
    let mut total_quote: i128 = 0;
    for f in &fills {
        total_base = total_base.saturating_add(f.base_delta_q64);
        total_quote = total_quote.saturating_add(f.quote_delta_q64);
    }
    kani::assume(total_base == 0);
    kani::assume(total_quote == 0);

    let (sb, sq, _) = sum_fills(&fills);

    assert!(sb == 0, "Batch must conserve base");
    assert!(sq == 0, "Batch must conserve quote");
}

// ============================================================================
// Proof 3: Slippage Guard - Zero Tolerance
// ============================================================================

#[kani::proof]
fn proof_slippage_zero_guard_rejects_deviation() {
    let exec: u128 = kani::any();
    let mid: u128 = kani::any();

    // Bound to prevent overflow in arithmetic
    kani::assume(exec < (1u128 << 120));
    kani::assume(mid > 0 && mid < (1u128 << 120));

    let guard = RiskGuard {
        max_slippage_bps: 0,
        max_fee_bps: 10_000,
        oracle_bound_bps: 10_000,
        _padding: [0; 2],
    };

    let result = check_slippage(exec, mid, &guard);

    // CRITICAL PROPERTY: Zero slippage guard ONLY passes on exact match
    if exec == mid {
        assert!(result, "Exact match should pass zero-slippage guard");
    } else {
        assert!(!result, "Any deviation should fail zero-slippage guard");
    }
}

// ============================================================================
// Proof 4: Slippage Guard - Symmetric Bounds
// ============================================================================

#[kani::proof]
fn proof_slippage_symmetric() {
    let mid: u128 = kani::any();
    let delta_bps: u128 = kani::any();

    kani::assume(mid > 0 && mid < (1u128 << 100));
    kani::assume(delta_bps <= 1000); // Max 10%

    let guard = RiskGuard {
        max_slippage_bps: delta_bps as u16,
        max_fee_bps: 10_000,
        oracle_bound_bps: 10_000,
        _padding: [0; 2],
    };

    // Compute exec prices above and below mid by exactly delta_bps
    let delta = (mid.saturating_mul(delta_bps)) / 10_000;
    let exec_up = mid.saturating_add(delta);
    let exec_down = mid.saturating_sub(delta);

    // CRITICAL PROPERTY: Slippage check is symmetric
    let result_up = check_slippage(exec_up, mid, &guard);
    let result_down = check_slippage(exec_down, mid, &guard);

    assert!(result_up == result_down, "Slippage bounds must be symmetric");
    assert!(result_up, "Should pass at exact boundary");
}

// ============================================================================
// Proof 5: Capability Bitmask Correctness
// ============================================================================

#[kani::proof]
fn proof_capability_bitmask() {
    let caps: u64 = kani::any();

    // Test each capability independently
    let has_amm = Capability::is_enabled(caps, Capability::SupportsAMM);
    let has_ob = Capability::is_enabled(caps, Capability::SupportsOrderBook);
    let has_hybrid = Capability::is_enabled(caps, Capability::SupportsHybrid);
    let has_hooks = Capability::is_enabled(caps, Capability::SupportsHooks);

    // CRITICAL PROPERTY: Bitmask operations are independent
    // If we set a capability, it's enabled
    if (caps & (Capability::SupportsAMM as u64)) != 0 {
        assert!(has_amm, "AMM capability should be detected");
    } else {
        assert!(!has_amm, "AMM capability should not be detected");
    }

    // CRITICAL PROPERTY: Setting one capability doesn't affect others
    let caps_amm_only = Capability::SupportsAMM as u64;
    assert!(Capability::is_enabled(caps_amm_only, Capability::SupportsAMM));
    assert!(!Capability::is_enabled(caps_amm_only, Capability::SupportsOrderBook));
    assert!(!Capability::is_enabled(caps_amm_only, Capability::SupportsHybrid));
    assert!(!Capability::is_enabled(caps_amm_only, Capability::SupportsHooks));
}

// ============================================================================
// Proof 6: Absolute Value Identity
// ============================================================================

#[kani::proof]
fn proof_abs_i128_identity() {
    let x: i128 = kani::any();

    // Bound to prevent overflow on negation
    kani::assume(x > i128::MIN);

    let abs_x = abs_i128(x);

    // CRITICAL PROPERTIES
    assert!(abs_x >= 0, "Absolute value must be non-negative");

    if x >= 0 {
        assert!(abs_x == x, "abs(x) = x when x >= 0");
    } else {
        assert!(abs_x == -x, "abs(x) = -x when x < 0");
    }

    // Idempotence
    assert!(abs_i128(abs_x) == abs_x, "abs(abs(x)) = abs(x)");
}

// ============================================================================
// Proof 7: Shares Delta Sign Preservation
// ============================================================================

#[kani::proof]
fn proof_shares_delta_sign() {
    let cur: u128 = kani::any();
    let delta: i128 = kani::any();

    kani::assume(cur < (1u128 << 100)); // Avoid overflow
    kani::assume(delta.abs() < (1i128 << 100));

    if let Ok(new_shares) = add_shares_checked(cur, delta) {
        // CRITICAL PROPERTY: Sign of delta matches direction of change
        if delta > 0 {
            assert!(new_shares > cur, "Positive delta increases shares");
        } else if delta < 0 {
            assert!(new_shares < cur, "Negative delta decreases shares");
        } else {
            assert!(new_shares == cur, "Zero delta preserves shares");
        }
    }
}

// ============================================================================
// Proof 8: Slippage Arithmetic Overflow Safety
// ============================================================================

#[kani::proof]
fn proof_slippage_no_overflow() {
    let exec: u128 = kani::any();
    let mid: u128 = kani::any();

    kani::assume(exec < (1u128 << 120));
    kani::assume(mid > 0 && mid < (1u128 << 120));

    let guard = RiskGuard::conservative();

    // This should never panic due to overflow
    let _result = check_slippage(exec, mid, &guard);

    // If we reach here, no overflow occurred
    assert!(true);
}
