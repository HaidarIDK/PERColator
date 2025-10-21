# PERColator - Perpetuals DEX

Ultra-cheap Solana perpetuals exchange with 0.5 SOL rent per market!

## Quick Start

### 1. Start Servers
```bash
# Backend (Terminal 1)
cd api && npm run dev

# Frontend (Terminal 2) 
cd frontend && npm run dev
```

### 2. Open Dashboard
http://localhost:3001/dashboard

### 3. Monitor Transactions
http://localhost:3001/monitor

## Key Stats

- **Slab Size:** 60 KB (vs 10 MB original)
- **Rent Cost:** 0.5 SOL (vs 73 SOL original)
- **Savings:** 72.5 SOL per market!
- **Capacity:** 50 users, 300 orders, 100 positions
- **Status:** Ready for beta testing

## Deploy to Devnet

```bash
# Get devnet SOL from https://faucet.solana.com
# Then run:
chmod +x deploy-quick.sh
./deploy-quick.sh
```

Cost: ~5 SOL total (FREE on devnet!)

## Files

- `api/` - Backend API server
- `frontend/` - Next.js dashboard
- `programs/` - Rust smart contracts
- `deploy-quick.sh` - Automated deployment
- `TODAY_PLAN.md` - Current work plan
- `DEPLOYMENT_GUIDE.md` - Full deployment steps

## What's Working

✅ Real-time price charts (ETH, BTC, SOL)
✅ Order book visualization
✅ Wallet integration (Phantom)
✅ Transaction monitoring
✅ WebSocket updates
✅ All 140 tests passing

## What's Next

See `TODAY_PLAN.md` for UI completion tasks.

---

**Forked from:** Toly's Percolator
**Optimizations:** 60 KB slab design (0.5 SOL rent!)

