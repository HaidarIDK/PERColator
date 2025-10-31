# LP Liquidation Architecture

## Overview

This document describes the LP (Liquidity Provider) liquidation architecture in the Percolator DEX. LP liquidation is the final safety mechanism that allows the protocol to close underwater LP positions to restore portfolio health when principal liquidation is insufficient.

## Background

In a margin DEX, users can provide liquidity to venues (orderbooks and AMMs) using cross-margined capital. When a portfolio becomes underwater (equity < maintenance margin), the liquidation engine attempts to restore health by:

1. **Principal liquidation** - Closing directional positions first
2. **Slab LP liquidation** - Liquidating orderbook LP positions (if principal insufficient)
3. **AMM LP liquidation** - Liquidating AMM LP positions as last resort (if still underwater)

This three-tier priority ensures that:
- Easier-to-value positions are liquidated first
- LP positions (harder to value) are only touched when necessary
- Staleness guards protect against mispriced AMM liquidations

## Architecture Goals

1. **Safety First** - Use KANI formally verified math for margin calculations
2. **Fair Pricing** - Staleness guards prevent liquidation on stale prices
3. **Capital Efficiency** - Only liquidate what's needed to restore health
4. **Integration** - Seamlessly integrate with existing liquidation flow
5. **Simplicity** - Reuse existing instructions (no new discriminators)

## Three-Tier Liquidation Priority

### Priority 1: Principal Positions

**What**: Directional exposures on instruments (long/short positions)

**Why First**:
- Easiest to value (oracle prices)
- Most liquid
- Standard liquidation mechanism

**Implemented In**: Existing `process_liquidate_user()` flow (steps 1-6)

### Priority 2: Slab LP Positions

**What**: Orderbook LP positions with reserved base/quote collateral

**Why Second**:
- Easier to value than AMM LP (no share price calculation)
- Reserves are locked but not complex
- No staleness concerns (reserves are deterministic)

**Implementation**: `liquidate_slab_lp_buckets()` in `programs/router/src/instructions/liquidate_user.rs:58-156`

**Process**:
1. Iterate through active Slab LP buckets
2. Calculate freed collateral: `reserved_base + reserved_quote`
3. Calculate remaining ratio after freeing collateral
4. Apply KANI verified proportional margin reduction (LP8-LP10)
5. Update portfolio equity with freed collateral
6. Mark bucket inactive
7. Continue until deficit eliminated or all Slab LP liquidated

### Priority 3: AMM LP Positions

**What**: AMM LP shares with cached share prices

**Why Last**:
- Hardest to value (requires recent share price)
- Share price can become stale
- Last resort due to complexity

**Implementation**: `liquidate_amm_lp_buckets()` in `programs/router/src/instructions/liquidate_user.rs:158-251`

**Process**:
1. Iterate through active AMM LP buckets
2. **SAFETY TRIPWIRE**: Check price staleness
   - Calculate `price_age = current_ts - last_update_ts`
   - Skip if `price_age > max_staleness_seconds`
   - Log warning for monitoring
3. Calculate redemption value using KANI verified function (LP6)
4. Apply proportional margin reduction (LP8-LP10) with 0% remaining ratio
5. Burn all LP shares
6. Update portfolio equity
7. Mark bucket inactive
8. Continue until deficit eliminated or all AMM LP liquidated

## KANI Verified Functions

The LP liquidation code uses formally verified math functions from `programs/router/src/state/model_bridge.rs`:

### LP6: `calculate_redemption_value_verified`

```rust
pub fn calculate_redemption_value_verified(
    shares_to_burn: u128,
    current_share_price: u64,
) -> Result<i128, &'static str>
```

**Purpose**: Calculate the redemption value of AMM LP shares

**Verification**: Proves no overflow when calculating `(shares * price) / SHARE_PRICE_SCALE`

### LP8-LP10: `proportional_margin_reduction_verified`

```rust
pub fn proportional_margin_reduction_verified(
    initial_margin: u128,
    remaining_ratio: u128,
) -> Result<u128, &'static str>
```

**Purpose**: Scale margin requirements proportionally when liquidating LP positions

**Verification**: Proves:
- No overflow in `(margin * ratio) / RATIO_SCALE`
- Monotonicity: `remaining_ratio` increases → margin requirement increases
- Boundary conditions: 0% ratio → 0 margin, 100% ratio → full margin

