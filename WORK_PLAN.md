# Percolator Router Program - Implementation Plan

## Current Status

**Completed:**
- Spot trading slab (10MB state, order matching, reserve/commit)
- Anti-toxicity enforcement (batch windows, freeze, JIT penalty, kill band, ARG)
- 38 passing tests
- API server for frontend integration

**Today's Work: Router Program + Capability Tokens**

Target: Devnet deployment at https://solscan.io/?cluster=devnet

---

## Architecture Overview

### Router Program (Global Aggregator)
Controls collateral, portfolio margin, and coordinates trades across multiple slabs.

```
Router Program
├── State Accounts
│   ├── Vault[mint]              - Custody of assets (USDC, etc.)
│   ├── Escrow(user,slab,mint)   - Per-user per-slab collateral pledges
│   ├── Cap(route_id)            - Capability tokens for scoped debits
│   ├── Portfolio(user)          - Cross-slab positions & margin
│   └── Registry                 - Whitelist of approved slabs
└── Instructions
    ├── reserve_multi            - Reserve across multiple slabs
    ├── commit_multi             - Atomic commit across slabs
    ├── cancel_reservation       - Cancel and refund
    ├── deposit_collateral       - Add collateral to vault
    ├── withdraw_collateral      - Remove collateral from vault
    ├── register_slab            - Add slab to whitelist (governance)
    └── liquidate                - Liquidation coordinator
```

### Security Model: Capability Tokens

**Key Invariant:** Slabs CANNOT access Router vaults directly.

**Flow:**
1. User requests trade via Router
2. Router calls `slab.reserve()` -> gets `{hold_id, max_charge}`
3. Router creates `Escrow(user, slab, mint)` with `max_charge` amount
4. Router mints `Cap(scope=(user,slab,mint), amount_max=max_charge, expiry=now+120s)`
5. Router calls `slab.commit(hold_id, cap)`
6. Slab can ONLY debit this specific escrow, up to `cap.remaining`, before `cap.expiry`
7. After commit or expiry, cap is burned

**Security Boundary:**
- Malicious slab can only affect users who explicitly routed to it
- Debit limited to pre-approved cap amount
- Time-limited (2 minute expiry)
- Scope-locked to (user, slab, mint) tuple

---

## Data Structures

### Vault Account
```rust
pub struct Vault {
    pub authority: Pubkey,      // Router program authority
    pub mint: Pubkey,            // Token mint (USDC, etc.)
    pub bump: u8,
}
```

### Escrow Account (PDA per user/slab/mint)
```rust
pub struct Escrow {
    pub user: Pubkey,
    pub slab: Pubkey,
    pub mint: Pubkey,
    pub balance: u128,
    pub nonce: u64,              // Anti-replay
    pub bump: u8,
}
```

### Capability Token
```rust
pub struct Cap {
    pub route_id: u64,
    pub scope_user: Pubkey,
    pub scope_slab: Pubkey,
    pub mint: Pubkey,
    pub amount_max: u128,
    pub remaining: u128,
    pub expiry_ts: i64,
    pub nonce: u64,
    pub burned: bool,
    pub bump: u8,
}
```

### Portfolio Account
```rust
pub struct Portfolio {
    pub user: Pubkey,
    pub equity: i128,            // Total equity across all slabs
    pub im: u128,                // Initial margin required
    pub mm: u128,                // Maintenance margin
    pub free_collateral: i128,   // Available for new trades
    pub last_mark_ts: i64,
    pub bump: u8,
}

// Stored separately, linked to portfolio
pub struct Exposure {
    pub portfolio: Pubkey,
    pub slab: Pubkey,
    pub instrument_idx: u16,
    pub qty: i128,               // Signed: +long, -short
    pub entry_px: u64,
}
```

### Slab Registry
```rust
pub struct SlabRegistry {
    pub authority: Pubkey,        // Governance
    pub slabs: Vec<RegisteredSlab>,
}

pub struct RegisteredSlab {
    pub slab_id: Pubkey,
    pub version_hash: [u8; 32],   // Code hash for verification
    pub oracle_id: Pubkey,
    pub imr: u16,                 // Risk params
    pub mmr: u16,
    pub max_fee_bps: u16,
    pub enabled: bool,
}
```

---

