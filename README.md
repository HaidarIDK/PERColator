# Percolator: Formally Verified Risk Engine for Perpetual DEXs

> **⚠️ EDUCATIONAL USE ONLY - NOT PRODUCTION READY**
>
> This code is an experimental research project provided for educational purposes only. It is **NOT production ready** and should **NOT be used with real funds**. While the code includes formal verification proofs and comprehensive testing, it has not been independently audited for production use. Use at your own risk.

A reusable, formally verified risk engine for perpetual futures decentralized exchanges (perp DEXs) built on Solana. This module provides mathematical guarantees about fund safety even under oracle manipulation attacks.

## Core Innovation: PNL Warmup Protection

**The Problem:** In any perp DEX, oracle manipulation can allow attackers to extract value by creating artificial profits through manipulated price feeds.

**The Solution:** PNL warmup ensures that realized profits cannot be withdrawn instantly. Instead, they "warm up" over a time period T, during which any ADL (Auto-Deleveraging) events will haircut unwrapped PNL first, protecting user principal.

### Key Guarantee

> **If an attacker can manipulate the oracle for at most time T, and PNL requires time T to become withdrawable, then user principal is always safe.**

This allows us to formally verify that users can always withdraw their principal from the system (subject to margin requirements if they have an open position).

## Architecture

### Memory Layout

Fixed-size slab design with ~664 KB memory footprint, suitable for a single Solana account:

```
┌─────────────────────────────────────┐
│ RiskEngine                          │
├─────────────────────────────────────┤
│ - vault: u128                       │
│ - insurance_fund: InsuranceFund     │
│ - params: RiskParams                │
│ - current_slot: u64                 │
│ - funding_index_qpb_e6: i128        │
│ - last_funding_slot: u64            │
│ - loss_accum: u128                  │
│ - withdrawal_only: bool             │
│ - withdrawal_mode_withdrawn: u128   │
│ - warmup_paused: bool               │
│ - warmup_pause_slot: u64            │
│                                     │
│ - used: [u64; 64]                   │  ← Bitmap (4096 bits)
│ - free_head: u16                    │  ← Freelist head
│ - next_free: [u16; 4096]            │  ← Freelist chain
│                                     │
│ - accounts: [Account; 4096]         │  ← Fixed slab (users + LPs)
└─────────────────────────────────────┘
```

### Core Data Structures

#### Account (Unified for Users and LPs)
```rust
pub struct Account {
    pub kind: AccountKind,                // User or LP discrimination

    // Capital & PNL
    pub capital: u128,                    // Deposited capital (NEVER reduced by ADL - I1)
    pub pnl: i128,                        // Realized PNL (+ or -)
    pub reserved_pnl: u128,               // Pending withdrawals

    // Warmup (embedded fields)
    pub warmup_started_at_slot: u64,      // Warmup start time
    pub warmup_slope_per_step: u128,      // Warmup rate

    // Position
    pub position_size: i128,              // Current position
    pub entry_price: u64,                 // Entry price

    // Funding
    pub funding_index: i128,              // Funding index snapshot

    // Matcher info (only meaningful for LP kind)
    pub matcher_program: [u8; 32],        // Matching engine program ID
    pub matcher_context: [u8; 32],        // Matching engine context
}

#[repr(u8)]
pub enum AccountKind {
    User = 0,
    LP = 1,
}
```

**Key Innovation**: By unifying UserAccount and LPAccount into a single Account type:
- ✅ LPs receive **same risk management** as users (warmup, ADL, liquidations)
- ✅ Eliminates 1000+ lines of code duplication
- ✅ Makes I1 invariant truly universal (both users and LPs protected)
- ✅ Prevents LP insolvency from socializing losses to users
- ✅ Fixed-size structure (no Option fields), ideal for slab allocation

#### InsuranceFund
```rust
pub struct InsuranceFund {
    pub balance: u128,              // Current balance
    pub fee_revenue: u128,          // Accumulated trading fees
    pub liquidation_revenue: u128,  // Accumulated liquidation fees
}
```

### No-Std Slab Design

The risk engine uses a fixed 4096-account slab with zero heap allocation:

**Memory Management:**
- **Fixed slab:** `accounts: [Account; 4096]` (~655 KB)
- **Bitmap:** `used: [u64; 64]` tracks occupied slots (512 bytes)
- **Freelist:** `free_head: u16` + `next_free: [u16; 4096]` for O(1) allocation (~8 KB)
- **Total:** ~664 KB fixed memory footprint

**Benefits:**
- ✅ `#![no_std]` compatible - no heap allocation required
- ✅ Deterministic performance - no allocation pauses
- ✅ Cache-friendly - contiguous array layout
- ✅ Predictable memory - always same size regardless of account count
- ✅ Simpler verification - fixed arrays easier to prove in Kani

**Bitmap Iteration Algorithm:**

All operations that scan accounts (ADL, conservation checks) use tight bitmap iteration:

