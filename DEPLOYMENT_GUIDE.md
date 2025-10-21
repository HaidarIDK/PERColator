# 🚀 Deployment Guide

## Quick Deploy to Devnet

### Prerequisites
- Solana CLI installed
- ~10-15 devnet SOL

### Get Devnet SOL
Go to: **https://faucet.solana.com**
- Select "Devnet"
- Paste your address from: `solana address`
- Get 5 SOL (repeat 2-3 times)

### Deploy

```bash
# In WSL/Linux
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator
chmod +x deploy-quick.sh
./deploy-quick.sh
```

This will:
1. Build router & slab programs
2. Deploy to devnet (~4 SOL)
3. Save Program IDs to `deployment-ids.txt`

### Create Slab Account

```bash
# Generate keypair
solana-keygen new --outfile slab-account.json --no-bip39-passphrase

# Get address
SLAB_ADDR=$(solana address -k slab-account.json)
SLAB_PROGRAM_ID="<from deployment-ids.txt>"

# Create 60 KB account (costs only 0.5 SOL!)
solana create-account $SLAB_ADDR 60000 $SLAB_PROGRAM_ID \
  --from ~/.config/solana/id.json \
  --keypair slab-account.json
```

### Update API Config

Edit `api/.env`:
```env
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com
ROUTER_PROGRAM_ID=<from deployment-ids.txt>
SLAB_PROGRAM_ID=<from deployment-ids.txt>
SLAB_ACCOUNT=<slab address>
```

### Restart

```bash
cd api && npm run dev
```

## Costs

- Router deploy: ~2 SOL
- Slab deploy: ~2 SOL
- Slab account rent: **0.5 SOL** (vs 73 SOL original!)
- **Total: ~5 SOL**

## Slab Size

- **60 KB** (vs 10 MB original)
- **0.5 SOL rent** (saved 72.5 SOL!)
- Same size as Serum/OpenBook
- Supports: 50 users, 300 orders, 100 positions

---

That's it! Simple and cheap. 🎉

