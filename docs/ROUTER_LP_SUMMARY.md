# Router LP Implementation Summary

## Overview

This document summarizes the critical architectural corrections and implementations for the **Percolator Margin DEX LP system**.

## The Fundamental Insight

> "even AMMs can't have mixed LPs. this code base is for the perp margin dex only, so you can get rid of all the non-router options for LPs."

This user insight revealed a fundamental misunderstanding in the initial documentation and led to critical fixes.

## What Was Wrong

### Incorrect: "Direct LP" Model
The initial documentation suggested **4 LP scenarios**:
1. ❌ Direct Slab LP (no margin)
2. ❌ Direct AMM LP (no margin)
3. ✓ Router→Slab LP (with margin)
4. ✓ Router→AMM LP (with margin)

### Why This Was Unsafe
- **Capital mismatch**: Direct LP (1:1 collateral) + Margin LP (leveraged) on same book
- **Settlement failure**: Router doesn't hold direct LP's capital
- **Liquidation risk**: Direct LP loses funds if margin LP gets liquidated
- **Architecture violation**: Margin DEX requires router custody of ALL capital

## What We Fixed

### 1. Discriminator Standardization (Commit d5d0cf5)

**Problem**: Router sent disc 2, but slab expected disc 4 for `adapter_liquidity`

**Solution**: Standardized disc 2 = `adapter_liquidity` across all matchers

#### New Slab Discriminator Mapping
```
0: Initialize
1: CommitFill (router only)
2: AdapterLiquidity (PRODUCTION - router LP only) ← Standardized
3: PlaceOrder (testing only) ← Deprecated
4: CancelOrder (testing only) ← Deprecated
5: UpdateFunding
6: HaltTrading
7: ResumeTrading
8: ModifyOrder (testing only) ← Deprecated
```

#### Changes
- `programs/slab/src/entrypoint.rs`: Moved adapter_liquidity from disc 4 → 2
- `programs/slab/src/instructions/mod.rs`: Updated enum values
- `cli/src/matcher.rs`: Updated PlaceOrder disc 2→3, CancelOrder disc 3→4
- Marked PlaceOrder/CancelOrder/ModifyOrder as **TESTING ONLY**

### 2. ObAdd Support (Commit a0d2e84)

**Problem**: RouterLiquidity only supported AmmAdd, missing ObAdd for orderbook

**Solution**: Added full ObAdd and orderbook removal support

#### Changes
- `programs/router/src/instructions/router_liquidity.rs`:
  - Added ObAdd intent serialization
  - Added ObByIds and ObAll removal selectors
  - Matches slab adapter's expected format

#### ObAdd Serialization Format
```rust
[disc: 2]                    // adapter_liquidity
[intent: 2]                  // ObAdd
[count: u32]                 // Number of orders
// For each order (37 bytes):
[side: u8]                   // 0=Bid, 1=Ask
[px_q64: u128]               // Price (16 bytes)
[qty_q64: u128]              // Quantity (16 bytes)
[tif_slots: u32]             // Time-in-force (4 bytes)
[post_only: u8]              // Flag
[reduce_only: u8]            // Flag
[RiskGuard: 8 bytes]         // Slippage, fees, oracle bounds
```

### 3. Documentation Correction (Commit 98ce1e7)

**Deleted**:
- `docs/LP_SCENARIOS.md` - Incorrect 4-scenario model
- `test_lp_scenarios.sh` - Incorrect direct LP tests

**Created**:
- `docs/MARGIN_DEX_LP_ARCHITECTURE.md` - Comprehensive margin DEX architecture
- `test_router_lp_slab.sh` - Slab LP test with flow documentation

### 4. Comprehensive Test Scenarios (Commit 6bd2b16)

**Created**:
- `test_router_lp_amm.sh` (8.6KB) - AMM LP flow with comparison to slab
- `test_router_lp_mixed.sh` (19KB) - Cross-margining demonstration

## Correct Architecture

### There Are ONLY 2 LP Scenarios

#### 1. Router→Slab LP (Orderbook Market Making)
```
Portfolio (margin account)
    ↓
RouterReserve (lock collateral)
    ↓
RouterLiquidity (ObAdd intent)
    ↓ CPI (discriminator 2)
Slab Adapter (verifies router authority)
    ↓
Orders placed on book (owned by lp_owner, capital in router)
    ↓
Seat limit check (exposure < reserved × (1 - haircut))
    ↓
RouterRelease (unlock collateral)
```

#### 2. Router→AMM LP (Concentrated Liquidity)
```
Portfolio (margin account)
    ↓
RouterReserve (lock collateral)
    ↓
RouterLiquidity (AmmAdd intent)
    ↓ CPI (discriminator 2)
AMM Adapter (verifies router authority)
    ↓
Liquidity added, LP shares minted (capital in router)
    ↓
Seat limit check (shares + exposure < reserved × (1 - haircut))
    ↓
RouterRelease (unlock collateral)
```

## Cross-Margining: The Key Value Proposition

### Traditional (Isolated Margin)
```
Slab LP: 50k locked → 500k max exposure
AMM LP:  50k locked → 500k max exposure
Total:   100k capital for 1M exposure
```

### Percolator (Cross-Margin)
```
Portfolio: 50k reserved → 1M+ exposure (slab + AMM combined)
Capital efficiency: 2× improvement
Risk netting: Long/short positions offset
```

### How It Works

#### Portfolio-Level View
```
Portfolio (100k total collateral)
├─> Slab LP Seat
│   ├─> Reserved: 25k base + 25k quote
│   ├─> Exposure: +10 base, -500k quote
│   └─> Orders: 2 active limit orders
│
├─> AMM LP Seat
│   ├─> Reserved: 25k base + 25k quote
│   ├─> Exposure: +5 base, -245k quote
│   └─> LP Shares: 15,000
│
└─> Free collateral: 50k
```

