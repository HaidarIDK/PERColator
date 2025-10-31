# Liquidity Provider Scenarios

## Overview

Percolator supports **4 distinct ways** for LPs to provide liquidity:

1. **Direct Slab LP** - Orderbook market making, no margin
2. **Direct AMM LP** - Concentrated liquidity, no margin
3. **Router→Slab LP** - Orderbook market making with leverage
4. **Router→AMM LP** - Concentrated liquidity with leverage

## Architecture Context

Percolator has a two-tier architecture:

- **Tier 1: Matchers (Slab/AMM)** - Orderbook and liquidity logic
- **Tier 2: Router** - Margin, portfolio, PnL management

LPs can choose to interact **directly** with matchers (simple, no leverage) or **through the router** (with leverage and cross-margining).

For comparison, **traders** can ONLY use the router (`ExecuteCrossSlab`) and can ONLY submit taker orders. Traders cannot provide resting liquidity.

---

## Scenario 1: Direct Slab LP (No Margin)

### Description
LP places limit orders directly on the orderbook using their own funds. No margin system involved.

### Flow
```
LP → PlaceOrder → Slab → Order rests on book
```

### Instructions Used
- **PlaceOrder** (discriminator 2)
  - Arguments: `price`, `qty`, `side`, `post_only`, `reduce_only`
  - Accounts: `[slab_account, owner_signer]`

- **CancelOrder** (discriminator 3)
  - Arguments: `order_id`
  - Accounts: `[slab_account, owner_signer]`

### CLI Commands
```bash
# Place buy order
./percolator matcher place-order <SLAB> buy \
  --price 50000000000 \
  --qty 1000000

# Place sell order
./percolator matcher place-order <SLAB> sell \
  --price 51000000000 \
  --qty 1000000

# Cancel order
./percolator matcher cancel-order <SLAB> <ORDER_ID>
```

### Characteristics
- ✅ **Simple** - Direct program calls, no middleware
- ✅ **No margin** - Uses LP's own funds
- ✅ **No leverage** - 1:1 capital usage
- ✅ **Full control** - LP manages individual orders
- ❌ **No cross-margining** - Each venue isolated
- ❌ **Capital intensive** - Must have full collateral

### Status
✅ **IMPLEMENTED & TESTED**

---

## Scenario 2: Direct AMM LP (No Margin)

### Description
LP adds liquidity directly to AMM pools, receiving LP shares. No margin system involved.

### Flow
```
LP → adapter_liquidity → AMM → LP shares minted
```

### Instructions Used
- **Initialize AMM** (discriminator 0)
  - Arguments: `lp_owner`, `router_id`, `instrument`, `mark_px`, `taker_fee_bps`, `contract_size`, `bump`, `x_reserve`, `y_reserve`
  - Accounts: `[amm_account, payer]`

- **AdapterLiquidity - AmmAdd** (discriminator 2)
  - Arguments:
    ```rust
    LiquidityIntent::AmmAdd {
        lower_px_q64: u128,      // Price range lower bound
        upper_px_q64: u128,      // Price range upper bound
        quote_notional_q64: u128,// Amount to add
        curve_id: u32,           // Curve type (0 = constant product)
        fee_bps: u16,            // LP fee (e.g., 30 = 0.3%)
    }
    RiskGuard {
        max_slippage_bps: u16,
        max_fee_bps: u16,
        oracle_bound_bps: u16,
    }
    ```
  - Accounts: `[amm_account, lp_token_account, user_signer]`

### CLI Commands (Proposed)
```bash
# Create AMM pool
./percolator amm create <REGISTRY> <INSTRUMENT> \
  --x-reserve 1000000000 \
  --y-reserve 1000000000

# Add liquidity
./percolator amm add-liquidity <AMM> \
  --amount 100000000 \
  --lower-price 49000000000 \
  --upper-price 51000000000 \
  --fee-bps 30
```

### Characteristics
- ✅ **Concentrated liquidity** - Capital efficient range orders
- ✅ **Passive** - No active order management needed
- ✅ **No margin** - LP owns shares directly
- ✅ **Fungible** - LP shares are transferable tokens
- ❌ **No leverage** - 1:1 capital usage
- ❌ **No cross-margining** - Each AMM isolated