**Usage in Code**:
```rust
// For partial Slab LP liquidation
let remaining_ratio = /* calculate based on collateral remaining */;
bucket.im = convert_verification_error(
    proportional_margin_reduction_verified(bucket.im, remaining_ratio)
)?;
bucket.mm = convert_verification_error(
    proportional_margin_reduction_verified(bucket.mm, remaining_ratio)
)?;

// For full AMM LP liquidation
let ratio_remaining = 0u128; // Burning all shares
bucket.im = convert_verification_error(
    proportional_margin_reduction_verified(bucket.im, ratio_remaining)
)?;
```

### Error Conversion

Verified functions return `Result<T, &'static str>` but production code needs `Result<T, PercolatorError>`:

```rust
#[inline]
fn convert_verification_error<T>(result: Result<T, &'static str>) -> Result<T, PercolatorError> {
    result.map_err(|_e| PercolatorError::Overflow)
}
```

This wrapper preserves the verification guarantees while integrating with the error system.

## Integration with Existing Liquidation Flow

The LP liquidation logic integrates at **Step 6.5** in `process_liquidate_user()`:

```
Step 1-5: Setup, fee calculation, price validation
Step 6:   Principal liquidation (existing)
Step 6.5: LP liquidation (NEW)
  6.5.1: Check remaining deficit
  6.5.2: Liquidate Slab LP if needed
  6.5.3: Recalculate margins
  6.5.4: Check deficit again
  6.5.5: Liquidate AMM LP if still needed
  6.5.6: Recalculate margins
  6.5.7: Final health check
Step 7:   Bad debt handling (existing)
Step 8:   Profit distribution (existing)
```

**Key Design Decisions**:

1. **No New Instructions** - Uses existing `liquidate_user` instruction (disc 0)
2. **Incremental Checks** - Recalculates health after each liquidation stage
3. **Early Exit** - Stops as soon as portfolio is healthy
4. **Bad Debt Integration** - LP proceeds flow into existing insurance fund logic

## Staleness Guards

### Why Staleness Matters

AMM share prices are updated periodically by the AMM program. If prices become stale:
- Liquidations may occur at incorrect valuations
- Liquidators may extract unfair value
- Users may be liquidated prematurely

### Implementation

```rust
// In liquidate_amm_lp_buckets()
let price_age = current_ts.saturating_sub(amm_lp.last_update_ts);
if price_age > max_staleness_seconds {
    msg!("Warning: AMM price stale, skipping bucket");
    continue; // Skip this bucket
}
```

**Parameters**:
- `current_ts`: Clock timestamp from Solana
- `last_update_ts`: Timestamp from `AmmLpPosition.last_update_ts`
- `max_staleness_seconds`: From `SlabRegistry.max_oracle_staleness_secs`

**Behavior**:
- Fresh prices → AMM LP can be liquidated
- Stale prices → AMM LP bucket skipped, portfolio may enter bad debt
- Logged for monitoring and keeper action

## Margin Recalculation

After each liquidation stage, portfolio margins are recalculated:

```rust
portfolio.mm = portfolio.calculate_total_mm();
portfolio.im = portfolio.calculate_total_im();
```

**Why Important**:
- LP positions contribute to MM/IM via bucket margins
- Liquidating LP reduces total margin requirements
- Must recalculate to determine if more liquidation needed

**Note**: `calculate_total_mm/im()` return `u128` directly (not `Result<u128, _>`)

## Bad Debt Handling

If LP liquidation is insufficient to restore health:

```rust
let final_deficit = calculate_remaining_deficit(portfolio, registry)?;
if final_deficit > 0 {
    msg!("LP Liquidation: Warning - still underwater");
    // Falls through to Step 7: Bad debt handling
}
```

The existing bad debt logic (Step 7 in `process_liquidate_user()`) handles:
- Recording bad debt amount
- Drawing from insurance fund
- Socializing losses if insurance insufficient

**LP proceeds** are included in the equity calculation, so they reduce bad debt before socialization.

## Keeper Integration

The keeper (`keeper/src/health.rs`) monitors portfolios and detects when LP liquidation may be needed:

### Detection Functions

