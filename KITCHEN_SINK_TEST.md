# Kitchen Sink End-to-End Test (KS-00)

## Overview

The Kitchen Sink test is a comprehensive multi-phase integration test designed to exercise the **entire Percolator protocol** under realistic conditions. It simulates a complete market lifecycle from bootstrap through crisis scenarios.

**Status**: Phases 1-3 complete (trading + funding implemented), Phases 4-5 pending feature implementation

## Test Philosophy

**"Kitchen Sink"** = Everything but the kitchen sink:
- Multi-market setup (heterogeneous matchers)
- Multiple actor types (LPs, takers, keepers)
- Full lifecycle coverage (bootstrap → trading → crisis → recovery)
- Cross-phase invariants verified at every boundary
- Deterministic execution (seed=42 for reproducibility)

## Test Phases

### Phase 1 (KS-01): Bootstrap Books & Reserves ✅ IMPLEMENTED

**Goal**: Establish multi-market infrastructure with funded actors

**Actions**:
1. Create SOL-PERP slab matcher
2. Create BTC-PERP slab matcher
3. Initialize portfolios for 4 actors:
   - **Alice**: Cash LP (800 SOL)
   - **Bob**: Margin LP (400 SOL)
   - **Dave**: Taker/buyer (200 SOL)
   - **Erin**: Taker/seller (200 SOL)
4. Deposit initial collateral

**Assertions**:
- ✓ All actors have non-negative principals
- ✓ Matchers registered in exchange registry
- ✓ Portfolio accounts created successfully

**Invariants Checked**:
- Non-negative balances: `principal_i ≥ 0 ∀ i`

---

### Phase 2 (KS-02): Taker Bursts + Fills ✅ IMPLEMENTED

**Goal**: Generate fills, fees, and PnL through taker trades

**Actions**:
1. Alice places limit orders on SOL-PERP (creates spread)
2. Bob places limit orders on BTC-PERP
3. Dave executes market buy on SOL-PERP → crosses Alice's ask
4. Erin executes market sell on SOL-PERP → crosses Alice's bid
5. Verify fills, maker rebates (if implemented), taker fees

**Implementation Details**:
- Uses `place_maker_order_as()` helper for maker liquidity
- Uses `place_taker_order_as()` helper for taker crosses
- Multi-actor execution (Alice, Bob, Dave, Erin)
- Actual on-chain order placement and fills

**Assertions**:
- Partial fills don't underflow `qty_remaining`
- Taker margin checks enforced (no negative free collateral)
- Maker rebates applied correctly
- Conservation: `taker_pays_fee + maker_earns_rebate = router_collects_fee`

**Invariants Checked**:
- Conservation: `vault == Σ principals + Σ PnL - fees_collected`
- Seqno monotonicity (TOCTOU protection)

---

### Phase 3 (KS-03): Funding Accrual ✅ IMPLEMENTED

**Goal**: Accrue funding rates on open positions

**Actions**:
1. Wait 65 seconds for funding eligibility (dt >= 60s requirement)
2. Set oracle price divergence on SOL-PERP:
   - Oracle: 101.0, Mark: 100.0
   - Mark < Oracle → longs pay funding to shorts
3. Update BTC-PERP funding (neutral oracle = mark)
4. Verify funding conservation (zero-sum by design)

**Implementation Details**:
- Uses `update_funding_as()` helper for UpdateFunding instruction
- LP owner signs as authority
- Funding sensitivity: 8 bps per hour (800 at 1e6 scale)
- Cumulative funding index updated on-chain
- Multi-market funding coordination

**Assertions**:
- Funding transfers are zero-sum: `Σ funding_paid == Σ funding_received`
- No funding on unfilled orders (only on positions)
- Maker positions updated before maintenance checks

**Invariants Checked**:
- Funding conservation: `Σ funding_transfers == 0` (fees aside)

---

### Phase 4 (KS-04): Oracle Shock + Liquidations ⚠️ PENDING

**Goal**: Trigger margin calls and test liquidation mechanics

**Planned Actions**:
1. Simulate adverse price movement
   - Example: SOL oracle drops from $100 → $84 (-16%)
2. Update mark price to reflect oracle
3. Keeper sweeps for underwater accounts
4. Cancel resting orders for makers under MM
5. Liquidate underwater positions
6. Verify liquidation fees flow to insurance fund

**Pending Implementation**:
- Oracle price update mechanism
- Keeper risk touch operations
- Maker order cancellation during liquidation
- Liquidation fee routing

