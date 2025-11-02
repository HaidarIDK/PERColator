# Test Implementation Plan

## Executive Summary

This document provides a detailed plan to implement all stub/placeholder tests identified in the audit, converting them from false positives (returning `Ok(())`) to actual working tests that verify on-chain behavior.

## Current Status

### Already Fixed ✅
- All stub tests now explicitly fail with `anyhow::bail()` instead of silently passing
- Placeholder CLI commands now fail instead of returning success
- ExecuteCrossSlab CPI bug fixed (InvalidSeeds error resolved)

### Tests Requiring Implementation

1. **test_offsetting_positions** - Cross-margin netting verification
2. **test_cross_margining_benefit** - Margin efficiency measurement
3. **test_cascade_liquidations** - Sequential liquidation handling
4. **test_amm_lp_insolvency** - AMM LP liquidation (BLOCKED - requires liquidity module)
5. **test_slab_lp_insolvency** - Slab LP liquidation (BLOCKED - requires liquidity module)
6. **test_lp_trader_isolation** - LP/Trader isolation invariant

## Test Infrastructure Pattern

Based on analysis of working tests (insurance fund usage, Kitchen Sink E2E), all tests follow this pattern:

```rust
async fn test_name(config: &NetworkConfig) -> Result<()> {
    // 1. Create RPC client
    let rpc_client = client::create_rpc_client(config);
    let payer = &config.keypair;

    // 2. Create actor keypairs and fund them
    let actor = Keypair::new();
    // ... transfer SOL for fees

    // 3. Initialize exchange infrastructure (registry, slabs, oracles)
    // ... create_slab(), initialize_oracle()

    // 4. Initialize portfolios for actors
    // ... margin::initialize_portfolio()

    // 5. Deposit collateral
    // ... margin::deposit()

    // 6. Execute test-specific operations
    // ... place orders, execute trades, etc.

    // 7. Verify expected outcomes
    // ... query accounts, check balances, assert invariants

    Ok(())
}
```

## Detailed Implementation Plans

### 1. test_offsetting_positions

**Goal:** Verify that opening long and short positions on correlated instruments reduces net exposure and margin requirements.

**Implementation Steps:**

```rust
async fn test_offsetting_positions(config: &NetworkConfig) -> Result<()> {
    // SETUP
    // - Create 2 slabs: SOL-PERP (long) and ETH-PERP (short) with similar prices
    // - Create actor keypair (Alice)
    // - Fund Alice with SOL, initialize portfolio, deposit collateral

    // PHASE 1: Open long position on SOL-PERP
    // - Create SOL-PERP slab with oracle at $150
    // - Place maker order: sell 10 SOL @ $150
    // - Execute taker order as Alice: buy 10 SOL @ $150
    // - Query Alice's portfolio: verify long_exposure = +10 SOL
    // - Calculate margin_required_long = position_value * initial_margin_rate

    // PHASE 2: Open offsetting short position on ETH-PERP
    // - Create ETH-PERP slab with oracle at $3000 (20x SOL price for correlation)
    // - Place maker order: buy 0.5 ETH @ $3000
    // - Execute taker order as Alice: sell 0.5 ETH @ $3000
    // - Query Alice's portfolio: verify short_exposure = -0.5 ETH

    // PHASE 3: Verify netting
    // - Portfolio should have TWO buckets (SOL-PERP and ETH-PERP)
    // - Calculate net_exposure_usd = (long_sol * sol_price) + (short_eth * eth_price)
    //   Expected: (10 * $150) + (-0.5 * $3000) = $1500 - $1500 = ~$0
    // - Margin requirement should be LOWER than sum of individual requirements
    //   margin_required_combined < margin_required_long + margin_required_short

    // ASSERTIONS
    // assert!(net_exposure_usd.abs() < 100.0, "Net exposure should be near zero");
    // assert!(margin_combined < margin_individual_sum * 0.9, "Margin benefit > 10%");

    Ok(())
}
```

