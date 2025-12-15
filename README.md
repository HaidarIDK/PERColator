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

All data structures are laid out in a single contiguous memory chunk, suitable for a single Solana account:

```
┌─────────────────────────────────────┐
│ RiskEngine                          │
├─────────────────────────────────────┤
│ - vault: u128                       │
│ - insurance_fund: InsuranceFund     │
│ - users: Vec<UserAccount>           │
│ - lps: Vec<LPAccount>               │
│ - params: RiskParams                │
│ - current_slot: u64                 │
│ - fee_index: u128                   │
│ - sum_vested_pos_pnl: u128          │
│ - loss_accum: u128                  │
│ - fee_carry: u128                   │
│ - funding_index_qpb_e6: i128        │
│ - last_funding_slot: u64            │
└─────────────────────────────────────┘
```

### Core Data Structures

#### UserAccount
```rust
pub struct UserAccount {
    pub principal: u128,           // NEVER reduced by ADL (Invariant I1)
    pub pnl_ledger: i128,          // Realized PNL (+ or -)
    pub reserved_pnl: u128,        // Pending withdrawals
    pub warmup_state: Warmup,      // PNL vesting state
    pub position_size: i128,       // Current position
    pub entry_price: u64,          // Entry price
    pub funding_index_user: i128,  // Funding index snapshot
    // ... fee tracking fields
}
```

#### LPAccount
```rust
pub struct LPAccount {
    pub matching_engine_program: [u8; 32],    // Program ID
    pub matching_engine_context: [u8; 32],    // Context account
    pub lp_capital: u128,                      // LP deposits
    pub lp_pnl: i128,                          // LP PNL
    pub lp_reserved_pnl: u128,                 // Reserved LP PNL
    pub lp_warmup_state: Warmup,               // LP PNL warmup
    pub lp_position_size: i128,                // LP position
    pub lp_entry_price: u64,                   // LP entry
    pub funding_index_lp: i128,                // Funding index snapshot
}
```

#### InsuranceFund
```rust
pub struct InsuranceFund {
    pub balance: u128,              // Current balance
    pub fee_revenue: u128,          // Accumulated trading fees
    pub liquidation_revenue: u128,  // Accumulated liquidation fees
}
```

## Operations

### 1. Trading

**Pluggable Matching Engines:** Order matching is delegated to separate programs via CPI (Cross-Program Invocation). Each LP account specifies its matching engine program.

```rust
pub fn execute_trade<M: MatchingEngine>(
    &mut self,
    matcher: &M,                  // Matching engine implementation
    lp_index: usize,              // LP providing liquidity
    user_index: usize,            // User trading
    oracle_price: u64,            // Oracle price
    size: i128,                   // Position size (+ long, - short)
) -> Result<()>
```

**Process:**
1. Settle funding for both user and LP (via `touch_user()`/`touch_lp()`)
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
pub fn deposit(&mut self, user_index: usize, amount: u128) -> Result<()>
```
- Always allowed
- Increases `principal` and `vault`
- Preserves conservation

#### Withdraw
```rust
pub fn withdraw(&mut self, user_index: usize, amount: u128) -> Result<()>
```

Unified withdrawal function that handles both principal and PNL:

**Process:**
1. Settle funding (via `touch_user()`)
2. Convert warmed-up PNL to principal
3. Withdraw requested amount from principal
4. Ensure margin requirements maintained if user has open position

**Key Properties:**
- Withdrawable = principal + warmed_up_pnl - margin_required
- Automatically converts vested PNL to principal before withdrawal
- Must maintain initial margin if position is open
- Never allows withdrawal below margin requirements

### 3. PNL Warmup

```rust
pub fn withdrawable_pnl(&self, user: &UserAccount) -> u128
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
- **Monotonic:** Withdrawable PNL never decreases over time
- **Bounded:** Never exceeds available positive PNL

**Invariants (Formally Verified):**
- I5: PNL warmup is deterministic
- I5+: PNL warmup is monotonically increasing
- I5++: Withdrawable PNL ≤ available PNL

### 4. ADL (Auto-Deleveraging)

```rust
pub fn apply_adl(&mut self, total_loss: u128) -> Result<()>
```

**Process:**
1. **Phase 1:** Haircut unwrapped PNL first
   - Iterate through users
   - Calculate `unwrapped_pnl = positive_pnl - withdrawable - reserved`
   - Reduce `pnl_ledger` by haircut amount
   - **Principal is NEVER touched** (Invariant I1)