```rust
for (block, word) in self.used.iter().copied().enumerate() {
    let mut w = word;
    while w != 0 {
        let bit = w.trailing_zeros() as usize;
        let idx = block * 64 + bit;
        w &= w - 1;  // Clear lowest bit
        // Process accounts[idx]
    }
}
```

**How It Works:**
- Only visits occupied slots (skips empty ones)
- No allocation required
- Uses `trailing_zeros()` to find next set bit efficiently
- `w &= w - 1` clears the lowest set bit
- Works with both immutable (`for_each_used`) and mutable (`for_each_used_mut`) access

**Bitmap Operations:**
- Word index: `w = idx >> 6` (divide by 64)
- Bit index: `b = idx & 63` (modulo 64)
- Check used: `(used[w] >> b) & 1 == 1`
- Set used: `used[w] |= 1u64 << b`
- Clear used: `used[w] &= !(1u64 << b)`

**Freelist Management:**

O(1) account allocation using singly-linked list:

```rust
// Allocation
fn alloc_slot(&mut self) -> Result<u16> {
    if self.free_head == u16::MAX {
        return Err(RiskError::Overflow);  // Slab full
    }
    let idx = self.free_head;
    self.free_head = self.next_free[idx as usize];
    self.set_used(idx as usize);
    Ok(idx)
}

// Initialization (in new())
self.free_head = 0;
for i in 0..MAX_ACCOUNTS - 1 {
    self.next_free[i] = (i + 1) as u16;
}
self.next_free[MAX_ACCOUNTS - 1] = u16::MAX;  // Sentinel
```

**Account References:**

All account operations use `u16` indices (0..4095):
- `add_user()` and `add_lp()` return `u16` indices
- Functions take `idx: u16` instead of `usize`
- Single unified `accounts` array contains both users and LPs

**Stack Size:**

Due to the large fixed arrays, tests require increased stack size:

```bash
RUST_MIN_STACK=16777216 cargo test --all-feature
```

## Operations

### 1. Trading

**Pluggable Matching Engines:** Order matching is delegated to separate programs via CPI (Cross-Program Invocation). Each LP account specifies its matching engine program.

```rust
pub fn execute_trade<M: MatchingEngine>(
    &mut self,
    matcher: &M,                  // Matching engine implementation
    lp_idx: u16,                  // LP providing liquidity
    user_idx: u16,                // User trading
    oracle_price: u64,            // Oracle price
    size: i128,                   // Position size (+ long, - short)
) -> Result<()>
```

**Process:**
1. Settle funding for both user and LP (via unified `touch_account()`)
2. Call matching engine to validate/execute trade
3. Apply trading fee → insurance fund
4. Update LP and user positions
5. Realize PNL if reducing position
6. Abort if either account becomes undercollateralized

**Safety:** No whitelist needed for matching engines because:
- Matching engine validates trade authorization (implementation-specific)
- Both LP and user must remain solvent (checked by risk engine)
- Fees flow to insurance fund
- PNL changes are zero-sum between LP and user
- Funding settled before position changes

### 2. Deposits & Withdrawals

#### Deposit
```rust
pub fn deposit(&mut self, idx: u16, amount: u128) -> Result<()>
```
- Works for both users and LPs
- Always allowed
- Increases `capital` and `vault`
- Preserves conservation

#### Withdraw
```rust
pub fn withdraw(&mut self, idx: u16, amount: u128) -> Result<()>
```

Unified withdrawal function that handles both users, LPs, principal, and PNL:

**Process:**
1. Settle funding (via unified `touch_account()`)
2. Convert warmed-up PNL to capital
3. Withdraw requested amount from capital
4. Ensure margin requirements maintained if account has open position

**Key Properties:**
- Withdrawable = capital + warmed_up_pnl - margin_required
- Automatically converts vested PNL to capital before withdrawal
- Must maintain initial margin if position is open
- Never allows withdrawal below margin requirements
- Blocks if `withdrawal_only` mode is active

### 3. PNL Warmup

```rust
pub fn withdrawable_pnl(&self, acct: &Account) -> u128
```

**Formula:**
```
withdrawable = min(
    positive_pnl - reserved_pnl,
    slope_per_step × elapsed_slots
)
```

**Properties:**
- **Deterministic:** Same inputs always yield same output
- **Monotonic:** Withdrawable PNL never decreases over time (unless warmup paused)
- **Bounded:** Never exceeds available positive PNL

**Invariants (Formally Verified):**
- I5: PNL warmup is deterministic
- I5+: PNL warmup is monotonically increasing
- I5++: Withdrawable PNL ≤ available PNL

**Warmup Pause:**

During crisis mode (`warmup_paused == true`), all warmup calculations freeze at `warmup_pause_slot`:

```rust
pub fn withdrawable_pnl(&self, acct: &Account) -> u128 {
    if acct.pnl <= 0 {
        return 0;
    }

    let effective_slot = if self.warmup_paused {
        self.warmup_pause_slot  // Use frozen timestamp
    } else {
        self.current_slot       // Use current time
    };

    let elapsed = effective_slot.saturating_sub(acct.warmup_started_at_slot);
    let warmed = acct.warmup_slope_per_step.saturating_mul(elapsed as u128);

    let available = (acct.pnl as u128).saturating_sub(acct.reserved_pnl);
    core::cmp::min(warmed, available)
}
```

**Properties:**
- Prevents PNL from continuing to warm up during ADL
- Ensures consistent valuation across all accounts
- Automatically activated when system enters withdrawal-only mode
- Global freeze (affects all accounts uniformly)

### 4. ADL (Auto-Deleveraging)

```rust
pub fn apply_adl(&mut self, total_loss: u128) -> Result<()>
```

**Scan-Based Design:**

ADL uses a two-pass bitmap scan to apply proportional haircuts across all accounts with positive PNL:

**Pass 1: Compute Total Unwrapped PNL**
```rust
let mut total_unwrapped = 0u128;
for each used account in bitmap:
    if account.pnl > 0:
        withdrawable = withdrawable_pnl(account)
        unwrapped = pnl - withdrawable - reserved_pnl
        total_unwrapped += unwrapped
```

**Pass 2: Apply Proportional Haircuts**
```rust
if total_unwrapped > 0:
    for each used account in bitmap:
        if account.pnl > 0:
            unwrapped = calculate_unwrapped(account)
            haircut = (total_loss × unwrapped) / total_unwrapped
            account.pnl -= haircut  // Principal NEVER touched
```

**Pass 3: Insurance and Bad Debt**
```rust
remaining_loss = total_loss - socialized_to_unwrapped
if remaining_loss > 0:
    insurance_cover = min(remaining_loss, insurance_fund.balance)
    insurance_fund.balance -= insurance_cover

    leftover = remaining_loss - insurance_cover
    if leftover > 0:
        loss_accum += leftover
        withdrawal_only = true
        warmup_paused = true
```

**Example:**
```
3 accounts with unwrapped PNL:
- Account A: unwrapped = 1,000
- Account B: unwrapped = 2,000
- Account C: unwrapped = 1,000
Total unwrapped: 4,000

ADL loss: 2,000

Proportional haircuts:
- Account A: 2,000 × (1,000/4,000) = 500
- Account B: 2,000 × (2,000/4,000) = 1,000
- Account C: 2,000 × (1,000/4,000) = 500

All capital unchanged!
```

**Properties:**
- **No allocation:** Bitmap iteration is stack-only
- **Proportional fairness:** Each account loses same percentage of unwrapped PNL
- **Capital protection:** Only `pnl` field is modified (I1 invariant)
- **Waterfall ordering:** Unwrapped → Insurance → Bad debt (I4 invariant)

**Formal Guarantees:**
- I1: Account capital NEVER reduced by ADL
- I4: ADL haircuts unwrapped PNL before touching insurance
- I2: Conservation maintained throughout

### 5. Liquidations

```rust
pub fn liquidate_account(
    &mut self,
    victim_idx: u16,
    keeper_idx: u16,
    oracle_price: u64,
) -> Result<()>
```

**Unified liquidation function for both users and LPs:**

**Process:**
1. Check if account is below maintenance margin
2. If yes, close position (or reduce to safe size)
3. Realize PNL from position closure
4. Calculate liquidation fee
5. Split fee: 50% insurance fund, 50% keeper

**Maintenance Margin Check:**
```
collateral = capital + max(0, pnl)
position_value = abs(position_size) × oracle_price
margin_required = position_value × maintenance_margin_bps / 10,000

is_safe = collateral >= margin_required
```

**Safety:**
- Permissionless (no signature needed)
- Only affects undercollateralized accounts
- Keeper receives fee (incentive to call)
- Insurance fund builds reserves
- Works identically for users and LPs

### 6. Withdrawal-Only Mode (Crisis Shutdown)

When the insurance fund is depleted and losses exceed what can be covered (`loss_accum > 0`), the system automatically enters **withdrawal-only mode** - a controlled shutdown mechanism that blocks withdrawals until solvency is restored.

```rust
pub fn apply_adl(&mut self, total_loss: u128) -> Result<()>
```

**Trigger Condition:**
When ADL depletes the insurance fund, `withdrawal_only` flag is set:
```rust
if self.insurance_fund.balance < remaining_loss {
    // Insurance fund depleted - crisis mode
    self.loss_accum = remaining_loss - insurance_fund.balance;
    self.withdrawal_only = true;    // Block withdrawals
    self.warmup_paused = true;      // Freeze warmup
}
```

#### Crisis Mode Behavior

**Blocked Operations:**
- All withdrawals are blocked (return `Err(RiskError::WithdrawalOnlyMode)`)
- No new positions can be opened
- No increasing existing positions

