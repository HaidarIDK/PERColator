# V1 Router Integration Plan

**Status**: Planned for v1  
**Current**: v0 uses direct AMM calls  
**Goal**: Add portfolio margin and cross-slab routing via Router program

---

## Overview

The Router program provides two critical features that v0 direct AMM calls don't have:

1. **Portfolio Margin**: Net exposure across multiple positions reduces capital requirements
2. **Cross-Slab Routing**: Split large orders across multiple pools for optimal pricing

---

## Architecture

```
User Wallet
    ↓
Frontend Call
    ↓
Router Program (RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr)
    ├→ Portfolio Account (PDA per user)
    ├→ Vault Account (collateral custody)
    └→ CPI to AMM/Slab Programs
        ├→ SOL/USDC AMM (FvxA93qPzDEGVdkP4PDE1gpXpt9R3u8gBgh9FTovsHJm)
        ├→ ETH/USDC AMM (GPLmAVdfwE6zD1acgd5mZ7Myfq57oCQeJ9KGnco58YdQ)
        └→ BTC/USDC AMM (6vpuVH6SZX5a9PZSgMNBoZQAsTZtNJrEtd87RQCUHsPC)
```

---

## Router Program Instructions

### Already Deployed on Devnet

**Program ID**: `RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr`  
**Registry Account**: `DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx`

### Available Instructions

```rust
pub enum RouterInstruction {
    /// Initialize router registry (✅ DONE)
    Initialize = 0,
    
    /// Initialize user portfolio (📝 TODO)
    InitializePortfolio = 1,
    
    /// Deposit collateral to vault (📝 TODO)
    Deposit = 2,
    
    /// Withdraw collateral from vault (📝 TODO)
    Withdraw = 3,
    
    /// Execute cross-slab order (📝 TODO - MAIN v1 FEATURE)
    ExecuteCrossSlab = 4,
    
    /// Liquidate user positions (📝 TODO)
    LiquidateUser = 5,
    
    /// Burn AMM LP shares (📝 TODO)
    BurnLpShares = 6,
    
    /// Cancel Slab LP orders (📝 TODO)
    CancelLpOrders = 7,
}
```

---

## Implementation Roadmap for v1

### Phase 1: User Portfolio System (Week 1)

#### 1.1 Initialize User Portfolios
Create script: `scripts/initialize-portfolio.ts`

```typescript
// Derive portfolio PDA for user
const [portfolioPDA, bump] = PublicKey.findProgramAddressSync(
  [Buffer.from('portfolio'), userPublicKey.toBuffer()],
  ROUTER_PROGRAM_ID
);

// Build InitializePortfolio instruction
const data = Buffer.alloc(1);
data.writeUInt8(1, 0); // InitializePortfolio discriminator

const instruction = new TransactionInstruction({
  keys: [
    { pubkey: portfolioPDA, isSigner: false, isWritable: true },
    { pubkey: userPublicKey, isSigner: true, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ],
  programId: ROUTER_PROGRAM_ID,
  data,
});
```

**Portfolio Account Structure**:
```rust
pub struct Portfolio {
    pub router_id: Pubkey,        // Router program ID
    pub user: Pubkey,              // User wallet
    pub bump: u8,                  // PDA bump
    
    // Margin tracking
    pub im: u128,                  // Initial margin required
    pub mm: u128,                  // Maintenance margin required
    
    // Exposures (up to 256 positions)
    pub exposures: [(u16, u16, i64); 256],  // (slab_idx, instrument_idx, qty)
    pub exposure_count: u16,
}
```

**Size**: ~12KB per portfolio

#### 1.2 Vault Account Setup
Create shared vault for collateral custody.

```rust
pub struct Vault {
    pub router_id: Pubkey,
    pub authority: Pubkey,
    pub total_collateral: u128,
    pub user_balances: BTreeMap<Pubkey, u128>,  // User -> Balance
}
```

---

### Phase 2: Deposit & Withdraw (Week 1-2)

#### 2.1 Deposit Collateral
```typescript
// Transfer SOL to vault, credit user portfolio
const depositIx = new TransactionInstruction({
  keys: [
    { pubkey: userPublicKey, isSigner: true, isWritable: true },
    { pubkey: portfolioPDA, isSigner: false, isWritable: true },
    { pubkey: vaultPDA, isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ],
  programId: ROUTER_PROGRAM_ID,
  data: Buffer.from([2, ...amount_bytes]), // Deposit discriminator + amount
});
```

#### 2.2 Withdraw Collateral
Similar to deposit, but with margin checks:
- Verify: `vault_balance - withdraw_amount >= portfolio.im`
- Transfer SOL back to user
- Update vault balance

---

### Phase 3: Execute Cross-Slab Order (Week 2-3)

This is the **main feature** that proves portfolio margin efficiency.

#### 3.1 Order Splitting Algorithm

