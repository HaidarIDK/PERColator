# ⚠️ CRITICAL: LP Risk Management Asymmetry

## Executive Summary

**CRITICAL SECURITY ISSUE IDENTIFIED**: The Percolator risk engine implements comprehensive risk management for **Users only**. LPs (Liquidity Providers) are **completely excluded** from:
- PNL Warmup protection
- ADL (Auto-Deleveraging)
- Liquidations
- Warmup rate limiting

This asymmetry allows LP losses to accumulate unbounded, ultimately socializing to users through withdrawal-mode haircuts, **violating the I1 invariant** that "user principal is always safe."

## The Problem

### Code Evidence

**Line 1070 in src/percolator.rs:**
```rust
self.update_warmup_slope(user_index)?;
// Note: LP warmup not implemented yet, would need similar call
```

**The developers knew about this gap but didn't fix it.**

### What's Missing for LPs

1. **NO PNL Warmup** - `lp_withdrawable_pnl()` returns full positive PNL immediately (no time-based vesting)
2. **NO ADL** - `apply_adl()` only haircuts `self.users`, excludes `self.lps` entirely
3. **NO Liquidations** - No `liquidate_lp()` function exists
4. **NO Rate Limiting** - LP warmup state exists but is never updated

### Attack Scenario

1. LP provides 100k capital
2. User opens 1M notional position, price manipulated 100%
3. User realizes 1M fake profit, **LP suffers -1M loss**
4. User's PNL warms up slowly (protected)
5. **LP's -1M loss is immediate and unbounded** (no protection)
6. LP underwater: `lp_capital (100k) + lp_pnl (-1M) = -900k`
7. No liquidation mechanism - LP stays underwater
8. Eventually triggers withdrawal mode
9. **Users' principal gets haircutted** to cover LP insolvency
10. **I1 invariant violated**

## Root Cause Analysis

The codebase duplicates account structures:

### UserAccount (13 fields)
- `principal`, `pnl_ledger`, `reserved_pnl`, `warmup_state`
- `position_size`, `entry_price`
- `fee_index_user`, `fee_accrued`, `vested_pos_snapshot`
- `funding_index_user`

### LPAccount (12 fields - nearly identical!)
- `lp_capital`, `lp_pnl`, `lp_reserved_pnl`, `lp_warmup_state`
- `lp_position_size`, `lp_entry_price`
- `funding_index_lp`
- **+ 2 unique:** `matching_engine_program`, `matching_engine_context`

**Only real difference:** LPs have matching engine identifiers.

**Everything else is duplicated**, leading to:
- 1000+ lines of duplicated risk management code
- LPs excluded from user-focused risk functions
- Asymmetric safety properties

## Recommended Solution: Account Unification

### Unified Account Type

```rust
pub struct Account {
    // Universal fields
    pub capital: u128,                    // principal OR lp_capital
    pub pnl: i128,                        // pnl_ledger OR lp_pnl
    pub reserved_pnl: u128,
    pub warmup_state: Warmup,
    pub position_size: i128,
    pub entry_price: u64,
    pub fee_index: u128,
    pub fee_accrued: u128,
    pub vested_pos_snapshot: u128,
    pub funding_index: i128,

    // Optional LP fields
    pub matching_engine_program: Option<[u8; 32]>,  // None = user, Some = LP
    pub matching_engine_context: Option<[u8; 32]>,
}

impl Account {
    pub fn is_lp(&self) -> bool {
        self.matching_engine_program.is_some()
    }
}
```

### Benefits

✅ **Fixes LP risk gap automatically** - same code handles both
✅ **Eliminates 1000+ lines of duplication**
✅ **Makes ADL fair** - single loop over all accounts
✅ **Enables LP liquidations** - `liquidate(account_index)` works for both
✅ **Enforces LP warmup** - `update_warmup_slope(account_index)` works for both
✅ **Makes I1 truly safe** - no LP losses to socialize

## Implementation Plan

### Phase 1: Type Unification
- [x] Create unified `Account` struct with optional LP fields
- [x] Update `RiskEngine<U, L>` to use `Account` for both type parameters
- [ ] Update all function signatures: `&UserAccount` → `&Account`

### Phase 2: Field Renaming
- [ ] `principal` → `capital`
- [ ] `pnl_ledger` → `pnl`
- [ ] `fee_index_user` → `fee_index`
- [ ] `funding_index_user` → `funding_index`
- [ ] `lp_capital` → `capital`
- [ ] `lp_pnl` → `pnl`
- [ ] `lp_position_size` → `position_size`
- [ ] etc.

### Phase 3: Function Unification
- [ ] Remove `lp_withdrawable_pnl()` - use `withdrawable_pnl()` for both
- [ ] Remove LP-specific functions - make all functions work for both
- [ ] Update `apply_adl()` to include LPs
- [ ] Update liquidation to work for both

### Phase 4: Testing
- [ ] Fix ~47 unit tests
- [ ] Fix ~5 AMM tests
- [ ] Update ~36 Kani proofs

### Phase 5: Documentation
- [ ] Update README
- [ ] Update invariants
- [ ] Document unified account model

## Estimated Effort

**Total:** 6-8 hours of focused work
- Type/signature updates: 1-2 hours
- Field renaming throughout codebase: 2-3 hours
- Test fixes: 2-3 hours
- Documentation: 1 hour

## Alternative: Minimal Fix (Not Recommended)

Instead of unification, could add LP-specific risk management:
1. Implement `update_lp_warmup_slope()`
2. Add LP Phase 1b to `apply_adl()`
3. Implement `liquidate_lp()`
4. Fix `lp_withdrawable_pnl()` to check warmup

**Why not recommended:**
- Maintains code duplication
- Error-prone (two copies of every function)
- Harder to verify safety properties
- Doesn't address root cause

## Current Status

**Started but paused** due to scope. Changes made:
1. ✅ Created unified `Account` struct
2. ✅ Updated `RiskEngine` generic parameters
3. ✅ Updated `withdrawable_pnl()` to use unified `Account`
4. ⏸️ Paused at field renaming (massive scope)

**Next steps:** Complete the refactor systematically, one phase at a time.

## Immediate Action Required

Until this is fixed, the system should:

1. **Add WARNING to README:**
   ```markdown
   ⚠️ **CRITICAL LIMITATION:** LP risk management not implemented.
   LP losses can accumulate unbounded and may socialize to users.
   DO NOT use in production until LP warmup/ADL/liquidation implemented.
   ```

2. **Update I1 Invariant docs:**
   ```markdown
   I1: User principal NEVER reduced by user-side ADL
   ⚠️  User principal CAN be haircutted in withdrawal mode if LP
   insolvency depletes insurance fund. Full protection requires LP
   risk management implementation.
   ```

3. **Disable LP operations (temporary):**
   ```rust
   pub fn add_lp(...) -> Result<usize> {
       return Err(RiskError::NotImplemented); // Until risk management added
   }
   ```

## Conclusion

This is not a minor bug - it's a **fundamental architectural gap**. The separation of `UserAccount` and `LPAccount` led directly to LPs being excluded from risk management.

**The fix is clear: unify accounts.** Then all risk management automatically applies to both users and LPs, and the system's safety properties become truly universal.

**Status:** Implementation started, requires completion.
**Priority:** CRITICAL
**Estimated completion:** 6-8 hours

---

*This document was created during audit review of the Percolator risk engine.*
*Analysis files: `/tmp/lp_asymmetry_analysis.md`, `/tmp/unified_account_design.md`*