**Allowed Operations:**
- Closing positions (users can reduce risk)
- Reducing positions
- Deposits (helps restore solvency)
- Insurance fund top-ups

**Position Management:**
```rust
// In withdrawal_only mode, only allow reducing positions
if new_position.abs() > current_position.abs() {
    return Err(RiskError::WithdrawalOnlyMode);
}
```

This allows orderly unwinding while preventing new risk-taking.

#### Recovery Path: Insurance Fund Top-Up

Anyone can contribute to the insurance fund to cover losses and restore normal operations:

```rust
pub fn top_up_insurance_fund(&mut self, amount: u128) -> Result<bool>
```

**Process:**
1. Contribution directly reduces `loss_accum` first
2. Remaining amount goes to insurance fund balance
3. If `loss_accum` reaches 0, **exits withdrawal-only mode** automatically
4. Trading resumes once solvency is restored

**Example:**
```rust
// System in crisis: loss_accum = 5,000
engine.withdrawal_only == true

// Someone tops up 3,000
engine.top_up_insurance_fund(3_000)?;
// loss_accum now 2,000, still in withdrawal mode

// Another 2,000 top-up
let exited = engine.top_up_insurance_fund(2_000)?;
assert!(exited);  // Exits withdrawal mode
assert!(!engine.withdrawal_only);  // Trading resumes
```

#### Design Benefits

**Simplicity:**
- Clear rule: no withdrawals during crisis
- No complex haircut calculations
- No O(N) scans for total capital computation

**Controlled Shutdown:**
- Prevents further risk-taking
- Blocks withdrawal runs
- Users can still close positions to reduce exposure

**Recovery Mechanism:**
- Clear path to restore solvency via insurance top-ups
- Automatic return to normal operations
- Transparent loss amount

**Security Properties:**
- No withdrawal order gaming
- No exploitation through position manipulation
- Clear separation of crisis vs normal operations
- Conservation maintained throughout

### 7. Funding Rates

**O(1) Perpetual Funding** with cumulative index pattern:

```rust
pub fn accrue_funding(
    &mut self,
    now_slot: u64,
    oracle_price: u64,
    funding_rate_bps_per_slot: i64,  // signed: + means longs pay shorts
) -> Result<()>
```

**How It Works:**

1. **Global Accrual (O(1)):**
   ```
   Δ F = price × rate_bps × dt / 10,000
   funding_index_qpb_e6 += ΔF
   ```
   - `funding_index_qpb_e6`: Cumulative funding index (quote per base, scaled by 1e6)
   - Updated once per slot, independent of account count

2. **Lazy Settlement (O(1) per account):**
   ```
   payment = position_size × (current_index - account_snapshot) / 1e6
   pnl -= payment
   account_snapshot = current_index
   ```
   - Settled automatically before any operation via unified `touch_account()`
   - Called before: withdrawals, trades, liquidations, position changes
   - Works identically for users and LPs

3. **Settle-Before-Mutate Pattern:**
   - **Critical for correctness:** Prevents double-charging on flips/reductions
   - `execute_trade()` calls `touch_account()` on both LP and user first
   - Ensures funding applies to old position before update

**Example:**
```rust
// Account has long position
account.position_size = 1_000_000;  // +1M base

// Accrue positive funding (+10 bps for 1 slot at $100)
engine.accrue_funding(1, 100_000_000, 10)?;
// Δ F = 100e6 × 10 × 1 / 10,000 = 100,000

// Settle funding
engine.touch_account(acct_idx)?;
// payment = 1M × 100,000 / 1e6 = 100,000
// account.pnl -= 100,000  (long pays)

// Reduce position - funding settled FIRST
engine.execute_trade(&matcher, lp_idx, user_idx, price, -500_000)?;
// 1. touch_account() settles any new funding on remaining position
// 2. Then position changes to 500,000
```

**Invariants (Formally Verified):**

| ID | Property | Proof |
|----|----------|-------|
| **F1** | Funding settlement is idempotent | `tests/kani.rs:537` |
| **F2** | Funding never modifies principal | `tests/kani.rs:579` |
| **F3** | Zero-sum between opposite positions | `tests/kani.rs:609` |
| **F4** | Settle-before-mutate correctness | `tests/kani.rs:651` |
| **F5** | No overflow on bounded inputs | `tests/kani.rs:696` |

**Units & Scaling:**
- `oracle_price`: u64 in 1e6 (e.g., 100_000_000 = $100)
- `position_size`: i128 in base units (signed: + long, - short)
- `funding_rate_bps_per_slot`: i64 (10 = 0.1% per slot)
- `funding_index_qpb_e6`: i128 quote-per-base in 1e6

**Convention:**
- Positive funding rate → longs pay shorts
- Negative funding rate → shorts pay longs
- Payment formula: `pnl -= position × ΔF / 1e6`

**Key Design Choices:**
1. **Checked arithmetic** (not saturating) - funding is zero-sum, silent overflow breaks invariants
2. **No iteration** - scales to unlimited accounts
3. **Automatic settlement** - users can't dodge funding by avoiding touch operations
4. **Rate supplied externally** - separates mark-vs-index logic from risk engine