2. **Phase 2:** Apply remaining loss to insurance fund
   - If insurance insufficient, record in `loss_accum`

**Example:**
```
User has:
- principal: 10,000
- pnl_ledger: 5,000
- withdrawable (warmed up): 1,000
- reserved: 500

ADL loss: 4,000

Unwrapped PNL = 5,000 - 1,000 - 500 = 3,500
Haircut = min(3,500, 4,000) = 3,500 from PNL
Remaining = 4,000 - 3,500 = 500 from insurance

Result:
- principal: 10,000 (unchanged!)
- pnl_ledger: 1,500
- insurance: -500
```

**Formal Guarantees:**
- I1: User principal NEVER reduced by ADL
- I4: ADL haircuts unwrapped PNL before touching insurance
- I2: Conservation maintained throughout

### 5. Liquidations

```rust
pub fn liquidate_user(
    &mut self,
    user_index: usize,
    keeper_account: usize,
    oracle_price: u64,
) -> Result<()>
```

**Process:**
1. Check if user is below maintenance margin
2. If yes, close position (or reduce to safe size)
3. Realize PNL from position closure
4. Calculate liquidation fee
5. Split fee: 50% insurance fund, 50% keeper

**Maintenance Margin Check:**
```
collateral = principal + max(0, pnl_ledger)
position_value = abs(position_size) × oracle_price
margin_required = position_value × maintenance_margin_bps / 10,000

is_safe = collateral >= margin_required
```

**Safety:**
- Permissionless (no signature needed)
- Only affects undercollateralized accounts
- Keeper receives fee (incentive to call)
- Insurance fund builds reserves

### 6. Funding Rates

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
   payment = position_size × (current_index - user_snapshot) / 1e6
   pnl_ledger -= payment
   user_snapshot = current_index
   ```
   - Settled automatically before any operation via `touch_user()`/`touch_lp()`
   - Called before: withdrawals, trades, liquidations, position changes

3. **Settle-Before-Mutate Pattern:**
   - **Critical for correctness:** Prevents double-charging on flips/reductions
   - `execute_trade()` calls `touch_user()` and `touch_lp()` first
   - Ensures funding applies to old position before update

**Example:**
```rust
// User has long position
user.position_size = 1_000_000;  // +1M base

// Accrue positive funding (+10 bps for 1 slot at $100)
engine.accrue_funding(1, 100_000_000, 10)?;
// Δ F = 100e6 × 10 × 1 / 10,000 = 100,000

// Settle funding
engine.touch_user(user_idx)?;
// payment = 1M × 100,000 / 1e6 = 100,000
// user.pnl_ledger -= 100,000  (long pays)

// Reduce position - funding settled FIRST
engine.execute_trade(&matcher, lp_idx, user_idx, price, -500_000)?;
// 1. touch_user() settles any new funding on remaining position
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
| **I1** | User principal NEVER reduced by ADL | `tests/kani.rs:21` |
| **I2** | Conservation of funds | `tests/kani.rs:44` |
| **I3** | Authorization enforced | (implied by signature checks) |
| **I4** | ADL haircuts unwrapped PNL first | `tests/kani.rs:236` |
| **I5** | PNL warmup deterministic | `tests/kani.rs:75` |
| **I5+** | PNL warmup monotonic | `tests/kani.rs:105` |
| **I5++** | Withdrawable ≤ available PNL | `tests/kani.rs:131` |
| **I7** | User isolation | `tests/kani.rs:159` |
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

### Unit Tests (Fast)
```bash
cargo test
```

33 unit tests covering:
- Deposit/withdrawal mechanics
- PNL warmup over time
- ADL haircut logic
- Conservation invariants
- Trading and position management
- Liquidation mechanics
- Funding rate payments (longs/shorts)
- Funding settlement idempotence
- Funding with position changes

### Fuzzing Tests
```bash
cargo test --features fuzz
```

22 property-based tests using proptest:
- Random deposit/withdrawal sequences
- Conservation under chaos
- Warmup monotonicity with random inputs
- Multi-user isolation
- ADL with random losses
- Funding idempotence (random inputs)
- Funding zero-sum property
- Differential fuzzing (vs reference model)
- Funding with position flips/reductions

### Formal Verification
```bash
cargo kani
```