#### Margin Enforcement
- **Per-seat tracking**: Each venue has its own LP seat
- **Aggregate limits**: Router checks TOTAL exposure < TOTAL reserved
- **Haircut adjustments**: exposure < reserved × (1 - haircut_bps / 10000)
- **Dynamic enforcement**: Checked on every RouterLiquidity call

## Settlement Flow

### For Traders
```
Trader → ExecuteCrossSlab → Router checks margin
                          ↓
                    CommitFill on matcher
                          ↓
                    Router settles from escrow
```

### For LPs
```
LP Order (via RouterLiquidity) → Rests on book/curve
                                      ↓
Trader → ExecuteCrossSlab → CommitFill matches LP order
                                      ↓
                          Router settles both sides
                          (trader's margin + LP's reserved collateral)
```

**Critical**: Router owns/controls ALL capital, guaranteeing settlement.

## Code Organization

### Core Components

#### Router Instructions
- `router_reserve.rs` - Lock collateral from portfolio to LP seat
- `router_liquidity.rs` - Execute LP operations via matcher adapters
- `router_release.rs` - Unlock collateral from LP seat to portfolio

#### Matcher Adapters
- `programs/slab/src/adapter.rs` - Slab orderbook adapter
- `programs/amm/src/adapter.rs` - AMM liquidity adapter
- Both use discriminator 2 for `adapter_liquidity`

#### Adapter Core
- `crates/adapter_core/src/lib.rs` - Shared types
  - `LiquidityIntent` enum (AmmAdd, ObAdd, Remove, etc.)
  - `ObOrder` struct
  - `RemoveSel` enum
  - `LiquidityResult` struct

### Discriminator Map

#### Intent Discriminators (for AdapterLiquidity)
- 0: AmmAdd
- 1: Remove
- 2: ObAdd
- 3: Hook (custom matcher operations)
- 4: Modify

#### Remove Selectors
- 0: AmmByShares
- 1: ObByIds
- 2: ObAll

## Testing Infrastructure

### Test Files Created

1. **test_router_lp_slab.sh** (7.6KB)
   - Isolated slab LP testing
   - Documents ObAdd flow
   - Identifies CLI gaps

2. **test_router_lp_amm.sh** (8.6KB)
   - Isolated AMM LP testing
   - Documents AmmAdd flow
   - Compares AMM vs orderbook

3. **test_router_lp_mixed.sh** (19KB)
   - Cross-margining demonstration
   - Portfolio-level visualization
   - Capital efficiency examples
   - Margin enforcement scenarios

### Running Tests

```bash
# Slab LP (isolated)
./test_router_lp_slab.sh

# AMM LP (isolated)
./test_router_lp_amm.sh

# Cross-margining (mixed)
./test_router_lp_mixed.sh
```

## Next Steps for Production

### CLI Enhancements Needed

1. **Orderbook mode support**:
   ```bash
   ./percolator liquidity add <SLAB> <AMOUNT> --mode orderbook \
     --price <PRICE> --post-only --reduce-only
   ```

2. **Router reserve/release**:
   ```bash
   ./percolator router reserve <MATCHER> --base <AMT> --quote <AMT>
   ./percolator router release <MATCHER> --base <AMT> --quote <AMT>
   ```

3. **AMM creation**:
   ```bash
   ./percolator amm create <REGISTRY> <INSTRUMENT> \
     --x-reserve <AMT> --y-reserve <AMT>
   ```

### Testing Roadmap

1. ✅ Infrastructure complete (ObAdd support)
2. ✅ Documentation corrected (margin DEX architecture)
3. ✅ Test scenarios created (slab, AMM, mixed)
4. ⏳ CLI enhancements (--mode orderbook, reserve/release)
5. ⏳ E2E testing with state verification
6. ⏳ Seat limit enforcement verification
7. ⏳ Liquidation scenario testing

## Key Takeaways

### Architecture
- **Router-only LP**: All LPs must use router margin system
- **No direct LP**: PlaceOrder/CancelOrder are testing-only
- **Uniform discriminators**: Disc 2 = adapter_liquidity everywhere
- **Settlement guarantee**: Router custody enables safe margin trading

### Capital Efficiency
- **Cross-margining**: 2× capital efficiency vs isolated margin
- **Risk netting**: Offsetting positions reduce requirements
- **Flexible rebalancing**: Move exposure between venues
- **Unified liquidation**: Portfolio-level risk management

### Safety
- **No capital mismatch**: All LPs use same margin system
- **Margin enforcement**: Seat limits prevent over-leverage
- **Router authority**: Adapter verifies router CPI calls
- **Guaranteed settlement**: Router owns all capital

## Credits

This architectural correction was prompted by critical user feedback:

> "i dont see how direct LPs can work with margin LPs. since margin liqudity is synthetic, direct LPs will lose funds"

> "even AMMs can't have mixed LPs. this code base is for the perp margin dex only, so you can get rid of all the non-router options for LPs."

This insight led to:
- Discriminator standardization
- ObAdd implementation
- Documentation correction
- Comprehensive test scenarios

Thank you for catching this fundamental issue! 🙏

## References

- **Architecture**: `docs/MARGIN_DEX_LP_ARCHITECTURE.md`
- **Slab Adapter**: `programs/slab/src/adapter.rs`
- **Router Liquidity**: `programs/router/src/instructions/router_liquidity.rs`
- **Slab Entrypoint**: `programs/slab/src/entrypoint.rs` (disc 2 handling)
- **Adapter Core**: `crates/adapter_core/src/lib.rs`
