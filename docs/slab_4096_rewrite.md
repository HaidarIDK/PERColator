# Slab 4096 Rewrite Design Document

## Overview

This document describes the rewrite of the Percolator risk engine from a heap-allocated `Vec`-based design to a fixed 4096-account slab with bitmap-based iteration. The goal is to achieve `#![no_std]` compatibility with zero heap allocation while maintaining all existing safety properties and formal verification.

## Hard Requirements

- `#![no_std]` - No standard library
- `#![forbid(unsafe_code)]` - No unsafe code
- No `Vec`, no heap allocation, no `alloc` crate
- No `AccountStorage<T>` trait abstraction
- Single unified account array: 4096 max accounts
- Users and LPs distinguished by `kind` field (no `Option` fields)
- Iteration uses bitmap to skip unused slots
- All scans allocation-free and tight

## Memory Layout

### New Constants

```rust
pub const MAX_ACCOUNTS: usize = 4096;
pub const BITMAP_WORDS: usize = MAX_ACCOUNTS / 64; // 64 words of u64
```

### Account Kind Enum

```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountKind {
    User = 0,
    LP = 1,
}
```

### Account Structure

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Account {
    pub kind: AccountKind,

    // Capital & PnL
    pub capital: u128,
    pub pnl: i128,
    pub reserved_pnl: u128,

    // Warmup (embedded, no separate struct)
    pub warmup_started_at_slot: u64,
    pub warmup_slope_per_step: u128,

    // Position
    pub position_size: i128,
    pub entry_price: u64,

    // Funding
    pub funding_index: i128,

    // LP matcher info (only meaningful for LP kind)
    pub matcher_program: [u8; 32],
    pub matcher_context: [u8; 32],
}
```

**Key Changes from Old Design:**
- `Warmup` struct fields embedded directly
- `kind: AccountKind` instead of `Option<[u8;32]>` to distinguish user/LP
- `matcher_program` and `matcher_context` always present (zero for users)
- No more `fee_index`, `fee_accrued`, `vested_pos_snapshot` (simplified)
- All fields are `Copy`, no heap references

### RiskEngine Structure

```rust
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskEngine {
    // Vault and insurance
    pub vault: u128,
    pub insurance_fund: InsuranceFund,

    // Parameters
    pub params: RiskParams,

    // Time tracking
    pub current_slot: u64,

    // Funding rate state
    pub funding_index_qpb_e6: i128,
    pub last_funding_slot: u64,

    // Crisis mode
    pub loss_accum: u128,
    pub withdrawal_only: bool,
    pub withdrawal_mode_withdrawn: u128,

    // Warmup pause
    pub warmup_paused: bool,
    pub warmup_pause_slot: u64,

    // Occupancy tracking
    pub used: [u64; BITMAP_WORDS],  // 64 words, 4096 bits

    // Freelist (simple singly-linked list in array)
    pub free_head: u16,              // Index of first free slot (u16::MAX = none)
    pub next_free: [u16; MAX_ACCOUNTS],  // Next free slot for each index

    // Account slab
    pub accounts: [Account; MAX_ACCOUNTS],
}
```

**Removed from Old Design:**
- Generic parameters `<U, L>`
- `users: U` and `lps: L` separate collections
- `total_warmup_rate` and warmup rate cap (too complex for scan-free design)
- `total_user_capital`, `total_lp_capital`, `total_unwrapped_pnl`, `unwrapped_debt` (O(1) ADL aggregates - reverting to scan-based ADL)
- `fee_index`, `fee_carry` (fee system simplified)
- `sum_vested_pos_pnl` (unused)

## Indexing Scheme

All account references use `u16` indices (0..4095).

**Bitmap Operations:**
- Word index: `w = idx >> 6` (divide by 64)
- Bit index: `b = idx & 63` (modulo 64)
- Check used: `(used[w] >> b) & 1 == 1`
- Set used: `used[w] |= 1u64 << b`
- Clear used: `used[w] &= !(1u64 << b)`

## Bitmap Iteration Algorithm

### Tight Scan (Zero Allocation)

```rust
fn for_each_used_mut<F: FnMut(usize, &mut Account)>(&mut self, mut f: F) {
    for (block, word) in self.used.iter().copied().enumerate() {
        let mut w = word;
        while w != 0 {
            let bit = w.trailing_zeros() as usize;
            let idx = block * 64 + bit;
            w &= w - 1;  // Clear lowest bit
            f(idx, &mut self.accounts[idx]);
        }
    }
}
```

**Properties:**
- Only visits occupied slots (skips empty ones)
- No allocation
- Uses `trailing_zeros()` to find next bit efficiently
- `w &= w - 1` clears the lowest set bit

### Immutable Version

```rust
fn for_each_used<F: FnMut(usize, &Account)>(&self, mut f: F) {
    for (block, word) in self.used.iter().copied().enumerate() {
        let mut w = word;
        while w != 0 {
            let bit = w.trailing_zeros() as usize;
            let idx = block * 64 + bit;
            w &= w - 1;
            f(idx, &self.accounts[idx]);
        }
    }
}
```

## Freelist Management

### Allocation

```rust
fn alloc_slot(&mut self) -> Result<u16> {
    if self.free_head == u16::MAX {
        return Err(RiskError::Overflow);  // Slab full
    }
    let idx = self.free_head;
    self.free_head = self.next_free[idx as usize];
    self.set_used(idx as usize);
    Ok(idx)
}
```

### Initialization (in `new()`)

```rust
// Build freelist chain: 0 → 1 → 2 → ... → 4095 → NONE
self.free_head = 0;
for i in 0..MAX_ACCOUNTS - 1 {
    self.next_free[i] = (i + 1) as u16;
}
self.next_free[MAX_ACCOUNTS - 1] = u16::MAX;  // Sentinel
```

**Note:** No deallocation implemented (keeps design simplest). Accounts are never freed once allocated.

## Functions to Port

### Core Operations

1. **Account Creation**
   - `add_user(fee_payment: u128) -> Result<u16>`
   - `add_lp(program: [u8;32], context: [u8;32], fee_payment: u128) -> Result<u16>`

2. **Funding Settlement**
   - `touch_account(idx: u16) -> Result<()>` (unified, replaces `touch_user` and `touch_lp`)

3. **Warmup**
   - `withdrawable_pnl(&self, acct: &Account) -> u128`
   - `update_warmup_slope(&mut self, idx: u16) -> Result<()>`

4. **Deposits/Withdrawals**
   - `deposit(idx: u16, amount: u128) -> Result<()>`
   - `withdraw(idx: u16, amount: u128) -> Result<()>`
   - Aliases: `lp_deposit` and `lp_withdraw` (same implementation)

5. **Trading**
   - `execute_trade<M: MatchingEngine>(&mut self, matcher: &M, lp_idx: u16, user_idx: u16, oracle_price: u64, size: i128) -> Result<()>`

6. **ADL (Scan-Based)**
   - `apply_adl(&mut self, total_loss: u128) -> Result<()>`
   - Two-pass bitmap scan:
     1. Compute `total_unwrapped`
     2. Apply proportional haircuts to each account

7. **Liquidation**
   - `liquidate_account(victim_idx: u16, keeper_idx: u16, oracle_price: u64) -> Result<()>`

8. **Funding Rate**
   - `accrue_funding(now_slot: u64, oracle_price: u64, rate_bps_per_slot: i64) -> Result<()>`

9. **Utilities**
   - `check_conservation(&self) -> bool` (updated to scan slab)
   - `advance_slot(slots: u64)` (testing helper)

### Removed Functions

- All `touch_user` / `touch_lp` separate functions → unified `touch_account`
- All `users` / `lps` accessors → direct `accounts[idx]` access
- `update_user_warmup_slope` / `update_lp_warmup_slope` → unified `update_warmup_slope`
- O(1) ADL functions: `touch_adl_user`, `touch_adl_lp` → reverted to scan-based ADL
- Warmup rate cap functions (removed complexity)

## ADL Design: Scan-Based with Tight Loops

### Original Waterfall (Preserved)

1. **Unwrapped PnL first** (proportional haircut across all accounts)
2. **Insurance fund second** (after unwrapped exhausted)
3. **Bad debt third** (triggers withdrawal-only mode)

### Implementation (Two Bitmap Passes)

**Pass 1: Compute Total Unwrapped**
```rust
let mut total_unwrapped = 0u128;
self.for_each_used(|idx, acct| {
    if acct.pnl > 0 {
        let withdrawable = self.withdrawable_pnl(acct);
        let unwrapped = (acct.pnl as u128)
            .saturating_sub(withdrawable)
            .saturating_sub(acct.reserved_pnl);
        total_unwrapped = total_unwrapped.saturating_add(unwrapped);
    }
});
```

**Pass 2: Apply Proportional Haircuts**
```rust
let loss_to_socialize = core::cmp::min(total_loss, total_unwrapped);
if loss_to_socialize > 0 && total_unwrapped > 0 {
    self.for_each_used_mut(|idx, acct| {
        if acct.pnl > 0 {
            let withdrawable = self.withdrawable_pnl(acct);
            let unwrapped = (acct.pnl as u128)
                .saturating_sub(withdrawable)
                .saturating_sub(acct.reserved_pnl);
            let haircut = (loss_to_socialize * unwrapped) / total_unwrapped;
            acct.pnl = acct.pnl.saturating_sub(haircut as i128);
        }
    });
}
```

**Remaining Loss Handling**
```rust
let remaining_loss = total_loss.saturating_sub(loss_to_socialize);
if remaining_loss > 0 {
    // Debit insurance
    let insurance_cover = core::cmp::min(remaining_loss, self.insurance_fund.balance);
    self.insurance_fund.balance -= insurance_cover;

    let leftover = remaining_loss - insurance_cover;
    if leftover > 0 {
        // Enter crisis mode
        self.loss_accum += leftover;
        self.withdrawal_only = true;
        self.warmup_paused = true;
        self.warmup_pause_slot = self.current_slot;
    }
}
```

**Properties:**
- No allocations (no Vec, no intermediate lists)
- Two bitmap scans (O(N) but tight and cache-friendly)
- Proportional haircut (fair distribution)
- Preserves capital (I1 invariant)

## Withdrawal-Only Mode (Simplified)

### Old Behavior
- Proportional haircut on withdrawals using `loss_accum / total_principal` ratio
- Required O(N) scan to compute `total_principal`

### New Behavior (Simplest)
- **Block all withdrawals** when `withdrawal_only == true`
- Return `Err(RiskError::WithdrawalOnlyMode)`
- Recovery path: `top_up_insurance_fund()` to cover `loss_accum`
- Once `loss_accum == 0`, exit withdrawal-only mode

**Rationale:**
- Avoids O(N) scan for total capital computation
- Clear, simple rule: no withdrawals during crisis
- Users can still close positions (reduces risk)
- Insurance top-up provides clear recovery path

## Warmup Pause

### Global Freeze

When `warmup_paused == true`:
- All `withdrawable_pnl()` calculations use `warmup_pause_slot` instead of `current_slot`
- Prevents PnL from continuing to warm up during crisis
- Ensures consistent valuation for ADL haircuts

### Implementation

```rust
pub fn withdrawable_pnl(&self, acct: &Account) -> u128 {
    if acct.pnl <= 0 {
        return 0;
    }

    let effective_slot = if self.warmup_paused {
        self.warmup_pause_slot
    } else {
        self.current_slot
    };

    let elapsed = effective_slot.saturating_sub(acct.warmup_started_at_slot);
    let warmed = acct.warmup_slope_per_step.saturating_mul(elapsed as u128);

    let available = (acct.pnl as u128).saturating_sub(acct.reserved_pnl);
    core::cmp::min(warmed, available)
}
```

## Test List

### Unit Tests (Step 13)

1. **test_bitmap_allocation**
   - Add 10 accounts
   - Verify used bits set correctly
   - Verify indices are unique and < 4096

2. **test_scan_adl_no_alloc**
   - Set up accounts with various PnL states
   - Run `apply_adl()`
   - Verify PnL reductions follow proportional haircut
   - Verify insurance usage order correct

3. **test_warmup_pause_freezes_withdrawable**
   - Set `warmup_paused = true`
   - Advance `current_slot`
   - Verify `withdrawable_pnl()` does not increase

4. **test_withdrawal_only_blocks_withdraw**
   - Set `withdrawal_only = true`
   - Attempt withdrawal
   - Verify returns `Err(WithdrawalOnlyMode)`

5. **test_conservation_slab_scan**
   - Perform sequence of deposits, trades, withdrawals
   - Call `check_conservation()` (updated to scan slab via bitmap)
   - Verify returns `true`

6. **test_freelist_allocation**
   - Allocate multiple accounts
   - Verify freelist updates correctly
   - Fill slab to MAX_ACCOUNTS
   - Verify next allocation returns `Err(Overflow)`

7. **test_unified_touch_account**
   - Create user and LP accounts
   - Advance funding index
   - Call `touch_account()` on both
   - Verify funding settlements applied correctly

### Kani Proofs (Step 14)

1. **kani_no_principal_reduction_adl**
   - Set up 1-3 accounts with various PnL
   - Run `apply_adl(loss)`
   - Assert `capital` unchanged for all accounts
   - **Verifies I1 invariant**

2. **kani_insurance_after_unwrapped_exhausted**
   - Set up accounts with enough unwrapped PnL to cover loss
   - Run `apply_adl(loss)` where `loss < total_unwrapped`
   - Assert `insurance_fund.balance` unchanged
   - **Verifies waterfall ordering**

3. **kani_bitmap_iteration_only_used_slots**
   - Construct bitmap with single bit set at index `i`
   - Place sentinel value in `accounts[i]`
   - Run bitmap iteration
   - Assert only index `i` was touched (sentinel modified)
   - **Verifies bitmap iteration correctness**

4. **kani_allocation_sets_used_bit**
   - Call `alloc_slot()`
   - Verify `is_used(idx)` returns `true`
   - Verify `idx < MAX_ACCOUNTS`

5. **kani_warmup_pause_invariant**
   - Set `warmup_paused = true` at slot N
   - Advance `current_slot` to N+100
   - Assert `withdrawable_pnl()` same as at slot N

## Migration Path

### Breaking Changes

1. **API Changes:**
   - All functions now take `u16` indices instead of `usize`
   - `add_user()` and `add_lp()` return `u16` instead of `usize`
   - Unified `touch_account(idx)` instead of separate `touch_user`/`touch_lp`
   - Unified `withdraw(idx)` and `deposit(idx)` for all accounts

2. **Removed Features:**
   - Warmup rate cap (no `total_warmup_rate` tracking)
   - O(1) ADL touch-based collection (reverted to scan-based)
   - Withdrawal haircut during withdrawal-only mode (now blocks completely)
   - Fee accrual system (simplified)

3. **Behavioral Changes:**
   - Withdrawal-only mode blocks all withdrawals (no proportional haircut)
   - ADL is proportional across all accounts (no lazy collection)
   - No account deallocation (accounts never freed once allocated)

### Compatibility Notes

- All existing tests need account index type changes (`usize` → `u16`)
- Tests using `engine.users[i]` → `engine.accounts[i]`
- Tests checking account kind need `accounts[i].kind == AccountKind::User`

## Performance Characteristics

### Time Complexity

| Operation | Old (Vec) | New (Slab) |
|-----------|-----------|------------|
| Account creation | O(1) | O(1) |
| Deposit/Withdraw | O(1) | O(1) |
| Trade execution | O(1) | O(1) |
| ADL | O(1)* | O(N) |
| Liquidation | O(1) | O(1) |
| Funding accrual | O(1) | O(1) |
| Conservation check | O(N) | O(N) |

*Old O(1) ADL was only for the `apply_adl()` call; collection via `touch_adl_*()` was still O(N) amortized.

### Space Complexity

- **Old:** `O(N)` dynamic (N = actual account count)
- **New:** `O(4096)` fixed (always same memory footprint)

**Slab Size Estimate:**
- Account: ~160 bytes
- Slab: 4096 × 160 = ~655 KB
- Bitmap: 64 × 8 = 512 bytes
- Freelist: 4096 × 2 = ~8 KB
- **Total:** ~664 KB fixed

## Formal Verification

All existing invariants remain verified with updated implementations:

| ID | Property | Location |
|----|----------|----------|
| **I1** | Principal never reduced by ADL | Scan-based ADL only modifies `pnl` |
| **I2** | Conservation of funds | Updated `check_conservation()` scans slab |
| **I4** | ADL haircuts unwrapped first | Two-pass scan preserves waterfall |
| **I5** | Warmup deterministic | Same formula, embedded fields |
| **I5+** | Warmup monotonic | Global pause prevents decrease |
| **I5++** | Withdrawable ≤ available | Formula unchanged |

## Benefits of Slab Design

1. **Predictable Memory Usage:** Fixed 664 KB, no dynamic allocation
2. **no_std Compatible:** Works in embedded/Solana runtime without allocator
3. **Cache Friendly:** Contiguous array layout improves cache locality
4. **Simpler Code:** No generic parameters, no trait abstractions
5. **Formal Verification:** Easier to reason about fixed arrays in Kani
6. **Deterministic Performance:** No heap allocation pauses

## Drawbacks

1. **Memory Overhead:** Always consumes 664 KB even with few accounts
2. **Account Limit:** Hard cap at 4096 accounts
3. **ADL Performance:** O(N) scan instead of O(1) lazy collection
4. **No Deallocation:** Accounts never freed (could add if needed)

## Implementation Checklist

- [ ] Step 0: Create this design doc
- [ ] Step 1: Delete heap/Vec dependencies
- [ ] Step 2: Define constants and enums
- [ ] Step 3: Replace Account structure
- [ ] Step 4: Rewrite RiskEngine as slab
- [ ] Step 5: Implement bitmap helpers
- [ ] Step 6: Implement O(1) allocation
- [ ] Step 7: Port funding settlement
- [ ] Step 8: Port warmup math
- [ ] Step 9: Port deposit/withdraw
- [ ] Step 10: Port trading
- [ ] Step 11: Rewrite ADL (scan-based)
- [ ] Step 12: Port liquidation
- [ ] Step 13: Add unit tests
- [ ] Step 14: Add Kani proofs
- [ ] Step 15: Delete dead code

## References

- Original plan.md specification
- Existing percolator.rs implementation
- Solana account model documentation
