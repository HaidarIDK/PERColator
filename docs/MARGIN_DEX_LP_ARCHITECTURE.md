# Margin DEX LP Architecture

## Critical Understanding

**This codebase is for a PERP MARGIN DEX ONLY.**

There is **NO "direct LP" option**. ALL capital must flow through the router for proper margin accounting and settlement.

## Why Direct LP + Margin LP Cannot Mix

### The Problem
- **Direct LP**: Places orders with real 1:1 collateral
- **Margin LP**: Places orders with leveraged/synthetic positions (e.g., 5:1)
- **Settlement mismatch**: When they trade, direct LP expects full settlement but margin LP might be undercollateralized
- **Result**: Direct LP loses funds if margin LP gets liquidated

### The Solution
**Router-only LP**: ALL LPs use the router margin system
- Router holds all capital in custody
- Router enforces margin requirements via seat limits
- Router guarantees settlement for all fills via CommitFill
- Cross-margining across multiple venues

## Correct Architecture

### There Are Only 2 LP Scenarios

1. **Router→Slab LP** - Orderbook market making with margin
2. **Router→AMM LP** - Concentrated liquidity with margin

Both use the same flow: `RouterReserve → RouterLiquidity → Adapter → RouterRelease`

## Router→Slab LP Flow

### Step-by-Step

```
1. Initialize Portfolio (margin account)
   └─> User's portfolio PDA created

2. Deposit Collateral
   └─> Funds transferred to portfolio

3. RouterReserve (discriminator 9)
   ├─> Lock collateral from portfolio into LP seat
   ├─> Accounts: [portfolio_pda, lp_seat_pda]
   └─> Data: [disc(1), base_amount_q64(16), quote_amount_q64(16)]

4. RouterLiquidity (discriminator 11) with ObAdd intent
   ├─> RiskGuard: max_slippage_bps, max_fee_bps, oracle_bound_bps
   ├─> Intent: ObAdd {
   │     orders: Vec<ObOrder>,
   │     post_only: bool,
   │     reduce_only: bool,
   │   }
   ├─> CPI to slab program (discriminator 2 - adapter_liquidity)
   ├─> Accounts: [portfolio_pda, lp_seat_pda, venue_pnl_pda, slab_state]
   └─> Slab adapter verifies router authority, places orders

5. Slab Adapter Processing
   ├─> Verify router signer (adapter.rs:52-55)
   ├─> Call process_place_order with lp_owner (adapter.rs:116)
   ├─> Orders owned by slab's lp_owner
   ├─> Capital stays in router custody
   └─> Return LiquidityResult (exposure delta, fees, etc.)

6. Seat Limit Check
   ├─> Router verifies: exposure within reserved amounts
   ├─> check_limits(haircut_base_bps, haircut_quote_bps)
   └─> Fails if LP exceeds margin limits

7. RouterRelease (discriminator 10)
   ├─> Unlock collateral from LP seat back to portfolio
   ├─> Accounts: [portfolio_pda, lp_seat_pda]
   └─> Data: [disc(1), base_amount_q64(16), quote_amount_q64(16)]
```

### ObAdd Serialization Format

```rust
// RouterLiquidity builds adapter instruction (disc 2)
[discriminator: 2]           // adapter_liquidity
[intent_disc: 2]             // ObAdd
[orders_count: u32]          // Number of orders
// For each order:
[side: u8]                   // 0=Bid, 1=Ask
[px_q64: u128]               // Price in Q64 (16 bytes)
[qty_q64: u128]              // Quantity in Q64 (16 bytes)
[tif_slots: u32]             // Time-in-force (4 bytes)
[post_only: u8]              // 0=false, 1=true
[reduce_only: u8]            // 0=false, 1=true
[RiskGuard: 8 bytes]         // max_slippage_bps, max_fee_bps, oracle_bound_bps
```

## Router→AMM LP Flow

### Step-by-Step

Same as slab, but with **AmmAdd** intent instead of ObAdd:

```
3-4. RouterReserve → RouterLiquidity with AmmAdd intent
     ├─> Intent: AmmAdd {
     │     lower_px_q64: u128,      // Price range lower bound
     │     upper_px_q64: u128,      // Price range upper bound
     │     quote_notional_q64: u128,// Amount to add
     │     curve_id: u32,           // Curve type
     │     fee_bps: u16,            // LP fee
     │   }
     ├─> CPI to AMM program (discriminator 2 - adapter_liquidity)
     └─> AMM mints LP shares, capital in router custody
```

## Key Components

### Discriminators

