# Percolator Security Audit

**Disclaimer:** This audit was performed by an AI assistant assuming an adversarial developer. It is not a substitute for a professional security audit.

## Summary

The Percolator codebase is well-structured with strong security focus:
- `saturating_*` arithmetic prevents overflow/underflow
- Formal verification with Kani proofs
- Comprehensive fuzz testing with invariant checks
- Atomic operations (Err => no mutation)

## Issues

### High

*   **[H-01] ~~Unused `pinocchio` Dependency~~:** FIXED - Removed unused `pinocchio` and `pinocchio-log` dependencies from `Cargo.toml`.

### Medium

*   **[M-01] No Account Deallocation:** Once account slots are allocated, they cannot be freed. While the exponential fee mechanism (`account_fee_multiplier`) makes slot exhaustion expensive (fees double as capacity fills), a determined attacker with sufficient capital could still exhaust all slots permanently. Consider adding account deallocation for inactive/empty accounts.

### Low

*   **[L-01] force_realize_losses Must Be Called Explicitly:** The `force_realize_losses` function is not auto-triggered. Callers must explicitly invoke it when insurance drops to threshold. This is intentional for atomicity but should be documented clearly.

### Informational

*   **[I-01] NoOpMatcher is Test-Only:** The `NoOpMatcher` accepts any trade at oracle price. This is appropriate for testing but the trait design allows production matchers to enforce proper price/size validation.

*   **[I-02] Large Stack Allocation:** `RiskEngine` is ~6MB on stack (4096 accounts). Tests use `Box::new()` to heap-allocate. On-chain deployment would need similar handling.

## Recommendations

*   **[R-01] ~~Remove Unused Dependencies~~:** DONE - Removed `pinocchio` and `pinocchio-log` from `Cargo.toml`.

*   **[R-02] Consider Account Deallocation:** Add a mechanism to reclaim slots from accounts with zero capital, zero position, and zero PnL. Could require a waiting period to prevent abuse.

*   **[R-03] Document force_realize_losses Calling Convention:** Make clear in API docs that callers must check `insurance_fund.balance <= risk_reduction_threshold` and call `force_realize_losses` before attempting trades in that state.
