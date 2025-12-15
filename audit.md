# Audit Report: `percolator.rs` Slab Rewrite

This audit assesses the implementation of the "slab rewrite" described in `plan.md`. The developer successfully refactored the architecture to a fixed-size slab model, removing all heap allocations. However, the audit, conducted with an adversarial mindset, has identified several new and significant issues.

## High Severity

### H1: Critical Accounting Error in `apply_adl` Scan

*   **Location:** `apply_adl` function.
*   **Description:** The ADL logic uses a two-pass scan. Pass 1 calculates `total_unwrapped` PNL. Pass 2 is intended to apply a proportional haircut. However, Pass 2 *recalculates* each account's individual `unwrapped` PNL after its `pnl` has already been modified by previous iterations within the same loop. The `unwrapped` value for a given account in Pass 2 is therefore different from the value that was used to compute the `total_unwrapped` denominator in Pass 1.
*   **Impact:** This is a critical accounting violation. The sum of all proportional `haircut` amounts will no longer equal the `loss_to_socialize`. This will lead to either insufficient deleveraging or an excessive haircut, breaking the system's conservation of funds invariant. This bug undermines the correctness of the entire ADL mechanism.
*   **Recommendation:** The `unwrapped` amount for each account must be cached during the first pass. A stack-allocated array (e.g., `let mut unwrapped_cache: [u128; MAX_ACCOUNTS] = [0; MAX_ACCOUNTS];`) should be used to store these values. The second pass must then use the cached values from this array for its haircut calculation, not recalculate them on the fly.

### H2: Account Creation Fee Bypass (TOCTOU)

*   **Location:** `add_user` and `add_lp` functions.
*   **Description:** These functions use `self.count_used()`—which performs a full scan—to determine the current number of users and calculate the account creation fee. In a blockchain environment, multiple actors can submit transactions in the same block. All of them will read the same initial state, calculate the same low fee based on the same `count_used`, and bypass the intended fee escalation mechanism.
*   **Impact:** This is a time-of-check-to-time-of-use (TOCTOU) vulnerability. It allows adversaries to mass-create accounts at the lowest possible fee tier, defeating the purpose of the dynamic fee and potentially leading to a denial-of-service by filling up the account slab cheaply.
*   **Recommendation:** The number of used accounts must be tracked in an `O(1)` counter field on the `RiskEngine` struct (e.g., `num_used_accounts: u64`). This counter should be incremented within `alloc_slot()` and used for the fee calculation, ensuring each allocation transaction correctly increases the fee basis for the next.

## Medium Severity

### M1: Inefficient Data Handling in `execute_trade`

*   **Location:** `execute_trade` function.
*   **Description:** To work around Rust's borrow checker, the implementation copies the user and LP account structs before calculating PNL changes, and then re-borrows them mutably to apply the updates. The comment `// Need to handle both accounts - copy data first to avoid borrow issues` confirms this was a deliberate but suboptimal choice.
*   **Impact:** While functionally correct, this pattern is inefficient. It creates unnecessary data copies and could contribute to higher compute unit consumption, especially in a function as critical as trading. An adversarial developer could leave such patterns in hot paths to degrade performance.
*   **Recommendation:** Refactor the function to use `accounts.split_at_mut()` to get simultaneous mutable references to two different elements in the slice. This is the idiomatic and efficient way to handle this situation in Rust, eliminating the need for copies.

### M2: Potential `max_accounts` Configuration Mismatch

*   **Location:** `add_user` and `add_lp` functions.
*   **Description:** The code checks `if used_count >= self.params.max_accounts`, using a configurable parameter from `RiskParams`. However, the system's memory is hardcoded to `const MAX_ACCOUNTS: usize = 4096`. If `params.max_accounts` is set to a value different from `4096`, it could lead to confusing behavior or errors. For instance, setting it higher than 4096 would cause the fee logic to behave unexpectedly as the slab nears full capacity.
*   **Impact:** Inconsistent system limits can lead to unpredictable behavior and incorrect fee calculations.
*   **Recommendation:** The system should rely on a single source of truth. Either remove `params.max_accounts` and use the hardcoded constant everywhere, or ensure at initialization that `params.max_accounts` is less than or equal to `MAX_ACCOUNTS` and use it consistently.

### M3: Inconsistent `warmup_paused` Logic

*   **Location:** `withdrawable_pnl` function.
*   **Description:** The logic for pausing warmups is `if self.warmup_paused { self.warmup_pause_slot } else { self.current_slot }`. This is less robust than the logic discussed in previous plans (`core::cmp::min(self.current_slot, self.warmup_pause_slot)`), which would handle edge cases where `current_slot` might appear to be before the `warmup_pause_slot`.
*   **Impact:** While unlikely, bugs or manipulation of the slot-advancement logic could lead to incorrect PNL warmup calculations.
*   **Recommendation:** Adopt the more robust `min(self.current_slot, self.warmup_pause_slot)` logic to ensure correctness even in unusual timing scenarios.

## Low Severity

### L1: Self-Liquidation is Not Prevented

*   **Location:** `liquidate_account` function.
*   **Description:** The function takes a `victim_idx` and a `keeper_idx` but does not check if they are the same.
*   **Impact:** An account can act as the keeper for its own liquidation. While not a direct exploit, this is unusual behavior and could lead to unexpected accounting, as the keeper bonus would be paid to the same account that is being liquidated.
*   **Recommendation:** Add a check `if victim_idx == keeper_idx { return Err(RiskError::Unauthorized) }` or a similar error to prevent an account from liquidating itself.