**Frontend Logic** (before sending transaction):
```typescript
async function splitOrder(
  symbol: string,
  side: 'buy' | 'sell',
  totalQty: number,
  pools: AMMPool[]
): Promise<SlabSplit[]> {
  const splits: SlabSplit[] = [];
  
  // Read QuoteCache from each pool
  for (const pool of pools) {
    const accountInfo = await connection.getAccountInfo(pool.address);
    const quoteCache = parseQuoteCache(accountInfo.data);
    
    // Calculate how much this pool can handle
    const availableQty = side === 'buy' 
      ? quoteCache.total_ask_qty()
      : quoteCache.total_bid_qty();
    
    const allocatedQty = Math.min(totalQty, availableQty);
    
    if (allocatedQty > 0) {
      splits.push({
        slab_id: pool.address,
        qty: allocatedQty,
        side: side === 'buy' ? 0 : 1,
        limit_px: calculateLimitPrice(quoteCache, side),
      });
      
      totalQty -= allocatedQty;
    }
    
    if (totalQty <= 0) break;
  }
  
  return splits;
}
```

#### 3.2 Execute Cross-Slab Instruction

```typescript
// Build ExecuteCrossSlab instruction
const data = Buffer.alloc(1 + splits.length * 41); // discriminator + splits
let offset = 0;

data.writeUInt8(4, offset); // ExecuteCrossSlab discriminator
offset += 1;

for (const split of splits) {
  split.slab_id.toBuffer().copy(data, offset);
  offset += 32;
  data.writeBigInt64LE(BigInt(split.qty), offset);
  offset += 8;
  data.writeUInt8(split.side, offset);
  offset += 1;
}

const accounts = [
  { pubkey: portfolioPDA, isSigner: false, isWritable: true },
  { pubkey: userPublicKey, isSigner: true, isWritable: false },
  { pubkey: vaultPDA, isSigner: false, isWritable: true },
  { pubkey: routerAuthorityPDA, isSigner: false, isWritable: false },
  // For each split:
  ...splits.flatMap(split => [
    { pubkey: split.slab_id, isSigner: false, isWritable: true },
    { pubkey: receiptPDA, isSigner: false, isWritable: true },
  ]),
];

const instruction = new TransactionInstruction({
  keys: accounts,
  programId: ROUTER_PROGRAM_ID,
  data,
});
```

#### 3.3 Portfolio Margin Calculation

**The Key Innovation**: Router calculates IM on **net exposure**, not gross.

Example:
```
User trades:
- Buy 10 ETH on Slab A
- Sell 8 ETH on Slab B
- Net exposure: +2 ETH

Traditional:
IM = (10 * $4000 * 10%) + (8 * $4000 * 10%) = $7,200

Portfolio Margin:
IM = (2 * $4000 * 10%) = $800

Capital Efficiency: 9x! 🚀
```

**Router Code** (from `execute_cross_slab.rs`):
```rust
// Calculate net exposure across all slabs
let net_exposure = calculate_net_exposure(portfolio);

// IM on net only!
let im_required = calculate_initial_margin(net_exposure, splits);

// Update portfolio
portfolio.update_margin(im_required, im_required / 2);

// Check margin
if !portfolio.has_sufficient_margin() {
    return Err(PercolatorError::PortfolioInsufficientMargin);
}
```

---

### Phase 4: Frontend Integration (Week 3-4)

#### 4.1 Update Dashboard UI

Add toggle for "Direct AMM" vs "Router Mode":
```tsx
const [tradingMode, setTradingMode] = useState<'direct' | 'router'>('direct');

// In AMM interface:
{tradingMode === 'router' ? (
  <RouterSwapInterface 
    pools={[solPool, ethPool, btcPool]}
    onSwap={handleRouterSwap}
  />
) : (
  <DirectAMMSwapInterface 
    pool={currentPool}
    onSwap={handleDirectSwap}
  />
)}
```

#### 4.2 Portfolio Display

Show user's portfolio status:
```tsx
<div className="portfolio-status">
  <div>Collateral: {vaultBalance} USDC</div>
  <div>IM Required: {portfolio.im} USDC</div>
  <div>Available: {vaultBalance - portfolio.im} USDC</div>
  <div>Net Exposure: {portfolio.net_exposure} contracts</div>
</div>
```

#### 4.3 Multi-Pool Order Form

```tsx
<div className="order-form">
  <input type="number" placeholder="Total Quantity" />
  
  <div className="pool-allocation">
    <h4>Optimal Split:</h4>
    {splits.map(split => (
      <div key={split.slab_id}>
        Pool {split.slab_id.slice(0,8)}: {split.qty} @ ${split.limit_px}
      </div>
    ))}
  </div>
  
  <button onClick={executeRouterTrade}>
    Execute Cross-Slab Trade
  </button>
</div>
```

---

## Testing Plan