```rust
/// Check if portfolio may need LP liquidation
pub fn needs_lp_liquidation(portfolio: &Portfolio) -> bool {
    if portfolio.lp_buckets.is_empty() {
        return false;
    }
    portfolio.equity < (portfolio.mm as i128)
}
```

### Priority Ordering

```rust
/// Get buckets in liquidation priority order
pub fn get_lp_liquidation_priority(
    portfolio: &Portfolio
) -> (Vec<&LpBucket>, Vec<&LpBucket>) {
    // Returns (slab_buckets, amm_buckets)
}
```

### Staleness Monitoring

```rust
/// Check if AMM price is stale
pub fn is_amm_price_stale(
    bucket: &LpBucket,
    current_timestamp: u64,
    max_staleness_secs: u64,
) -> bool
```

**Keeper Workflow**:
1. Monitor portfolio health continuously
2. Detect underwater portfolios with LP positions
3. Call `liquidate_user` instruction
4. Router automatically handles principal → Slab LP → AMM LP priority
5. Monitor for stale AMM prices and trigger price updates if needed

## Example Scenarios

### Scenario 1: Principal Liquidation Sufficient

**Initial State**:
- Equity: $95
- MM: $100
- Principal positions: $80 in ETH-PERP
- Slab LP: $20 in BTC/USDC orderbook

**Liquidation Flow**:
1. Liquidate $10 of ETH-PERP → Equity now $105
2. Check deficit → 0 (healthy)
3. **Exit early** - LP positions untouched

**Result**: Only principal liquidated, LP positions preserved

### Scenario 2: Slab LP Liquidation Required

**Initial State**:
- Equity: $95
- MM: $100
- Principal positions: $50 in ETH-PERP
- Slab LP: $30 in BTC/USDC orderbook (reserved_base: $15, reserved_quote: $15)

**Liquidation Flow**:
1. Liquidate $50 of ETH-PERP → Equity now $95 (still underwater)
2. Check deficit → $5
3. Liquidate Slab LP bucket:
   - Free $30 in reserves ($15 base + $15 quote)
   - Apply proportional margin reduction (0% remaining)
   - Equity now $125
4. Recalculate margins → MM now $70 (no LP margin)
5. Check deficit → 0 (healthy)

**Result**: Principal + Slab LP liquidated, portfolio restored

### Scenario 3: AMM LP Liquidation Required

**Initial State**:
- Equity: $90
- MM: $100
- Principal positions: $40 in ETH-PERP
- Slab LP: $20 in BTC/USDC orderbook
- AMM LP: $50 in SOL/USDC AMM (1000 shares @ $50/share, updated 10s ago)

**Liquidation Flow**:
1. Liquidate $40 of ETH-PERP → Equity now $90
2. Check deficit → $10
3. Liquidate Slab LP → Equity now $110, MM now $90
4. Check deficit → 0 (healthy!)
5. **Exit early** - AMM LP preserved

**Alternative** (if Slab LP insufficient):
3. Liquidate Slab LP → Equity now $110, MM now $95
4. Check deficit → Still underwater
5. Check AMM price staleness → Fresh (10s < 60s max)
6. Liquidate AMM LP:
   - Calculate redemption: 1000 shares × $50 / 1e6 = $50
   - Burn all shares
   - Equity now $160
7. Recalculate margins → MM now $50
8. Check deficit → 0 (healthy)

**Result**: Principal + Slab LP + AMM LP liquidated

### Scenario 4: Stale AMM Price Protection

**Initial State**:
- Equity: $85
- MM: $100
- Principal positions: $50 (liquidated)
- Slab LP: $20 (liquidated)
- AMM LP: $50 (1000 shares @ $50/share, **updated 120s ago**)
- Max staleness: 60 seconds

**Liquidation Flow**:
1. Liquidate principal and Slab LP → Equity now $105, MM now $90
2. Still underwater → Try AMM LP
3. **Staleness guard triggers**: 120s > 60s max
4. Skip AMM LP bucket with warning log
5. Final deficit → Portfolio enters bad debt

**Result**: AMM LP preserved (stale price protection), bad debt handled by insurance fund

## Testing

### Unit Tests

Location: `programs/router/src/instructions/liquidate_user.rs:715-930`