## Formal Verification

All critical invariants are proven using Kani, a model checker for Rust.

### Invariants Proven

| ID | Property | File |
|----|----------|------|
| **I1** | Account capital NEVER reduced by ADL | `tests/kani.rs:45` |
| **I2** | Conservation of funds | `tests/kani.rs:76` |
| **I3** | Authorization enforced | (implied by signature checks) |
| **I4** | ADL haircuts unwrapped PNL first | `tests/kani.rs:236` |
| **I5** | PNL warmup deterministic | `tests/kani.rs:121` |
| **I5+** | PNL warmup monotonic | `tests/kani.rs:150` |
| **I5++** | Withdrawable ≤ available PNL | `tests/kani.rs:180` |
| **I7** | Account isolation | `tests/kani.rs:159` |
| **I8** | Collateral consistency | `tests/kani.rs:204` |

### Running Verification

```bash
# Install Kani
cargo install --locked kani-verifier
cargo kani setup

# Run all proofs
cargo kani

# Run specific proof
cargo kani --harness i1_adl_never_reduces_principal
```

## Testing

**Important:** All tests require increased stack size due to the large fixed arrays (4096 accounts, ~664 KB):

```bash
RUST_MIN_STACK=16777216 cargo test
```

This allocates 16 MB of stack space for the test runner.

### Unit Tests

Run all unit tests:
```bash
RUST_MIN_STACK=16777216 cargo test --test unit_tests
```

**44 tests passing**, comprehensive coverage:
- Bitmap allocation and freelist management
- Deposit/withdrawal mechanics (unified for users and LPs)
- PNL warmup over time with deterministic vesting
- Scan-based ADL with proportional haircuts
- Conservation invariants via bitmap scan
- Trading and position management
- Unified liquidation (users + LPs)
- Funding rate payments (longs/shorts)
- Funding settlement idempotence
- Funding with position changes and flips
- Withdrawal-only mode (crisis shutdown)
- Insurance fund top-up and recovery
- Warmup pause during crisis

### Fuzzing Tests

Run property-based tests:
```bash
RUST_MIN_STACK=16777216 cargo test --features fuzz --test fuzzing
```