#### Slab Program
- 0: Initialize
- 1: CommitFill (router only - match orders)
- **2: AdapterLiquidity (router LP - THE ONLY WAY for margin DEX)** ← Production
- 3: PlaceOrder (testing only - deprecated)
- 4: CancelOrder (testing only - deprecated)
- 5: UpdateFunding
- 6: HaltTrading
- 7: ResumeTrading
- 8: ModifyOrder (testing only - deprecated)

#### AMM Program
- 0: Initialize
- 1: CommitFill (router only)
- **2: AdapterLiquidity (router LP - production)** ← Production

#### Router Program
- 9: RouterReserve
- 10: RouterRelease
- 11: RouterLiquidity

### Intent Discriminators (for AdapterLiquidity)
- 0: AmmAdd
- 1: Remove
- 2: ObAdd

### Remove Selectors
- 0: AmmByShares
- 1: ObByIds
- 2: ObAll

## Settlement Flow

### For Traders
```
Trader → ExecuteCrossSlab → Router checks margin
                          ↓
                    CommitFill on slab
                          ↓
                    Router settles from escrow
```

### For LPs
```
LP Order (via RouterLiquidity) → Rests on slab
                                      ↓
Trader → ExecuteCrossSlab → CommitFill matches LP order
                                      ↓
                          Router settles both sides
                          (trader's margin, LP's reserved collateral)
```

**Critical**: Router owns/controls ALL capital, guaranteeing settlement for both sides.

## Why This Architecture Works

1. **Single Source of Truth**: Router custody ensures settlement
2. **Margin Enforcement**: Seat limits prevent over-leverage
3. **Cross-Margining**: Same collateral across multiple venues
4. **No Capital Mismatch**: All parties use router's margin system
5. **Liquidation Safety**: Router can liquidate undercollateralized positions

## Why "Direct LP" Was Wrong

The initial documentation mentioned "direct LP" where LPs call PlaceOrder directly:
- ❌ Capital would NOT be in router custody
- ❌ Settlement would fail (router doesn't have LP's funds)
- ❌ Margin/direct LP mix would create capital mismatch
- ❌ No cross-margining possible
- ❌ Violates margin DEX architecture

**Correct**: PlaceOrder (disc 3) is for **TESTING ONLY**. Production uses adapter_liquidity (disc 2) via router.

## LP Operations Summary

### Add Liquidity
```
Router: RouterReserve → RouterLiquidity (ObAdd/AmmAdd) → Adapter
           ↓                    ↓                           ↓
     Lock collateral    CPI with intent           Place orders/mint shares
```

### Remove Liquidity
```
Router: RouterLiquidity (Remove) → Adapter → RouterRelease
                ↓                       ↓            ↓
         CPI with selector      Cancel orders   Unlock collateral
```

### Modify Liquidity
```
Router: RouterLiquidity (Remove) → RouterLiquidity (ObAdd/AmmAdd)
                ↓                           ↓
         Cancel old orders          Place new orders
```

## Testing

### Current Status
- ✅ Infrastructure complete (ObAdd support added)
- ✅ Discriminators standardized (disc 2 = adapter_liquidity)
- ⚠️ CLI needs enhancement (--mode orderbook support)

### Test Files
- `test_router_lp_slab.sh` - Documents router→slab LP flow
- (TODO) `test_router_lp_amm.sh` - Router→AMM LP flow
- (TODO) `test_router_lp_mixed.sh` - Cross-margining test

### Next Steps for Full Testing
1. Add CLI support for ObAdd:
   ```bash
   ./percolator liquidity add <SLAB> <AMOUNT> --mode orderbook \
     --price <PRICE> --post-only --reduce-only
   ```

2. Implement reserve/release CLI commands:
   ```bash
   ./percolator router reserve <MATCHER> --base <AMT> --quote <AMT>
   ./percolator router release <MATCHER> --base <AMT> --quote <AMT>
   ```

3. Create E2E tests verifying:
   - Reserve → Liquidity → seat limit check
   - Liquidity → Remove → Release
   - Mixed slab + AMM with shared collateral

## References

- Slab adapter: `programs/slab/src/adapter.rs`
- Router liquidity: `programs/router/src/instructions/router_liquidity.rs`
- Adapter core types: `crates/adapter_core/src/lib.rs`
- Slab entrypoint: `programs/slab/src/entrypoint.rs` (disc 2 handling)

## Conclusion

**For a perp margin DEX:**
- There is ONLY router-based LP
- ALL capital flows through router
- Discriminator 2 = adapter_liquidity (production LP path)
- PlaceOrder/CancelOrder = testing only, deprecated for production
- This architecture prevents capital mismatch and enables proper margin enforcement