**Coverage**:
- ✅ Deficit calculation (healthy, underwater, exact MM)
- ✅ Empty LP positions (early exit)
- ✅ Successful Slab LP liquidation
- ✅ Successful AMM LP liquidation
- ✅ Staleness guard functionality
- ✅ Bucket state transitions (active → inactive)
- ✅ LP share burning
- ✅ KANI verified function integration

### Integration Testing (Future)

Recommended test scenarios:
1. Full liquidation flow (principal → Slab → AMM)
2. Mixed venue liquidation with multiple buckets
3. Bad debt with LP liquidation
4. Rate limiting with LP liquidation
5. Concurrent liquidations on same portfolio

## Security Considerations

### 1. Staleness Guards

**Risk**: Liquidating on stale AMM prices could extract unfair value

**Mitigation**:
- Check `price_age > max_staleness_seconds`
- Skip stale buckets with warning log
- Keeper monitors and triggers price updates

### 2. Proportional Margin Reduction

**Risk**: Incorrect margin scaling could leave portfolio under-margined or over-margined

**Mitigation**:
- Use KANI formally verified functions (LP8-LP10)
- Verify no overflow, monotonicity, boundary conditions
- Unit tests cover 0%, 50%, 100% ratios

### 3. Redemption Value Calculation

**Risk**: Overflow when calculating `(shares * price) / scale`

**Mitigation**:
- Use KANI verified `calculate_redemption_value_verified()` (LP6)
- Proven overflow-free for valid inputs

### 4. Bad Debt Socialization

**Risk**: LP liquidation insufficient → insurance fund depletion → loss socialization

**Mitigation**:
- Three-tier priority minimizes bad debt
- Staleness guards prevent premature AMM LP liquidation
- Insurance fund handles remaining bad debt
- Rate limiting prevents vampire attacks

## Performance Considerations

### Computational Cost

**Per Liquidation**:
- Iterate through LP buckets (max ~10-20 per portfolio)
- KANI verified math: ~100-500 CU per call
- Margin recalculation: ~1000-5000 CU
- Total additional cost: ~5000-10000 CU

**Optimization**:
- Early exit when health restored
- Skip inactive buckets
- Skip stale AMM buckets

### Account Requirements

**No Additional Accounts**:
- LP liquidation uses existing `Portfolio` account
- No new accounts required in `liquidate_user` instruction
- Keeper uses existing RPC queries

## Future Enhancements

### 1. Partial LP Liquidation

**Current**: Liquidates entire bucket (all-or-nothing)

**Enhancement**: Liquidate only portion needed to restore health

**Benefit**: Preserve more LP positions, better UX

**Implementation**: Calculate exact shares to burn based on deficit

### 2. LP Position Transfers

**Current**: Liquidate LP positions (burn shares, free reserves)

**Enhancement**: Transfer LP positions to liquidator

**Benefit**: Liquidators can maintain LP positions, potentially better price discovery

**Challenge**: Requires liquidator to have margin capacity

### 3. Dutch Auction for LP Liquidation

**Current**: Liquidate at current redemption value

**Enhancement**: Gradual price decay to find market price

**Benefit**: More efficient price discovery, better liquidator competition

**Challenge**: Requires auction state management

## References

- **Implementation**: `programs/router/src/instructions/liquidate_user.rs`
- **Verified Functions**: `programs/router/src/state/model_bridge.rs`
- **Keeper Monitoring**: `keeper/src/health.rs`
- **LP Bucket State**: `programs/router/src/state/lp_bucket.rs`
- **Unit Tests**: `programs/router/src/instructions/liquidate_user.rs:715-930`
- **Original Plan**: `/tmp/lp_liquidations_revised.md`
- **Status Tracking**: `/tmp/lp_liquidation_implementation_status.md`

## Summary

The LP liquidation architecture provides a safe, capital-efficient mechanism to restore portfolio health when principal liquidation is insufficient. By using:

1. **Three-tier priority** - Principal → Slab LP → AMM LP
2. **KANI verified math** - Formally proven overflow-free, monotonic
3. **Staleness guards** - Protect against mispriced AMM liquidations
4. **Incremental health checks** - Only liquidate what's needed
5. **Existing instructions** - No protocol changes required

The system maintains the safety and capital efficiency goals of the Percolator DEX while handling the complex edge cases of LP position liquidation.
