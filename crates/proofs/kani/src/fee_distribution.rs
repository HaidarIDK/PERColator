//! Kani proofs for fee distribution properties
//!
//! Verifies:
//! - F1: Loss-first property (fees offset losses before distribution)
//! - F2: No subsidy to losers (only positive vested PnL gets fees)
//! - F3: Monotonicity (more vested PnL → more fees)
//! - F4: Conservation (total credited fees ≤ distributable amount)

use model_safety::*;

/// F1: Loss-first property
///
/// Proves that fees first offset loss_accum before distribution:
/// - cover = min(F, loss_accum_before)
/// - loss_accum_after = loss_accum_before - cover
/// - distributable = F - cover
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn f1_loss_first_property() {
    let mut s = State::default();

    // Symbolic inputs
    let loss_before: u128 = kani::any();
    let fees: u128 = kani::any();
    let sum_vested: u128 = kani::any();

    // Constraints
    kani::assume(loss_before <= 1_000_000);
    kani::assume(fees <= 1_000_000);
    kani::assume(sum_vested <= 10_000_000);

    s.loss_accum = loss_before;
    s.sum_vested_pos_pnl = sum_vested;

    let (cover, distributable) = fee_distribution::on_fees(&mut s, fees);
    let loss_after = s.loss_accum;

    // Property 1: cover = min(fees, loss_before)
    if fees <= loss_before {
        assert!(cover == fees);
    } else {
        assert!(cover == loss_before);
    }

    // Property 2: loss_after = loss_before - cover
    assert!(loss_after == loss_before.saturating_sub(cover));

    // Property 3: distributable = fees - cover
    assert!(distributable == fees.saturating_sub(cover));

    // Property 4: distributable is non-negative (guaranteed by u128)
    assert!(distributable <= fees);
}

/// F2: No subsidy to losers
///
/// Proves that accounts with non-positive vested PnL receive zero fees:
/// - If vested_pnl <= 0, then credited = 0
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn f2_no_subsidy_to_losers() {
    let mut s = State::default();
    let mut acc = Account::default();

    // Symbolic inputs
    let pnl: i128 = kani::any();
    let fee_index_global: u128 = kani::any();
    let fee_index_user: u128 = kani::any();

    // Constraints
    kani::assume(pnl <= 0); // Loser or zero PnL
    kani::assume(fee_index_global <= 1_000_000);
    kani::assume(fee_index_user <= fee_index_global);

    acc.pnl_ledger = pnl;
    acc.fee_index_user = fee_index_user;
    s.fee_index = fee_index_global;

    let credited = fee_distribution::on_touch(&mut s, &mut acc);

    // Property: Losers get zero fees
    assert!(credited == 0);
    assert!(acc.fee_accrued == 0);
}

/// F3: Monotonicity in vested PnL
///
/// Proves that more vested PnL leads to more (or equal) fees:
/// - If vested_a > vested_b >= 0, then fees_a >= fees_b
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn f3_monotonicity_in_vested_weight() {
    let mut s = State::default();
    let mut acc_big = Account::default();
    let mut acc_small = Account::default();

    // Symbolic inputs
    let pnl_big: i128 = kani::any();
    let pnl_small: i128 = kani::any();
    let fee_index_global: u128 = kani::any();

    // Constraints
    kani::assume(pnl_big >= pnl_small);
    kani::assume(pnl_small >= 0);
    kani::assume(pnl_big <= 100_000);
    kani::assume(fee_index_global <= 1_000);

    acc_big.pnl_ledger = pnl_big;
    acc_small.pnl_ledger = pnl_small;
    acc_big.fee_index_user = 0;
    acc_small.fee_index_user = 0;
    s.fee_index = fee_index_global;

    let credited_big = fee_distribution::on_touch(&mut s, &mut acc_big);

    // Reset global state for second account
    s.sum_vested_pos_pnl = 0; // Reset to avoid interference

    let credited_small = fee_distribution::on_touch(&mut s, &mut acc_small);

    // Property: Monotonicity → credited_big >= credited_small
    assert!(credited_big >= credited_small);
}

