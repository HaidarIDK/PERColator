# Percolator Security Audit

**Disclaimer:** This audit was performed by an AI assistant assuming an adversarial developer. It is not a substitute for a professional security audit.

## Summary

The Percolator codebase is a well-structured and security-conscious piece of software. The developer has clearly put a lot of thought into the design and implementation of the risk engine. The use of `saturating_*` arithmetic, formal verification with Kani, and a detailed set of invariants in the `README.md` are all commendable.

However, even the most well-designed systems can have vulnerabilities. This audit has identified a few issues that should be addressed to further improve the security of the system.

## Issues

### Critical

*   **[C-01] Unused and Undocumented `pinocchio` Dependency:** The `pinocchio` dependency is included in the `Cargo.toml` file but is not used anywhere in the codebase. This is a significant security risk, as the dependency could contain malicious code that could be executed at runtime. The fact that the dependency is not documented in the `README.md` is also a major red flag. An adversarial developer could have added this dependency as a backdoor.

*   **[C-02] Permanent Denial of Service via Account Slot Exhaustion:** The system has a fixed number of account slots (`MAX_ACCOUNTS`) and does not provide a mechanism for deallocating accounts. An attacker with sufficient funds could create new accounts until all the slots are filled, at which point no new users would be able to join the system. This is a permanent denial-of-service vulnerability that would require a hard fork to fix.

*   **[C-03] `apply_adl` Can Over-Charge Insurance/Inflate `loss_accum`:** In `apply_adl`, the sum of haircuts applied to user accounts can be less than `loss_to_socialize` due to integer division rounding. However, the `remaining_loss` is calculated as `total_loss.saturating_sub(loss_to_socialize)`, which assumes that the full `loss_to_socialize` was successfully applied. This discrepancy will cause the `remaining_loss` to be larger than it should be, leading to an over-charge of the insurance fund and/or an inflation of `loss_accum`. This is a conservation bug.

### High

*   **[H-01] PNL Can Stop Warming Due to Slope Truncation:** In `update_warmup_slope`, the `slope` is calculated as `positive_pnl / warmup_period_slots`. If `positive_pnl` is less than `warmup_period_slots`, the slope will be truncated to 0. This will cause the PNL to stop warming, and it will never become withdrawable.

### Medium

*   **[M-01] Test-Only `NoOpMatcher`:** The `NoOpMatcher` is a test-only matching engine that does not perform any validation of trades. If this were to be used in a production environment, it would allow any trade to be executed, regardless of price or size. This could lead to market manipulation and other economic attacks.

*   **[M-02] `warmup_insurance_reserved` is Monotone and Never Releases:** The `warmup_insurance_reserved` is a monotonically increasing value that is never released. This is a liveness issue, as it can cause the system to get stuck in a state where the spendable insurance is low, even if losses are later realized that would have covered the previously warmed profits.

*   **[M-03] `compute_unwrapped_pnl` Double-Subtracts Reserved PNL:** The `compute_unwrapped_pnl` function subtracts `reserved_pnl` twice in some cases. This could lead to unexpected behavior in the ADL process, as it makes the haircut avoid reserved PNL even if it is withdrawable.

### Low

*   **[L-01] Inconsistent Rounding Logic:** The `settle_account_funding` function uses a one-sided rounding logic to ensure that the vault always has at least what is owed. This is a good practice, but it is not used consistently throughout the codebase. For example, the `execute_trade` function uses truncating division when calculating PNL. This inconsistency could lead to small rounding errors that could be exploited by an attacker who repeatedly opens and closes small positions.

## Recommendations

*   **[R-01] Remove Unused Dependencies:** The `pinocchio` and `pinocchio-log` dependencies should be removed from the `Cargo.toml` file immediately.

*   **[R-02] Implement Account Deallocation:** A mechanism for deallocating accounts should be implemented to mitigate the DoS vulnerability. This mechanism should allow unused accounts to be removed from the system, freeing up slots for new users. A fee could be charged for deallocation to prevent spamming.

*   **[R-03] Fix `apply_adl` Rounding Bug:** Track the actual amount of haircuts applied and use that to calculate the remaining loss. To be even more robust, implement a remainder distribution step to ensure that the full `loss_to_socialize` is applied.

*   **[R-04] Fix PNL Warmup Stalling:** To prevent PNL from stalling, either enforce a minimum slope of 1 when `positive_pnl > 0`, or carry the fractional remainder explicitly in the `Account` struct.

*   **[R-05] Allow `warmup_insurance_reserved` to be Released:** After any event that increases `warmed_neg_total`, the `warmup_insurance_reserved` should be recalculated to allow it to be released.

*   **[R-06] Fix `compute_unwrapped_pnl` Logic:** The logic in `compute_unwrapped_pnl` should be reviewed and fixed to ensure that `reserved_pnl` is not double-subtracted.

*   **[R-07] Remove `NoOpMatcher` from Production Builds:** The `NoOpMatcher` should be removed from production builds of the software. This can be achieved by using conditional compilation flags to exclude it from release builds.

*   **[R-08] Use Consistent Rounding Logic:** The rounding logic should be made consistent across the entire codebase. All PNL calculations should use the same one-sided rounding logic as `settle_account_funding`. This will prevent rounding errors from accumulating and potentially being exploited.

*   **[R-09] Improve Testing:** Implement the Kani and fuzz testing strategies suggested by the user to catch these and other potential bugs.