### Status
⚠️ **PROGRAM READY, CLI PENDING**
- Program: AMM `adapter_liquidity` instruction implemented
- CLI gap: Need AMM create and add-liquidity commands

---

## Scenario 3: Router→Slab LP (With Margin)

### Description
LP uses the router's margin system to place orderbook orders with leverage and cross-margining.

### Flow
```
LP → RouterReserve → Reserve collateral from portfolio
   → RouterLiquidity → CPI to slab adapter → PlaceOrder
   → Slab orders placed, seat limits checked
```

### Instructions Used
1. **RouterReserve** (discriminator 9)
   - Arguments: `base_amount_q64`, `quote_amount_q64`
   - Accounts: `[portfolio_pda, lp_seat_pda]`
   - Effect: Locks collateral from portfolio into LP seat

2. **RouterLiquidity** (discriminator 11) with **ObAdd** intent
   - Arguments:
     ```rust
     LiquidityIntent::ObAdd {
         orders: Vec<ObOrder>,  // List of orders to place
         post_only: bool,
         reduce_only: bool,
     }
     RiskGuard {
         max_slippage_bps: u16,
         max_fee_bps: u16,
         oracle_bound_bps: u16,
     }
     ```
   - Accounts: `[portfolio_pda, lp_seat_pda, venue_pnl_pda, matcher_state]`
   - Effect: CPI to slab adapter, places orders, checks seat limits

3. **RouterRelease** (discriminator 10)
   - Arguments: `base_amount_q64`, `quote_amount_q64`
   - Accounts: `[portfolio_pda, lp_seat_pda]`
   - Effect: Unlocks collateral from LP seat back to portfolio

### CLI Commands (Proposed)
```bash
# Initialize portfolio
./percolator margin init

# Deposit collateral
./percolator margin deposit 1000

# Add orderbook liquidity via router
./percolator liquidity add <SLAB> 100 \
  --mode orderbook \
  --price 50500000000 \
  --post-only

# Remove liquidity
./percolator liquidity remove <SLAB> \
  --mode orderbook
```

### Characteristics
- ✅ **Leverage** - Use same collateral across multiple venues
- ✅ **Cross-margining** - Portfolio-level risk management
- ✅ **Credit discipline** - Seat limits enforce exposure caps
- ✅ **PnL tracking** - Centralized accounting
- ❌ **More complex** - Multi-step flow
- ❌ **Router custody** - Collateral held by router

### Status
⚠️ **PARTIALLY IMPLEMENTED**
- Infrastructure: Router + Slab adapter ready
- CLI gap: Need **ObAdd** variant support in `liquidity add` command
- Current CLI only supports AmmAdd

---

## Scenario 4: Router→AMM LP (With Margin)

### Description
LP uses the router's margin system to provide AMM liquidity with leverage and cross-margining.

### Flow
```
LP → RouterReserve → Reserve collateral from portfolio
   → RouterLiquidity → CPI to AMM adapter → Add liquidity
   → LP shares minted, seat limits checked
```

### Instructions Used
1. **RouterReserve** (discriminator 9)
   - Same as Scenario 3

2. **RouterLiquidity** (discriminator 11) with **AmmAdd** intent
   - Arguments:
     ```rust
     LiquidityIntent::AmmAdd {
         lower_px_q64: u128,
         upper_px_q64: u128,
         quote_notional_q64: u128,
         curve_id: u32,
         fee_bps: u16,
     }
     RiskGuard { ... }
     ```
   - Accounts: `[portfolio_pda, lp_seat_pda, venue_pnl_pda, matcher_state]`
   - Effect: CPI to AMM adapter, adds liquidity, mints LP shares to seat

3. **RouterRelease** (discriminator 10)
   - Same as Scenario 3

### CLI Commands
```bash
# Initialize portfolio (if not done)
./percolator margin init

# Deposit collateral
./percolator margin deposit 1000

# Add AMM liquidity via router (CURRENT DEFAULT)
./percolator liquidity add <AMM> 100 \
  --lower-price 49000000000 \
  --upper-price 51000000000

# Remove liquidity
./percolator liquidity remove <AMM> 100
```