**Dependencies:**
- Working ExecuteCrossSlab (✅ FIXED)
- Oracle price updates
- Portfolio query/deserialization

**Estimated Complexity:** Medium (3-4 hours)

---

### 2. test_cross_margining_benefit

**Goal:** Measure capital efficiency improvement from portfolio margining with correlated positions.

**Implementation Steps:**

```rust
async fn test_cross_margining_benefit(config: &NetworkConfig) -> Result<()> {
    // SETUP
    // - Create 3 slabs: BTC-PERP, ETH-PERP, SOL-PERP (correlated crypto)
    // - Create actor Alice with substantial collateral

    // BASELINE: Calculate isolated margin requirements
    // Position 1: Long 1 BTC @ $40,000 = $40,000 notional
    //   isolated_margin_1 = $40,000 * 5% = $2,000
    // Position 2: Long 10 ETH @ $3,000 = $30,000 notional
    //   isolated_margin_2 = $30,000 * 5% = $1,500
    // Position 3: Long 100 SOL @ $150 = $15,000 notional
    //   isolated_margin_3 = $15,000 * 5% = $750
    // Total isolated margin = $4,250

    // TEST: Open all positions in single portfolio
    // - Execute trades to establish all 3 long positions
    // - Query portfolio margin requirement
    // - Should be LESS than $4,250 due to correlation

    // MEASURE EFFICIENCY
    // capital_efficiency = isolated_margin_sum / portfolio_margin
    // Expected: > 1.0 (e.g., 1.15 = 15% improvement)

    // VERIFY WITH PRICE SHOCK
    // - Update all oracles with +10% price increase
    // - Portfolio PnL should increase uniformly (correlation benefit)
    // - Margin requirement should remain efficient

    // ASSERTIONS
    // assert!(portfolio_margin < isolated_margin_sum);
    // assert!(capital_efficiency >= 1.10, "At least 10% margin efficiency");

    Ok(())
}
```

**Dependencies:**
- Working ExecuteCrossSlab
- Oracle updates
- Portfolio margin calculation access

**Estimated Complexity:** Medium-High (4-5 hours)

---

### 3. test_cascade_liquidations

**Goal:** Verify that multiple underwater accounts are liquidated sequentially without system failure.

**Implementation Steps:**

```rust
async fn test_cascade_liquidations(config: &NetworkConfig) -> Result<()> {
    // SETUP
    // - Create 4 undercollateralized actors: Alice, Bob, Carol, Dave
    // - Each has leveraged long position on SOL-PERP
    // - Collateral ratios: Alice 110%, Bob 115%, Carol 120%, Dave 125%
    //   (All above maintenance margin of 102.5% but vulnerable to price drop)

    // PHASE 1: Establish positions
    // For each actor:
    //   - Deposit collateral (barely sufficient)
    //   - Execute long SOL trade at $150
    //   - Verify account is solvent but highly leveraged

    // PHASE 2: Oracle price shock
    // - Update SOL oracle from $150 to $120 (20% drop)
    // - Calculate new health ratios:
    //   Alice: 110% → 88% (underwater)
    //   Bob: 115% → 92% (underwater)
    //   Carol: 120% → 96% (underwater)
    //   Dave: 125% → 100% (still solvent)

    // PHASE 3: Sequential liquidations
    // Liquidate Alice:
    //   - Call liquidate_user instruction
    //   - Verify Alice's position closed
    //   - Verify bad debt handled (insurance fund or socialization)
    //   - Check Bob, Carol, Dave unaffected

    // Liquidate Bob:
    //   - Same process, verify Carol and Dave still active

    // Liquidate Carol:
    //   - Verify Dave still trading normally

    // PHASE 4: Verify isolation
    // - Dave should still be solvent and able to trade
    // - Insurance fund should have absorbed losses OR
    // - Haircuts applied to other users (test both paths)

    // ASSERTIONS
    // assert!(alice_liquidated && bob_liquidated && carol_liquidated);
    // assert!(!dave_liquidated, "Dave should remain solvent");
    // assert!(insurance_fund_balance_decreased || haircuts_applied);

    Ok(())
}
```