**Assertions** (when implemented):
- Cancel-then-liquidate ordering respected
- Reservations freed before position liquidation
- Seat invariant holds post-liquidation
- No position grows during liquidation (monotonicity)

**Invariants Checked**:
- Non-negative free collateral post-liquidation
- Liquidation monotonicity: `exposure_post ≤ exposure_pre`

---

### Phase 5 (KS-05): Insurance Drawdown + Loss Socialization ⚠️ PENDING

**Goal**: Stress insurance fund and verify loss waterfall

**Planned Actions**:
1. Push further adverse move to create bad debt
   - Example: SOL drops to $70 → some liquidations create deficit
2. Batch liquidations consume insurance fund
3. Trigger loss socialization when insurance exhausted
4. Verify loss absorption ordering:
   - Insurance consumed to 0 first
   - Haircut positive realized PnL (respect warmup buckets if applicable)
   - System-wide equity haircut if still negative

**Pending Implementation**:
- Bad debt creation scenarios
- Loss socialization instruction
- Warmup bucket integration (if applicable)

**Assertions** (when implemented):
- Insurance drawn before any user haircut
- Haircut ratio calculation: `γ = (bad_debt - insurance) / total_equity`
- User impact: `user_final = user_initial × (1 - γ)`
- Mathematical verification: `insurance_paid + haircut_total == bad_debt`

**Invariants Checked**:
- Loss waterfall ordering: Insurance → Warmup → Equity
- Conservation: `Σ user_equity + insurance == pre_crisis_total - absorbed_loss`

---

## Cross-Phase Invariants

These invariants are checked at **every phase boundary**:

### 1. Conservation (Router Global)
```
vault_lamports == Σ principal_i + Σ max(vested_pnl_i, 0) - fees_outstanding ± ε
```

### 2. Non-Negative Balances
```
free_collateral_i ≥ 0 ∧ reserved_collateral_i ≥ 0  ∀ i
```

### 3. Funding Conservation
```
Σ funding_transfers == 0  (excluding fees)
```

### 4. Order Book Integrity
```
0 ≤ qty_remaining ≤ qty_init  ∧  seqno strictly increases
```

### 5. Liquidation Monotonicity
```
After cancel_all_resting(user) ⇒ im_locked_total == 0
```

### 6. Seat Haircuts (if implemented)
```
exposure_post ≤ seat_cap × (0.9 × equity_after - IM_locked)
```

### 7. Socialization Ordering
```
Insurance consumed → Positive PnL haircut → Global γ (no out-of-order)
```

---

## Running the Test

### Via CLI Test Suite
```bash
# Run all crisis tests (includes Kitchen Sink as Test 4)
./target/release/percolator --network localnet test --crisis
```

### Standalone Script
```bash
# Dedicated runner with detailed output
./test_kitchen_sink.sh
```

### Expected Output (Current Skeleton)
```
═══════════════════════════════════════════════════════════════
  Kitchen Sink E2E Test (KS-00)
═══════════════════════════════════════════════════════════════

Multi-phase comprehensive test covering:
  • Multi-market setup (SOL-PERP, BTC-PERP)
  • Multiple actors (Alice, Bob, Dave, Erin, Keeper)
  • Order book liquidity and taker trades
  • Funding rate accrual
  • Oracle shocks and liquidations
  • Insurance fund stress
  • Cross-phase invariants

═══ Setup: Actors & Initial State ═══
  Alice funded with 10 SOL
  Bob funded with 10 SOL
  Dave funded with 10 SOL
  Erin funded with 10 SOL
  ✓ All actors funded

═══ Phase 1 (KS-01): Bootstrap Books & Reserves ═══
  Creating SOL-PERP matcher...
  ✓ SOL-PERP created: <pubkey>
  Creating BTC-PERP matcher...
  ✓ BTC-PERP created: <pubkey>
  ✓ Alice portfolio initialized
  ✓ Bob portfolio initialized
  ✓ Dave portfolio initialized
  ✓ Erin portfolio initialized
  ✓ Alice deposited 800 SOL
  ✓ Bob deposited 400 SOL
  ✓ Dave deposited 200 SOL
  ✓ Erin deposited 200 SOL

  Phase 1 Complete: Multi-market bootstrapped
  - 2 markets: SOL-PERP, BTC-PERP
  - 4 actors with portfolios and collateral

  [INVARIANT] Checking non-negative balances...
  ✓ All actors have positive principals

═══ Phase 2 (KS-02): Taker Bursts + Fills ═══
  ⚠ Phase 2 implementation pending (requires liquidity placement)

  [INVARIANT] Checking conservation...
  ⚠ Conservation check skipped (needs vault query)

... (similar for Phases 3-5)

═══════════════════════════════════════════════════════════════
  Kitchen Sink Test Complete
═══════════════════════════════════════════════════════════════

Phases Completed:
  ✓ Phase 1: Multi-market bootstrap
  ⚠ Phase 2: Taker trades (partial)
  ⚠ Phase 3: Funding (pending)
  ⚠ Phase 4: Liquidations (pending)
  ⚠ Phase 5: Loss socialization (pending)
```