### Characteristics
- ✅ **Leverage** - Use same collateral across venues
- ✅ **Cross-margining** - Portfolio-level risk
- ✅ **Concentrated liquidity** - Capital efficient
- ✅ **PnL tracking** - Centralized accounting
- ❌ **More complex** - Multi-step flow
- ❌ **Router custody** - LP shares held in seat

### Status
⚠️ **INFRASTRUCTURE READY**
- Infrastructure: Router + AMM adapter exists
- CLI: Supports AmmAdd (default in `liquidity add`)
- Gap: Need AMM state creation in CLI

---

## Comparison Matrix

| Feature | Direct Slab | Direct AMM | Router→Slab | Router→AMM |
|---------|-------------|------------|-------------|------------|
| **Margin** | No | No | Yes | Yes |
| **Leverage** | No | No | Yes | Yes |
| **Cross-margining** | No | No | Yes | Yes |
| **Complexity** | Low | Low | High | High |
| **Custody** | Self | Self | Router | Router |
| **Liquidity Type** | Orderbook | AMM | Orderbook | AMM |
| **Capital Efficiency** | Low | Medium | High | High |
| **Active Management** | Yes | No | Yes | No |

---

## Use Cases

### Direct Slab LP (Scenario 1)
- **Who**: Professional market makers with ample capital
- **When**: Want full control, no leverage needed
- **Why**: Simplicity, direct custody, predictable fees

### Direct AMM LP (Scenario 2)
- **Who**: Passive LPs seeking yield
- **When**: Want set-and-forget liquidity provision
- **Why**: No active management, capital efficient range orders

### Router→Slab LP (Scenario 3)
- **Who**: Professional market makers with limited capital
- **When**: Want to provide liquidity across multiple venues
- **Why**: Leverage, cross-margining, portfolio risk management

### Router→AMM LP (Scenario 4)
- **Who**: Sophisticated LPs seeking maximum capital efficiency
- **When**: Want passive liquidity with leverage
- **Why**: Concentrated liquidity + leverage + cross-margining

---

## Test Coverage

### Implemented & Tested
✅ **Scenario 1: Direct Slab LP**
- Test: `test_lp_scenarios.sh` (Scenario 1)
- Coverage: Place orders, cancel orders, verify orderbook

### Ready for Testing (CLI Pending)
⚠️ **Scenario 2: Direct AMM LP**
- Program: Ready (discriminator 2)
- CLI: Need `amm create` and `amm add-liquidity`

⚠️ **Scenario 3: Router→Slab LP**
- Infrastructure: Ready
- CLI: Need ObAdd variant in `liquidity add`

⚠️ **Scenario 4: Router→AMM LP**
- Infrastructure: Ready
- CLI: Has AmmAdd support, needs AMM creation

---

## Next Steps

### CLI Enhancements Needed

1. **AMM Commands**
   ```bash
   ./percolator amm create <registry> <instrument> --x-reserve <amt> --y-reserve <amt>
   ./percolator amm add-liquidity <amm> --amount <amt> --lower-price <px> --upper-price <px>
   ./percolator amm remove-liquidity <amm> --shares <amt>
   ```

2. **Liquidity Command Enhancements**
   ```bash
   # Add --mode flag to support both ObAdd and AmmAdd
   ./percolator liquidity add <matcher> <amount> --mode {orderbook|amm} [OPTIONS]

   # For orderbook mode:
   --price <px>
   --post-only
   --reduce-only

   # For AMM mode (existing):
   --lower-price <px>
   --upper-price <px>
   --fee-bps <bps>
   ```

### Testing Roadmap

1. ✅ Direct Slab LP - DONE
2. Implement AMM CLI commands
3. Test Direct AMM LP (Scenario 2)
4. Add ObAdd support to liquidity CLI
5. Test Router→Slab LP (Scenario 3)
6. Test Router→AMM LP (Scenario 4)

---

## Key Takeaways

1. **Flexibility**: LPs have 4 distinct options based on their needs
2. **Two Axes**: Direct vs Router (margin), Slab vs AMM (liquidity type)
3. **Trade-offs**: Simplicity vs leverage, control vs passivity
4. **Trader Distinction**: Traders can ONLY use router, ONLY taker orders
5. **Production Ready**: Core infrastructure complete, CLI enhancements needed