## Instructions Implementation Order

### Phase 1: Core Infrastructure
1. **initialize_router** - Set up router authority, registry
2. **initialize_vault** - Create vault for asset (USDC)
3. **deposit_collateral** - User deposits into vault
4. **withdraw_collateral** - User withdraws from vault

### Phase 2: Capability System
5. **create_escrow** - Create escrow PDA
6. **mint_cap** - Mint capability token with scope/expiry
7. **burn_cap** - Burn capability after use
8. **verify_cap** - Helper for slab debit checks

### Phase 3: Multi-Slab Routing
9. **reserve_multi** - Reserve across N slabs
10. **commit_multi** - Atomic commit or rollback
11. **cancel_reservation** - Cancel holds, refund escrow

### Phase 4: Portfolio & Risk
12. **update_portfolio** - Recalculate equity/IM/MM
13. **register_slab** - Add slab to whitelist (governance)
14. **liquidate** - Coordinated liquidation

---

## Devnet Deployment Plan

### Step 1: Build Router Program
```bash
cd programs/router
anchor build
```

### Step 2: Deploy to Devnet
```bash
solana config set --url https://api.devnet.solana.com
anchor deploy --provider.cluster devnet
```

### Step 3: Initialize Router
```bash
# Create vault for USDC
anchor run initialize-router-devnet
```

### Step 4: Deploy Slab (if needed)
```bash
cd programs/slab
anchor deploy --provider.cluster devnet
```

### Step 5: Register Slab with Router
```bash
anchor run register-slab-devnet
```

### Step 6: Test End-to-End
- Deposit collateral
- Reserve across slab
- Commit trade
- Verify capability enforcement

---

## API Endpoints to Add

After Router deployment, extend API with:

```
POST /api/router/deposit
POST /api/router/withdraw
POST /api/router/reserve-multi
POST /api/router/commit-multi
POST /api/router/cancel
GET  /api/router/portfolio/{user}
GET  /api/router/slabs
```

---

## Testing Strategy

### Unit Tests (Per Instruction)
- Cap minting with correct scope/expiry
- Cap expiry rejection
- Escrow debit bounds (never exceed cap.remaining)
- Multi-slab atomic commit/rollback

### Integration Tests
- Malicious slab tries to debit wrong escrow (should fail)
- Cap expired during commit (should fail)
- Partial fills across multiple slabs
- Liquidation across slabs

### Devnet Tests
- Real transaction flow
- Multiple users trading
- Cross-slab routing efficiency

---

## Security Checklist

- [ ] Caps enforce (user, slab, mint) scope
- [ ] Caps enforce amount_max limit
- [ ] Caps enforce expiry timestamp
- [ ] Slabs cannot CPI to vaults
- [ ] Escrow debits require valid cap
- [ ] Nonce prevents replay attacks
- [ ] Authority checks on governance functions
- [ ] PDA derivations secure

---

## Success Criteria

**Router v1 Complete When:**
1. Can deposit/withdraw collateral
2. Can route trades to single slab via caps
3. Caps enforce all security boundaries
4. Deployed and verified on devnet
5. API endpoints functional
6. Frontend can execute trades via Router

**Future (Router v2):**
- Multi-slab atomic routing
- Portfolio cross-margin
- Liquidation coordination
- Funding rate aggregation

---

## File Structure

```
programs/router/
├── Cargo.toml
├── Xargo.toml
└── src/
    ├── lib.rs
    ├── entrypoint.rs
    ├── error.rs
    ├── state/
    │   ├── mod.rs
    │   ├── vault.rs
    │   ├── escrow.rs
    │   ├── cap.rs
    │   ├── portfolio.rs
    │   └── registry.rs
    ├── instructions/
    │   ├── mod.rs
    │   ├── initialize.rs
    │   ├── deposit.rs
    │   ├── withdraw.rs
    │   ├── cap_ops.rs
    │   ├── reserve.rs
    │   ├── commit.rs
    │   └── liquidate.rs
    └── pda.rs
```

---

## Notes

- All amounts in u128 to prevent overflow
- Time values in i64 Unix timestamps
- PDAs use proper seeds for derivation
- No heap allocations in slab commits (capability check must be O(1))
- Cap nonce prevents double-spend
- Escrow balance must never go negative