### Unit Tests
```rust
// Test net exposure calculation
#[test]
fn test_portfolio_netting() {
    let mut portfolio = Portfolio::new();
    
    // Add long position
    portfolio.update_exposure(0, 0, 10_000_000); // +10 contracts
    
    // Add short position
    portfolio.update_exposure(1, 0, -8_000_000); // -8 contracts
    
    // Net should be +2
    let net = calculate_net_exposure(&portfolio);
    assert_eq!(net, 2_000_000);
}
```

### Integration Tests
```typescript
describe('Router Integration', () => {
  it('should execute cross-slab trade with margin efficiency', async () => {
    // 1. Initialize portfolio
    await initializePortfolio(user);
    
    // 2. Deposit collateral
    await deposit(user, 10_000 * 1e6); // 10k USDC
    
    // 3. Buy 10 ETH on pool A
    await executeCrossSlab(user, 'ETH', 'buy', 10);
    
    // 4. Sell 8 ETH on pool B
    await executeCrossSlab(user, 'ETH', 'sell', 8);
    
    // 5. Check margin
    const portfolio = await getPortfolio(user);
    expect(portfolio.im).toBeLessThan(1000 * 1e6); // Should be ~$800 for net +2 ETH
  });
});
```

---

## Key Files to Modify

### Backend (Rust)
- ✅ `programs/router/src/instructions/execute_cross_slab.rs` - Already implemented!
- ✅ `programs/router/src/instructions/initialize_portfolio.rs` - Already implemented!
- ✅ `programs/router/src/state/portfolio.rs` - Already implemented!
- ✅ `programs/router/src/state/vault.rs` - Already implemented!

### Scripts
- 📝 `scripts/initialize-portfolio.ts` - New
- 📝 `scripts/deposit-collateral.ts` - New
- 📝 `scripts/execute-router-trade.ts` - New

### Frontend
- 📝 `frontend/src/lib/router-client.ts` - New SDK wrapper
- 📝 `frontend/src/components/RouterSwapInterface.tsx` - New component
- 📝 `frontend/src/app/dashboard/page.tsx` - Add router mode toggle

---

## Migration Path from v0 to v1

### Step 1: Keep Both Modes Available
```tsx
// Let users choose:
// - "Simple Mode" (direct AMM) - current v0
// - "Advanced Mode" (Router) - new v1
```

### Step 2: Gradual Rollout
1. Deploy v1 features to devnet first
2. Test with small trades
3. Gradually increase limits
4. Full cutover when stable

### Step 3: Benefits Communication
Show users the capital efficiency gains:
```tsx
<ComparisonCard>
  <h3>Your Trade</h3>
  <div>Direct AMM: $7,200 margin required</div>
  <div>Router Mode: $800 margin required</div>
  <div className="savings">💰 Save $6,400 (89% reduction!)</div>
</ComparisonCard>
```

---

## Current Status

### ✅ Completed (v0)
- Router program deployed on devnet
- Registry initialized
- AMM pools initialized (SOL, ETH, BTC)
- Direct AMM swaps working in frontend

### 📝 TODO (v1)
- [ ] Portfolio initialization script
- [ ] Deposit/withdraw functionality
- [ ] Cross-slab execution frontend integration
- [ ] Portfolio margin display
- [ ] Multi-pool order splitting logic
- [ ] Testing suite for router integration
- [ ] User documentation for portfolio margin

---

## Estimated Timeline

**Total**: 3-4 weeks

- **Week 1**: Portfolio system + deposit/withdraw
- **Week 2**: Cross-slab execution backend
- **Week 3**: Frontend integration
- **Week 4**: Testing + polish

**Team Size**: 1-2 developers

---

## Resources

### Documentation
- Router program source: `programs/router/`
- AMM program source: `programs/amm/`
- Common types: `programs/common/`

### Deployed Addresses (Devnet)
- Router: `RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr`
- Registry: `DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx`
- SOL/USDC AMM: `FvxA93qPzDEGVdkP4PDE1gpXpt9R3u8gBgh9FTovsHJm`
- ETH/USDC AMM: `GPLmAVdfwE6zD1acgd5mZ7Myfq57oCQeJ9KGnco58YdQ`
- BTC/USDC AMM: `6vpuVH6SZX5a9PZSgMNBoZQAsTZtNJrEtd87RQCUHsPC`

### Test Commands
```bash
# Initialize portfolio
cd scripts && npx ts-node initialize-portfolio.ts

# Deposit collateral
npx ts-node deposit-collateral.ts --amount 10000

# Execute router trade
npx ts-node execute-router-trade.ts --symbol ETH --side buy --qty 5
```

---

## Contact & Support

For questions about v1 Router integration:
1. Review this document
2. Check existing Router program code in `programs/router/`
3. Reference AMM integration in `programs/amm/src/instructions.rs`

---

**Last Updated**: October 28, 2025  
**Version**: 1.0  
**Status**: Ready for implementation when v1 begins