**Dependencies:**
- Working liquidate_user instruction
- Oracle price updates
- Insurance fund mechanics
- Portfolio health calculations

**Estimated Complexity:** High (6-8 hours)
**Risk:** May expose bugs in liquidation cascades

---

### 4. test_amm_lp_insolvency

**Status:** ⚠️ **BLOCKED**

**Blocker:** Requires AMM liquidity module functions that don't exist:
- `mint_lp_shares` instruction (not implemented)
- `add_liquidity` CLI function (placeholder)
- LP share accounting in portfolios

**Decision Options:**

**Option A: Keep Failing Test (Current)**
- Test explicitly fails with message explaining blocker
- Documents missing functionality
- Prevents false positive

**Option B: Skip Test**
```rust
#[ignore = "Blocked: AMM liquidity module not implemented"]
async fn test_amm_lp_insolvency(config: &NetworkConfig) -> Result<()> {
    // ...
}
```

**Option C: Implement Missing Instructions** (Out of scope for this plan)

**Recommendation:** Keep current explicit failure. Add detailed comment explaining:
1. What instruction is missing
2. Why it's needed
3. Link to design doc/spec

---

### 5. test_slab_lp_insolvency

**Status:** ⚠️ **BLOCKED**

**Blocker:** Similar to AMM LP insolvency:
- `place_lp_order` instruction doesn't exist
- LP order placement via CLI not implemented
- Slab LP bucket mechanics may be incomplete

**Recommendation:** Keep current explicit failure with detailed blocker documentation.

**Alternative Approach:** Test using the existing `burn_lp_shares` and `cancel_lp_orders` instructions:
```rust
async fn test_slab_lp_insolvency(config: &NetworkConfig) -> Result<()> {
    // NOTE: This tests LP exposure REDUCTION only, not creation
    // Real test requires place_lp_order instruction (not implemented)

    // SETUP (manually inject LP exposure into test account)
    // ... create portfolio with fake LP bucket data

    // TEST: Verify cancel_lp_orders reduces exposure
    // ... but this doesn't test insolvency scenario

    anyhow::bail!("Test not implemented: requires place_lp_order instruction")
}
```

---

### 6. test_lp_trader_isolation

**Goal:** Verify that LP losses NEVER affect trader principal positions.

**Implementation Steps:**

```rust
async fn test_lp_trader_isolation(config: &NetworkConfig) -> Result<()> {
    // SETUP
    // - Create two actors: Alice (LP+Trader), Bob (Pure Trader)
    // - Create SOL-PERP slab

    // PHASE 1: Establish positions
    // Alice:
    //   - Deposit 1 SOL collateral
    //   - Open 5 SOL long position (principal/trader bucket)
    //   - Provide LP liquidity (LP bucket) - IF POSSIBLE
    //     Otherwise: Manually inject LP bucket data for testing

    // Bob:
    //   - Deposit 0.5 SOL collateral
    //   - Open 2 SOL long position (principal bucket only)

    // PHASE 2: Create LP loss scenario
    // - Oracle price shock: SOL $150 → $100
    // - Alice's LP bucket becomes underwater (simulated impermanent loss)
    // - Alice's trader position should be INDEPENDENT

    // PHASE 3: Verify isolation
    // Query Alice's portfolio:
    //   lp_buckets[0].exposure = underwater
    //   principal_position = still valid
    //   collateral_balance = should NOT be touched by LP loss

    // Query Bob's portfolio:
    //   principal_position = unchanged
    //   collateral_balance = unchanged
    //   No LP buckets

    // PHASE 4: Test operations
    // - Alice should still be able to close trader position
    // - Bob should trade normally
    // - LP bucket liquidation should NOT touch principal

    // ASSERTIONS
    // assert_eq!(alice_principal_before, alice_principal_after);
    // assert_eq!(bob_portfolio_before, bob_portfolio_after);
    // assert!(alice_lp_bucket_liquidated);
    // assert!(!alice_principal_liquidated, "CRITICAL: Principal must be isolated");

    Ok(())
}
```

