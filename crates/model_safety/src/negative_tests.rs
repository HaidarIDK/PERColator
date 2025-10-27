//! Negative tests: Verify that invalid inputs and unauthorized operations are rejected
//!
//! These tests ensure that:
//! - Unauthorized operations (wrong signatures) are rejected
//! - Invalid inputs (out of bounds, overflow, etc.) are handled safely
//! - Precondition violations result in safe no-ops
//! - The system fails safely rather than panicking or corrupting state

use crate::state::*;
use crate::transitions::*;
use crate::helpers::*;

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Helper to create a valid test state
    // ========================================================================

    fn create_test_state() -> State {
        let account = Account {
            principal: 1000,
            pnl_ledger: 100,
            reserved_pnl: 0,
            warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
            position_size: 0,
            fee_index_user: 0,
            fee_accrued: 0,
            vested_pos_snapshot: 0,
        };

        let mut users = arrayvec::ArrayVec::<Account, 6>::new();
        users.push(account);

        State {
            vault: 1100,
            fees_outstanding: 0,
            users,
            params: Params {
                max_users: 6,
                withdraw_cap_per_step: 1000,
                maintenance_margin_bps: 50_000,
            },
            authorized_router: true,
            loss_accum: 0,
            fee_index: 0,
            sum_vested_pos_pnl: 0,
            fee_carry: 0,
        }
    }

    // ========================================================================
    // N1: Authorization Tests (Signature Validation)
    // ========================================================================

    #[test]
    fn n1_unauthorized_deposit_rejected() {
        let mut state = create_test_state();
        state.authorized_router = false; // Simulate unauthorized caller

        let before = state.clone();
        let after = deposit(state, 0, 500);

        // State should be unchanged
        assert_eq!(after.users[0].principal, before.users[0].principal,
            "N1: Unauthorized deposit must not change principal");
        assert_eq!(after.vault, before.vault,
            "N1: Unauthorized deposit must not change vault");
    }

    #[test]
    fn n1_unauthorized_withdraw_rejected() {
        let mut state = create_test_state();
        state.authorized_router = false;

        let before = state.clone();
        let after = withdraw_principal(state, 0, 100);

        assert_eq!(after.users[0].principal, before.users[0].principal,
            "N1: Unauthorized withdrawal must not change principal");
        assert_eq!(after.vault, before.vault,
            "N1: Unauthorized withdrawal must not change vault");
    }

    #[test]
    fn n1_unauthorized_trade_settle_rejected() {
        let mut state = create_test_state();
        state.authorized_router = false;

        let before = state.clone();
        let after = trade_settle(state, 0, 50);

        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N1: Unauthorized trade settlement must not change PnL");
        assert_eq!(after.vault, before.vault,
            "N1: Unauthorized trade settlement must not change vault");
    }

    #[test]
    fn n1_unauthorized_socialize_rejected() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = 500; // Positive PnL
        state.authorized_router = false;

        let before = state.clone();
        let after = socialize_losses(state, 100);

        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N1: Unauthorized loss socialization must not change PnL");
        assert_eq!(after.vault, before.vault,
            "N1: Unauthorized loss socialization must not change vault");
    }

    #[test]
    fn n1_unauthorized_liquidation_rejected() {
        let mut state = create_test_state();
        state.users[0].position_size = 100;
        state.users[0].pnl_ledger = -50; // Negative PnL

        let prices = Prices {
            p: [100_000_000; 4], // $100 for all price feeds
        };

        let before = state.clone();
        let after = liquidate_one_unauthorized(state, &prices);

        assert_eq!(after.users[0].position_size, before.users[0].position_size,
            "N1: Unauthorized liquidation must not change position");
        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N1: Unauthorized liquidation must not change PnL");
    }

    // ========================================================================
    // N2: Out-of-Bounds Access Tests
    // ========================================================================

    #[test]
    fn n2_deposit_invalid_user_index() {
        let state = create_test_state();
        let before = state.clone();

        // Try to deposit to user index 999 (doesn't exist)
        let after = deposit(state, 999, 500);

        // State should be unchanged (no panic, safe no-op)
        assert_eq!(after.vault, before.vault,
            "N2: Deposit to invalid user index must not change state");
        assert_eq!(after.users.len(), before.users.len(),
            "N2: Invalid deposit must not add users");
    }

    #[test]
    fn n2_withdraw_invalid_user_index() {
        let state = create_test_state();
        let before = state.clone();

        let after = withdraw_principal(state, 999, 100);

        assert_eq!(after.vault, before.vault,
            "N2: Withdrawal from invalid user index must not change state");
    }

    #[test]
    fn n2_trade_settle_invalid_user_index() {
        let state = create_test_state();
        let before = state.clone();

        let after = trade_settle(state, 999, 50);

        assert_eq!(after.vault, before.vault,
            "N2: Trade settlement for invalid user must not change state");
    }

    #[test]
    fn n2_withdraw_pnl_invalid_user_index() {
        let state = create_test_state();
        let before = state.clone();

        let after = withdraw_pnl(state, 999, 50, 10);

        assert_eq!(after.vault, before.vault,
            "N2: PnL withdrawal for invalid user must not change state");
    }

    // ========================================================================
    // N3: Insufficient Balance Tests
    // ========================================================================

    #[test]
    fn n3_withdraw_exceeds_principal() {
        let state = create_test_state();
        // User has 1000 principal, try to withdraw 2000

        let after = withdraw_principal(state.clone(), 0, 2000);

        // Should only withdraw available amount (1000)
        assert_eq!(after.users[0].principal, 0,
            "N3: Withdrawal should be capped at available balance");
        assert_eq!(after.vault, state.vault - 1000,
            "N3: Vault should decrease by actual withdrawn amount");
    }

    #[test]
    fn n3_withdraw_pnl_exceeds_available() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = 50; // Only 50 available

        // Try to withdraw 200 (more than available)
        let after = withdraw_pnl(state.clone(), 0, 200, 1000);

        // Should withdraw at most 50
        assert!(after.users[0].pnl_ledger >= 0,
            "N3: PnL withdrawal should not go negative");
        assert!(after.users[0].pnl_ledger <= state.users[0].pnl_ledger,
            "N3: PnL should not increase from withdrawal");
    }

    #[test]
    fn n3_socialize_with_no_winners() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = -100; // Loser, not winner

        let before = state.clone();
        let after = socialize_losses(state, 50);

        // Should be no-op since there are no winners
        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N3: Socializing with no winners should not change state");
        assert_eq!(after.vault, before.vault,
            "N3: Vault should not change when no winners to socialize to");
    }

    // ========================================================================
    // N4: Zero and Edge Value Tests
    // ========================================================================

    #[test]
    fn n4_zero_deposit() {
        let state = create_test_state();
        let before = state.clone();

        let after = deposit(state, 0, 0);

        // Zero deposit should be safe no-op
        assert_eq!(after.users[0].principal, before.users[0].principal,
            "N4: Zero deposit should not change principal");
        assert_eq!(after.vault, before.vault,
            "N4: Zero deposit should not change vault");
    }

    #[test]
    fn n4_zero_withdrawal() {
        let state = create_test_state();
        let before = state.clone();

        let after = withdraw_principal(state, 0, 0);

        assert_eq!(after.users[0].principal, before.users[0].principal,
            "N4: Zero withdrawal should not change principal");
    }

    #[test]
    fn n4_socialize_zero_deficit() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = 500; // Winner

        let before = state.clone();
        let after = socialize_losses(state, 0);

        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N4: Zero deficit socialization should not change state");
    }

    // ========================================================================
    // N5: Warmup/Vesting Constraint Tests
    // ========================================================================

    #[test]
    fn n5_withdraw_pnl_respects_warmup() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = 1000; // Large PnL
        state.users[0].warmup_state = Warmup {
            started_at_slot: 100,
            slope_per_step: 10,
        };

        // Try to withdraw at step 101 (only 1 step elapsed)
        // Should only be able to withdraw slope_per_step * steps_elapsed = 10 * 1 = 10
        let after = withdraw_pnl(state.clone(), 0, 1000, 101);

        let withdrawn = state.users[0].pnl_ledger - after.users[0].pnl_ledger;
        assert!(withdrawn <= 10,
            "N5: Withdrawal should be limited by warmup (withdrawn: {})", withdrawn);
    }

    #[test]
    fn n5_withdraw_pnl_before_warmup_start() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = 1000;
        state.users[0].warmup_state = Warmup {
            started_at_slot: 100,
            slope_per_step: 10,
        };

        // Try to withdraw at step 50 (before warmup started)
        let after = withdraw_pnl(state.clone(), 0, 1000, 50);

        // Should withdraw nothing (or very little)
        assert_eq!(after.users[0].pnl_ledger, state.users[0].pnl_ledger,
            "N5: Should not be able to withdraw before warmup period");
    }

    // ========================================================================
    // N6: Margin Requirement Tests
    // ========================================================================

    #[test]
    fn n6_withdraw_pnl_respects_margin() {
        let mut state = create_test_state();
        state.users[0].principal = 1000;
        state.users[0].pnl_ledger = 200;
        state.users[0].position_size = 2000; // Requires margin
        state.params.maintenance_margin_bps = 50_000; // 5% margin

        // Current collateral: 1000 + 200 = 1200
        // Required margin: 2000 * 0.05 = 100
        // Max withdrawable: 1200 - 100 = 1100
        // But PnL is only 200, so max is 200

        // Try to withdraw all PnL (which would violate margin)
        let after = withdraw_pnl(state.clone(), 0, 200, 1000);

        // Should preserve margin requirement
        let after_collateral = after.users[0].principal as i128 + after.users[0].pnl_ledger;
        let required_margin = (state.users[0].position_size * state.params.maintenance_margin_bps as u128 / 1_000_000) as i128;

        assert!(after_collateral >= required_margin,
            "N6: Withdrawal should preserve margin requirement (collateral: {}, required: {})",
            after_collateral, required_margin);
    }

    // ========================================================================
    // N7: Non-Liquidatable Account Tests
    // ========================================================================

    #[test]
    fn n7_liquidate_healthy_account_noop() {
        let mut state = create_test_state();
        state.users[0].principal = 10000;
        state.users[0].pnl_ledger = 1000; // Healthy
        state.users[0].position_size = 1000;

        let prices = Prices {
            p: [100_000_000; 4], // $100 for all price feeds
        };

        // Account is healthy, should not be liquidatable
        assert!(!is_liquidatable(&state.users[0], &prices, &state.params),
            "N7: Healthy account should not be liquidatable");

        let before = state.clone();
        let after = liquidate_account(state, 0, &prices);

        // Should be no-op
        assert_eq!(after.users[0].position_size, before.users[0].position_size,
            "N7: Liquidating healthy account should not change position");
        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N7: Liquidating healthy account should not change PnL");
    }

    #[test]
    fn n7_liquidate_one_when_none_liquidatable() {
        let mut state = create_test_state();
        state.users[0].principal = 10000;
        state.users[0].pnl_ledger = 1000;
        state.users[0].position_size = 1000;

        let prices = Prices {
            p: [100_000_000; 4], // $100 for all price feeds
        };

        let before = state.clone();
        let after = liquidate_one(state, &prices);

        // Should be complete no-op
        assert_eq!(after.vault, before.vault,
            "N7: liquidate_one with no liquidatables should not change vault");
        assert_eq!(after.users[0].position_size, before.users[0].position_size,
            "N7: liquidate_one with no liquidatables should not change positions");
    }

    // ========================================================================
    // N8: Overflow/Underflow Safety Tests
    // ========================================================================

    #[test]
    fn n8_max_deposit_safe() {
        let state = create_test_state();

        // Try to deposit u128::MAX (should saturate safely)
        let after = deposit(state.clone(), 0, u128::MAX);

        // Should not panic, but may saturate
        assert!(after.users[0].principal >= state.users[0].principal,
            "N8: Max deposit should not decrease principal");
    }

    #[test]
    fn n8_negative_pnl_withdrawal_safe() {
        let mut state = create_test_state();
        state.users[0].pnl_ledger = -500; // Negative PnL

        let before = state.clone();
        let after = withdraw_pnl(state, 0, 1000, 1000);

        // Should not withdraw from negative PnL
        assert_eq!(after.users[0].pnl_ledger, before.users[0].pnl_ledger,
            "N8: Cannot withdraw from negative PnL");
        assert_eq!(after.vault, before.vault,
            "N8: Vault should not change when withdrawing from negative PnL");
    }

    // ========================================================================
    // N9: State Consistency Tests
    // ========================================================================

    #[test]
    fn n9_operations_preserve_user_count() {
        let state = create_test_state();
        let user_count = state.users.len();

        let after = deposit(state.clone(), 0, 100);
        assert_eq!(after.users.len(), user_count,
            "N9: Deposit should not change user count");

        let after = withdraw_principal(state.clone(), 0, 50);
        assert_eq!(after.users.len(), user_count,
            "N9: Withdrawal should not change user count");

        let after = trade_settle(state.clone(), 0, 10);
        assert_eq!(after.users.len(), user_count,
            "N9: Trade settlement should not change user count");
    }

    #[test]
    fn n9_invalid_operations_preserve_full_state() {
        let state = create_test_state();

        // Invalid user index
        let after = deposit(state.clone(), 999, 100);
        assert_eq!(after.users.len(), state.users.len(),
            "N9: Invalid operation should preserve user count");
        assert_eq!(after.users[0].principal, state.users[0].principal,
            "N9: Invalid operation should preserve all user state");
        assert_eq!(after.vault, state.vault,
            "N9: Invalid operation should preserve vault");
        assert_eq!(after.loss_accum, state.loss_accum,
            "N9: Invalid operation should preserve loss_accum");
    }
}