26+ formal proofs covering all critical invariants (including funding).

## Usage Example

```rust
use percolator::*;

// Initialize risk engine
let params = RiskParams {
    warmup_period_slots: 100,        // ~50 seconds at 400ms/slot
    maintenance_margin_bps: 500,     // 5%
    initial_margin_bps: 1000,        // 10%
    trading_fee_bps: 10,             // 0.1%
    liquidation_fee_bps: 50,         // 0.5%
    insurance_fee_share_bps: 5000,   // 50% to insurance
};

let mut engine = RiskEngine::new(params);

// Add users and LPs (with account creation fees)
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

// Alice can withdraw (warmed-up PNL is automatically converted to principal)
let withdrawable = engine.withdrawable_pnl(&engine.users[alice]);
engine.withdraw(alice, withdrawable)?;

// Liquidations (permissionless)
let keeper = engine.add_user(10_000)?;
engine.liquidate_user(alice, keeper, oracle_price)?;
```

## Design Rationale

### Why PNL Warmup?

Traditional perp DEXs face an impossible tradeoff:
1. **Fast withdrawals** → vulnerable to oracle attacks
2. **Delayed withdrawals** → poor UX

PNL warmup solves this by:
- Allowing instant withdrawal of **principal** (users can always exit with their deposits)
- Requiring time for **PNL** to vest (prevents instant extraction of manipulated profits)
- Using ADL to haircut young PNL during crises (aligns incentives)

### Why Principal is Sacred?

By guaranteeing that `principal` is never touched by socialization:
1. Users can withdraw their deposits (subject to margin requirements if they have open positions)
2. The worst case is losing unrealized PNL, not principal deposits
3. Creates clear mental model: "principal is protected from ADL, profits take time to vest"

**Important:** Users with open positions must maintain margin requirements. If you have a position and negative PNL, you may not be able to withdraw all principal until you close the position.

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

✅ **User principal is protected from ADL** and withdrawable (subject to margin requirements)
✅ **PNL requires time T to become withdrawable**
✅ **ADL haircuts young PNL first, protecting principal**
✅ **Conservation of funds across all operations**
✅ **User isolation - one user can't affect another**
✅ **No oracle manipulation can extract funds faster than time T**

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
| ADL abuse | Principal never touched |
| Matching engine exploit | User signature + solvency checks |
| Liquidation griefing | Permissionless + keeper incentives |
| Insurance drain | Fees replenish fund |

## Dependencies

This implementation uses **no_std** and minimal dependencies:

- `pinocchio`: Solana program framework (no_std compatible)
- `pinocchio-log`: Logging for Solana programs

For testing:
- `proptest`: Property-based testing (fuzzing)
- `kani`: Formal verification

## Building and Testing

This is a library module designed to be used as a dependency in Solana programs.

```bash
# Build the library
cargo build

# Run unit tests
cargo test --test unit_tests

# Run fuzzing tests (property-based testing)
cargo test --features fuzz --test fuzzing

# Run formal verification (requires Kani)
cargo kani --tests --harness i1_adl_never_reduces_principal
cargo kani --tests --harness i5_warmup_monotonicity
# ... see tests/kani.rs for all proofs
```

### Using as a Dependency

Add to your Solana program's `Cargo.toml`:

```toml
[dependencies]
percolator = { git = "https://github.com/aeyakovenko/percolator", branch = "clean" }
```

Then import and use the risk engine:

```rust
use percolator::{RiskEngine, RiskParams, UserAccount};

// Initialize risk engine with your parameters
let params = RiskParams {
    warmup_period_slots: 100,
    maintenance_margin_bps: 500,
    // ... other params
};

let mut engine = RiskEngine::new(params);
```

## Project Structure

```
percolator/
├── src/
│   └── percolator.rs          # Single implementation file
├── tests/
│   ├── unit_tests.rs          # Fast unit tests
│   ├── fuzzing.rs             # Property-based tests (--features fuzz)
│   └── kani.rs                # Formal verification proofs
├── Cargo.toml                 # Minimal dependencies
└── README.md                  # This file
```

## Contributing

This is a formally verified safety-critical module. All changes must:

1. ✅ Pass all unit tests
2. ✅ Pass all fuzzing tests
3. ✅ Pass all Kani proofs
4. ✅ Maintain `no_std` compatibility
5. ✅ Preserve all invariants (I1-I8)

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