**Dependencies:**
- LP bucket mechanics (may need manual injection for testing)
- Portfolio query/deserialization
- Understanding of LP vs Principal accounting

**Estimated Complexity:** Medium-High (5-6 hours)
**Note:** May need to artificially inject LP bucket data if `mint_lp_shares` unavailable

---

## Implementation Priority

### Phase 1: Infrastructure Verification (Week 1)
1. ✅ Verify Kitchen Sink test passes (already fixed ExecuteCrossSlab)
2. Extract reusable test helpers from Kitchen Sink
3. Document Portfolio query/deserialization patterns

### Phase 2: Core Trading Tests (Week 2)
1. **test_offsetting_positions** (Medium complexity, high value)
2. **test_cross_margining_benefit** (Medium-high complexity, demonstrates core feature)

### Phase 3: Risk Management Tests (Week 3)
1. **test_cascade_liquidations** (High complexity, critical for safety)
2. **test_lp_trader_isolation** (Medium-high complexity, critical invariant)

### Phase 4: LP Tests (Blocked - Document Only)
1. **test_amm_lp_insolvency** - Keep explicit failure, add blocker docs
2. **test_slab_lp_insolvency** - Keep explicit failure, add blocker docs

## Success Criteria

### Test Quality Standards
Each implemented test must:
1. ✅ Execute actual on-chain transactions (no mocks)
2. ✅ Query and verify on-chain state
3. ✅ Assert specific numerical values (not just "no error")
4. ✅ Test both success and failure paths where applicable
5. ✅ Include cleanup/teardown if needed
6. ✅ Document expected behavior in comments

### Coverage Goals
- All non-blocked tests passing ✅
- False positive rate = 0% (no stub tests)
- Integration test coverage for core features:
  - Cross-margin netting ✅
  - Capital efficiency ✅
  - Liquidation cascades ✅
  - LP/Trader isolation ✅

## Risk Analysis

### High Risk Areas
1. **Cascade Liquidations** - May expose unforeseen bugs in sequential liquidation handling
2. **Oracle Integration** - Price updates may have race conditions
3. **Portfolio Deserialization** - Unsafe pointer casts may cause issues

### Mitigation Strategies
1. Start with simpler tests (offsetting positions)
2. Add extensive logging in complex tests
3. Use test validator with fresh state for each test
4. Document any workarounds or limitations

## Timeline Estimate

| Phase | Tasks | Duration | Dependencies |
|-------|-------|----------|--------------|
| Infrastructure | Kitchen Sink verification, helpers | 2-3 days | None |
| Core Trading | 2 tests | 4-5 days | Infrastructure |
| Risk Management | 2 tests | 6-7 days | Core Trading |
| Documentation | LP test blockers | 1 day | Risk Management |
| **Total** | | **13-16 days** | |

## Open Questions

1. **Portfolio Deserialization:** Current tests use unsafe pointer casts. Is there a safer deserialization method?
2. **Oracle Updates:** What's the proper way to update oracle prices in tests? Direct instruction or via CLI?
3. **LP Bucket Injection:** For isolation test, can we manually create LP bucket state for testing?
4. **Insurance Fund:** How do we verify insurance fund mechanics without actual LP insolvencies?

## Next Steps

1. Update todo list with Phase 1 tasks
2. Verify Kitchen Sink test passes with current fixes
3. Extract reusable test helpers into separate module
4. Begin implementation of test_offsetting_positions