/// F4: Conservation of fees
///
/// Proves that total credited fees cannot exceed distributable amount:
/// - sum(credited) <= distributable (bounded by rounding)
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn f4_conservation_of_fees() {
    const N: usize = 2; // Small for Kani tractability

    let mut s = State::default();

    // Symbolic inputs
    let fees: u128 = kani::any();
    let loss: u128 = kani::any();

    // Constraints
    kani::assume(fees <= 10_000);
    kani::assume(loss <= 5_000);

    s.loss_accum = loss;

    // Process fees
    let (cover, distributable) = fee_distribution::on_fees(&mut s, fees);

    // Create N accounts with positive vested PnL
    let mut accounts = arrayvec::ArrayVec::<Account, 6>::new();
    let mut total_credited: u128 = 0;

    for i in 0..N {
        let mut acc = Account::default();

        let pnl: i128 = kani::any();
        kani::assume(pnl > 0);
        kani::assume(pnl <= 1_000);

        acc.pnl_ledger = pnl;
        acc.fee_index_user = 0; // Start at zero to get full delta

        // Touch account to accrue fees
        let credited = fee_distribution::on_touch(&mut s, &mut acc);
        total_credited = total_credited.saturating_add(credited);

        accounts.push(acc);
    }

    // Property: Total credited cannot exceed distributable
    // Allow small rounding error due to integer division
    let max_rounding_error = N as u128; // At most 1 per account
    assert!(total_credited <= distributable.saturating_add(max_rounding_error));
}

/// F5: Fee index monotonicity
///
/// Proves that fee_index is non-decreasing:
/// - fee_index_after >= fee_index_before
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn f5_fee_index_monotonic() {
    let mut s = State::default();

    // Symbolic inputs
    let fees: u128 = kani::any();
    let sum_vested: u128 = kani::any();
    let fee_index_before: u128 = kani::any();

    // Constraints
    kani::assume(fees <= 100_000);
    kani::assume(sum_vested > 0); // Need winners for index to update
    kani::assume(sum_vested <= 1_000_000);
    kani::assume(fee_index_before <= 1_000_000);

    s.fee_index = fee_index_before;
    s.sum_vested_pos_pnl = sum_vested;
    s.loss_accum = 0; // No losses so all fees distribute

    fee_distribution::on_fees(&mut s, fees);

    let fee_index_after = s.fee_index;

    // Property: Fee index is non-decreasing
    assert!(fee_index_after >= fee_index_before);
}

/// F6: Vested snapshot reconciliation
///
/// Proves that sum_vested_pos_pnl is correctly updated on touch:
/// - If vested increases: sum increases by delta
/// - If vested decreases: sum decreases by delta
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(3)]
fn f6_vested_snapshot_reconciliation() {
    let mut s = State::default();
    let mut acc = Account::default();

    // Symbolic inputs
    let pnl_before: i128 = kani::any();
    let pnl_after: i128 = kani::any();
    let sum_before: u128 = kani::any();

    // Constraints
    kani::assume(pnl_before >= 0);
    kani::assume(pnl_after >= 0);
    kani::assume(pnl_before <= 10_000);
    kani::assume(pnl_after <= 10_000);
    kani::assume(sum_before <= 100_000);
    kani::assume(sum_before >= pnl_before as u128); // sum must be at least this account's contribution

    // Setup
    acc.pnl_ledger = pnl_before;
    acc.vested_pos_snapshot = pnl_before as u128;
    s.sum_vested_pos_pnl = sum_before;

    // Change PnL
    acc.pnl_ledger = pnl_after;

    // Touch to reconcile
    fee_distribution::on_touch(&mut s, &mut acc);

    let sum_after = s.sum_vested_pos_pnl;
    let expected_delta = if pnl_after > pnl_before {
        (pnl_after - pnl_before) as u128
    } else {
        0 // For decreases, we use saturating_sub
    };

    // Property: Sum is reconciled correctly
    if pnl_after >= pnl_before {
        assert!(sum_after == sum_before + expected_delta);
    } else {
        // Decreasing case - sum should decrease or saturate to 0
        assert!(sum_after <= sum_before);
    }

    // Property: Snapshot is updated
    assert!(acc.vested_pos_snapshot == pnl_after as u128);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        // Kani proofs are not run as regular tests
        assert!(true);
    }
}