---

## Implementation Roadmap

### Completed (Phases 1-3)
- ✅ Multi-market bootstrap
- ✅ Actor initialization
- ✅ Collateral deposits
- ✅ Non-negative balance invariant
- ✅ Liquidity placement (maker orders)
- ✅ Taker crosses and fills
- ✅ Multi-actor trading coordination
- ✅ Funding rate mechanism (UpdateFunding instruction)
- ✅ Funding conservation verification (zero-sum)
- ✅ Oracle price deviation handling

### Medium-term (Next Steps)
- ⏳ Fee accounting verification (query receipts)
- ⏳ PnL tracking and validation
- ⏳ Funding impact on user positions

### Long-term (Phases 4-5)
- ⏳ Oracle price updates
- ⏳ Keeper operations (risk touch, liquidations)
- ⏳ Bad debt scenarios
- ⏳ Loss socialization
- ⏳ Full invariant suite

---

## Fuzz Extension (Future)

The kitchen sink test is designed to support **fuzz testing** with deterministic seeds:

```rust
// Future enhancement
let seed = 42; // Deterministic for reproducibility
let mut rng = ChaCha8Rng::seed_from_u64(seed);

// Randomize within bounds:
// - Oracle jitters (±2% from base)
// - Bursty taker timing
// - Partial order fills
// - Cancel races
```

**Fuzz Guards** (run every 50 ops):
- Run `keeper_reconcile_all`
- Assert all 7 cross-phase invariants
- Verify no account corruption

---

## Integration with Other Tests

The Kitchen Sink test complements existing tests:

| Test | Focus | Kitchen Sink Adds |
|------|-------|-------------------|
| `test_insurance_fund_usage` | Single insurance operation | Multi-phase stress |
| `test_loss_socialization_integration` | Insurance exhaustion (1 scenario) | Full lifecycle + cascades |
| `test_cascade_liquidations` | Liquidation mechanics | Multi-market + funding |
| `test_amm_lp_liquidation` | AMM LP stress | Slab + AMM heterogeneity |

**Kitchen Sink unique value**:
- End-to-end lifecycle (bootstrap → crisis → recovery)
- Multi-market interactions
- Actor isolation verification
- Cross-phase invariants at every boundary

---

## Code Location

**Test Implementation**: `cli/src/tests.rs:1965-2225`
**Test Runner**: Integrated into `run_crisis_tests()` (Test 4)
**Standalone Script**: `test_kitchen_sink.sh`

**Key Functions**:
- `test_kitchen_sink_e2e(config: &NetworkConfig)` - Main test entry point
- Phase implementations embedded inline with TODO markers

---

## Contributing Phases

To expand the Kitchen Sink test:

1. **Implement missing feature** (e.g., liquidity placement)
2. **Add phase logic** in corresponding section:
   ```rust
   // PHASE 2 (KS-02): Taker bursts + fills
   // Replace TODO with actual implementation
   alice_place_limit_order(...)?;
   dave_market_buy(...)?;
   verify_fill_receipt(...)?;
   ```
3. **Add invariant checks** at phase boundary:
   ```rust
   // INVARIANT CHECK: Conservation after trades
   let vault_balance = query_vault_balance()?;
   let total_principals = query_all_principals()?;
   let total_pnl = query_all_pnl()?;
   assert_approx_eq!(vault_balance, total_principals + total_pnl - fees);
   ```
4. **Update test script** output expectations
5. **Update this document** with actual results

---

## References

- Original specification: User's "kitchen sink" request (comprehensive E2E)
- Crisis module: `crates/model_safety/src/crisis/`
- Insurance tests: `cli/src/tests.rs:1254-1679`
- Loss socialization: `test_loss_socialization_integration()`

---

**Last Updated**: November 1, 2025
**Status**: Phases 1-3 complete, Phases 4-5 pending feature implementations