Property-based tests using proptest:
- Random deposit/withdrawal sequences
- Conservation under chaos (random operations)
- Warmup monotonicity with random inputs
- Account isolation (operations don't affect unrelated accounts)
- Scan-based ADL with random losses
- Funding idempotence (random inputs)
- Funding zero-sum property
- Differential fuzzing (vs reference model)
- Funding with position flips/reductions

### Formal Verification

Run Kani model checker:
```bash
cargo kani
```

Formal proofs covering all critical invariants using symbolic execution:

**Specific proofs:**
```bash
# I1 invariant - principal never reduced by ADL
cargo kani --harness i1_adl_never_reduces_principal

# I5 invariant - warmup determinism
cargo kani --harness i5_warmup_deterministic

# I5+ invariant - warmup monotonicity
cargo kani --harness i5_warmup_monotonicity

# Conservation of funds
cargo kani --harness i2_conservation

# All funding invariants (F1-F5)
cargo kani --harness f1_funding_idempotent
cargo kani --harness f2_funding_never_touches_capital
cargo kani --harness f3_funding_zero_sum
```

See `tests/kani.rs` for all available proofs.

## Usage Example

```rust
use percolator::*;

// Initialize risk engine
let params = RiskParams {
    warmup_period_slots: 100,                  // ~50 seconds at 400ms/slot
    maintenance_margin_bps: 500,               // 5%
    initial_margin_bps: 1000,                  // 10%
    trading_fee_bps: 10,                       // 0.1%
    liquidation_fee_bps: 50,                   // 0.5%
    insurance_fee_share_bps: 5000,             // 50% to insurance
    account_fee_bps: 10000,                    // 1% account creation fee
};

let mut engine = RiskEngine::new(params);

// Add users and LPs (returns u16 indices)
let alice = engine.add_user(10_000)?;  // Pay account creation fee
let lp = engine.add_lp(matching_program_id, matching_context, 10_000)?;

// Alice deposits
engine.deposit(alice, 10_000)?;

// Define a matcher (NoOpMatcher for tests, or custom implementation)
let matcher = NoOpMatcher;

// Alice trades (via matching engine)
engine.execute_trade(&matcher, lp, alice, 1_000_000, 100)?;

// Time passes...
engine.advance_slot(50);

// Alice can withdraw (warmed-up PNL is automatically converted to capital)
let withdrawable = engine.withdrawable_pnl(&engine.accounts[alice as usize]);
engine.withdraw(alice, withdrawable)?;

// Liquidations (permissionless, unified for users + LPs)
let keeper = engine.add_user(10_000)?;
engine.liquidate_account(alice, keeper, oracle_price)?;
```

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Account creation | O(1) | Freelist allocation |
| Deposit/Withdraw | O(1) | Direct array access |
| Trade execution | O(1) | Direct array access |
| **ADL** | **O(N)** | Two bitmap scans (N = active accounts) |
| Liquidation | O(1) | Direct array access |
| Funding accrual | O(1) | Global index update |
| Conservation check | O(N) | Bitmap scan over active accounts |

### Space Complexity

- **Fixed:** O(4096) regardless of active accounts
- **Total size:** ~664 KB (Account[4096] + bitmap + freelist)

**Slab Size Breakdown:**
- Account struct: ~160 bytes each
- Account array: 4096 × 160 = ~655 KB
- Bitmap: 64 × 8 = 512 bytes
- Freelist: 4096 × 2 = ~8 KB
- **Total fixed footprint:** ~664 KB

### Benefits of Slab Design

1. **Predictable Memory Usage:** Fixed 664 KB, no dynamic allocation
2. **no_std Compatible:** Works in embedded/Solana runtime without allocator
3. **Cache Friendly:** Contiguous array layout improves cache locality
4. **Simpler Code:** No generic parameters, no trait abstractions
5. **Formal Verification:** Easier to reason about fixed arrays in Kani
6. **Deterministic Performance:** No heap allocation pauses

### Tradeoffs

**Advantages:**
- Deterministic memory and performance
- No heap allocation or garbage collection
- Simple, auditable implementation
- Formal verification friendly

**Limitations:**
- Fixed maximum of 4096 accounts
- Always consumes 664 KB even with few accounts
- ADL is O(N) instead of O(1) lazy collection
- No account deallocation (accounts never freed once allocated)

## Implementation Checklist

This section documents the slab 4096 rewrite implementation completed in January 2025.

### Hard Requirements (All Met)

- ✅ `#![no_std]` - No standard library
- ✅ `#![forbid(unsafe_code)]` - No unsafe code
- ✅ No `Vec`, no heap allocation, no `alloc` crate
- ✅ Single unified account array: 4096 max accounts
- ✅ Users and LPs distinguished by `kind` field (no `Option` fields)
- ✅ Iteration uses bitmap to skip unused slots
- ✅ All scans allocation-free and tight

### Implementation Steps Completed

1. ✅ **Design Document** - Created detailed specification
2. ✅ **Delete heap/Vec dependencies** - Removed all allocations
3. ✅ **Define constants and enums** - `MAX_ACCOUNTS`, `AccountKind`
4. ✅ **Replace Account structure** - Unified user/LP with `kind` field
5. ✅ **Rewrite RiskEngine as slab** - Fixed array with bitmap
6. ✅ **Implement bitmap helpers** - `is_used()`, `set_used()`, iteration
7. ✅ **Implement O(1) allocation** - Freelist with `alloc_slot()`
8. ✅ **Port funding settlement** - Unified `touch_account()`
9. ✅ **Port warmup math** - Embedded warmup fields, pause support
10. ✅ **Port deposit/withdraw** - Unified functions for all accounts
11. ✅ **Port trading** - Updated to use u16 indices
12. ✅ **Rewrite ADL** - Two-pass bitmap scan, no allocations
13. ✅ **Port liquidation** - Unified for users and LPs
14. ✅ **Add unit tests** - 44 tests passing
15. ✅ **Add Kani proofs** - All formal verification updated
16. ✅ **Delete dead code** - Removed old abstractions

### Breaking Changes from Previous Version

**API Changes:**
- All functions now take `u16` indices instead of `usize`
- `add_user()` and `add_lp()` return `u16` instead of `usize`
- Unified `touch_account(idx)` instead of separate `touch_user`/`touch_lp`
- Unified `withdraw(idx)` and `deposit(idx)` for all accounts
- Unified `liquidate_account()` replaces separate user/LP liquidation

**Behavioral Changes:**
- Withdrawal-only mode blocks all withdrawals (no proportional haircut)
- ADL is proportional across all accounts (scan-based, not lazy collection)
- No account deallocation (accounts never freed once allocated)
- Maximum 4096 accounts (hard limit)

**Removed Features:**
- Warmup rate cap (no `total_warmup_rate` tracking)
- O(1) ADL touch-based collection (reverted to scan-based)
- Withdrawal haircut during withdrawal-only mode (now blocks completely)
- Fee accrual system (simplified)
- Generic account storage abstractions

### Test Coverage

**Unit Tests:** 44 tests passing
- Bitmap allocation and iteration
- Scan-based ADL with proportional haircuts
- Warmup pause freezes withdrawable
- Withdrawal-only blocks withdrawals
- Conservation check via bitmap scan
- Funding settlement and idempotence
- Unified liquidation for users and LPs

**Fuzzing Tests:** Property-based testing with proptest
- Random deposit/withdrawal sequences
- Conservation under chaos
- Warmup monotonicity with random inputs
- Scan-based ADL with random losses

**Formal Verification:** Kani proofs for all critical invariants
- I1: Principal never reduced by ADL
- I2: Conservation of funds
- I4: ADL haircuts unwrapped PnL first
- I5/I5+/I5++: Warmup determinism and monotonicity

## Design Rationale

### Why PNL Warmup?

Traditional perp DEXs face an impossible tradeoff:
1. **Fast withdrawals** → vulnerable to oracle attacks
2. **Delayed withdrawals** → poor UX

PNL warmup solves this by:
- Allowing instant withdrawal of **capital** (users can always exit with their deposits)
- Requiring time for **PNL** to vest (prevents instant extraction of manipulated profits)
- Using ADL to haircut young PNL during crises (aligns incentives)

### Why Capital is Sacred?

By guaranteeing that `capital` is never touched by socialization (ADL):
1. Users can withdraw their deposits (subject to margin requirements if they have open positions)
2. The worst case is losing unrealized PNL, not capital deposits
3. Creates clear mental model: "capital is protected from ADL, profits take time to vest"
4. Applies equally to users and LPs

**Important:** Accounts with open positions must maintain margin requirements. If you have a position and negative PNL, you may not be able to withdraw all capital until you close the position.

### Why Pluggable Matching Engines?

Different trading styles need different matching:
- **CLOB** (Central Limit Order Book): Traditional exchange UX
- **AMM** (Automated Market Maker): Instant execution
- **RFQ** (Request for Quote): OTC/large trades
- **Auction**: Fair price discovery

By separating matching from risk:
1. Risk engine focuses on safety
2. Matching engines focus on price discovery
3. LPs can choose their preferred matching model
4. No need to trust/whitelist matching engines (safety checks in place)

## Safety Properties

### What This Guarantees

✅ **Account capital is protected from ADL** and withdrawable (subject to margin requirements)
✅ **PNL requires time T to become withdrawable**
✅ **ADL haircuts young PNL first, protecting capital**
✅ **Conservation of funds across all operations**
✅ **Account isolation - one account can't affect another**
✅ **No oracle manipulation can extract funds faster than time T**
✅ **Crisis mode blocks withdrawal runs** during insolvency
✅ **Clear recovery path via insurance fund top-ups**

### What This Does NOT Guarantee

❌ **Oracle is correct** (garbage in, garbage out)
❌ **Matching engine is fair** (LPs choose their matching engine)
❌ **Insurance fund is always solvent** (can be depleted in crisis)
❌ **Profits are guaranteed** (trading is risky!)

### Attack Resistance

| Attack Vector | Protection Mechanism |
|--------------|----------------------|
| Oracle manipulation | PNL warmup (time T) |
| Flash price attacks | Unrealized PNL not withdrawable |
| ADL abuse | Capital never touched |
| Matching engine exploit | Signature + solvency checks |
| Liquidation griefing | Permissionless + keeper incentives |
| Insurance drain | Fees replenish fund |
| Withdrawal runs | Crisis mode blocks all withdrawals |
| Crisis exploitation | Withdrawal-only mode blocks new risk |

## Dependencies

This implementation uses **no_std** and minimal dependencies:

- `pinocchio`: Solana program framework (no_std compatible)
- `pinocchio-log`: Logging for Solana programs

For testing:
- `proptest`: Property-based testing (fuzzing)
- `kani`: Formal verification

## Building and Testing

This is a library module designed to be used as a dependency in Solana programs.

### Build Commands

```bash
# Build the library
cargo build

# Build with optimizations
cargo build --release

# Check for compilation errors
cargo check
```

### Test Commands

**All tests require increased stack size:**

```bash
# Run all unit tests (44 tests)
RUST_MIN_STACK=16777216 cargo test --test unit_tests

# Run specific test
RUST_MIN_STACK=16777216 cargo test --test unit_tests test_warmup_pause

# Run AMM integration tests
RUST_MIN_STACK=16777216 cargo test --test amm_tests

# Run fuzzing tests (property-based testing)
RUST_MIN_STACK=16777216 cargo test --features fuzz --test fuzzing

# Run all tests (unit + fuzzing)
RUST_MIN_STACK=16777216 cargo test --all-features
```

### Formal Verification Commands

```bash
# Install Kani (first time only)
cargo install --locked kani-verifier
cargo kani setup

# Run all Kani proofs
cargo kani

# Run specific proof harnesses
cargo kani --harness i1_adl_never_reduces_principal
cargo kani --harness i5_warmup_monotonicity
cargo kani --harness f1_funding_idempotent
cargo kani --harness i2_conservation

# See all available harnesses
grep -n "kani::proof" tests/kani.rs
```

**Stack Size Requirement:** The `RUST_MIN_STACK=16777216` environment variable (16 MB) is required due to the large fixed arrays (4096 accounts, ~664 KB). This allocates sufficient stack space for tests. Kani proofs don't require this as they use symbolic execution.

### Using as a Dependency

Add to your Solana program's `Cargo.toml`:

```toml
[dependencies]
percolator = { git = "https://github.com/aeyakovenko/percolator", branch = "clean" }
```

Then import and use the risk engine:

```rust
use percolator::{RiskEngine, RiskParams, Account, AccountKind};

// Initialize risk engine with your parameters
let params = RiskParams {
    warmup_period_slots: 100,
    maintenance_margin_bps: 500,
    // ... other params
};

let mut engine = RiskEngine::new(params);

// Account indices are u16
let user_idx = engine.add_user(fee_payment)?;
let lp_idx = engine.add_lp(program_id, context, fee_payment)?;

// Access accounts via unified array
let user = &engine.accounts[user_idx as usize];
assert_eq!(user.kind, AccountKind::User);
```

## Project Structure

```
percolator/
├── src/
│   ├── lib.rs                 # Library entry point
│   └── percolator.rs          # Core implementation (~2500 lines)
│       ├── Constants & Enums  # MAX_ACCOUNTS, AccountKind
│       ├── Data Structures    # Account, RiskEngine, InsuranceFund
│       ├── Bitmap Operations  # is_used(), for_each_used()
│       ├── Account Lifecycle  # add_user(), add_lp(), alloc_slot()
│       ├── Core Operations    # deposit(), withdraw(), execute_trade()
│       ├── Risk Management    # apply_adl(), liquidate_account()
│       ├── Funding System     # accrue_funding(), touch_account()
│       ├── PNL Warmup         # withdrawable_pnl(), update_warmup_slope()
│       └── Utilities          # check_conservation(), advance_slot()
│
├── tests/
│   ├── unit_tests.rs          # 44 comprehensive unit tests
│   │   ├── Account creation and bitmap allocation
│   │   ├── Deposit/withdrawal mechanics
│   │   ├── Trading and position management
│   │   ├── PNL warmup and vesting
│   │   ├── Scan-based ADL tests
│   │   ├── Liquidation tests
│   │   ├── Funding rate tests
│   │   ├── Crisis mode tests
│   │   └── Conservation tests
│   │
│   ├── fuzzing.rs             # Property-based tests (--features fuzz)
│   │   ├── Random operation sequences
│   │   ├── Conservation under chaos
│   │   ├── Warmup monotonicity
│   │   └── Funding invariants
│   │
│   ├── kani.rs                # Formal verification proofs
│   │   ├── I1: Principal protection
│   │   ├── I2: Conservation
│   │   ├── I4: ADL waterfall
│   │   ├── I5/I5+/I5++: Warmup invariants
│   │   └── F1-F5: Funding invariants
│   │
│   └── amm_tests.rs           # AMM integration tests (3 tests)
│
├── Cargo.toml                 # Minimal dependencies (no_std)
├── README.md                  # Comprehensive documentation (this file)
└── LICENSE                    # Apache-2.0
```

**Key Files:**
- **`src/percolator.rs`**: Single implementation file (~2500 lines), no_std compatible
- **`tests/unit_tests.rs`**: Fast comprehensive tests covering all operations
- **`tests/fuzzing.rs`**: Property-based tests for finding edge cases
- **`tests/kani.rs`**: Formal proofs of critical safety properties
- **`tests/amm_tests.rs`**: Integration tests with AMM matching engine

## Contributing

This is a formally verified safety-critical module. All changes must:

1. ✅ Pass all unit tests (with `RUST_MIN_STACK=16777216`)
2. ✅ Pass all fuzzing tests
3. ✅ Pass all Kani proofs
4. ✅ Maintain `no_std` compatibility
5. ✅ Preserve all invariants (I1-I8)
6. ✅ Maintain fixed 4096 slab design (no heap allocation)

## License

Apache-2.0

## Security

**⚠️ IMPORTANT DISCLAIMER ⚠️**

This code is an **experimental research project** and is **NOT production ready**:

- ❌ **NOT independently audited** - No professional security audit has been performed
- ❌ **NOT battle-tested** - Has not been used in production environments
- ❌ **NOT complete** - Missing critical production features (proper oracle integration, signature verification, etc.)
- ❌ **NOT safe for real funds** - Should only be used for educational and research purposes

**Do NOT use this code with real money.** This is a proof-of-concept demonstrating formally verified risk engine design patterns for educational purposes only.

For security issues or questions, please open a GitHub issue.

## Acknowledgments

This design builds on the formally verified model from the original Percolator project, simplified and focused exclusively on the risk engine component.

## Further Reading

- [Kani Rust Verifier](https://model-checking.github.io/kani/)
- [Formal Verification in Rust](https://rust-formal-methods.github.io/)
- [Perpetual Futures Mechanics](https://www.paradigm.xyz/2021/05/everlasting-options)
- [Oracle Manipulation Attacks](https://research.paradigm.xyz/2022/08/oracle-attack)

---

**Built with formal methods. Guaranteed with mathematics. Powered by Rust.**
