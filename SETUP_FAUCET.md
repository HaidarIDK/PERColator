# 💰 Setup Devnet Faucet

## Why Faucet Didn't Work

The API needs Solana RPC configured to send real SOL airdrops.

## Quick Fix (1 minute)

### Create api/.env file

```bash
cd api
cp .env.example .env
```

Or manually create `api/.env` with this content:

```env
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com
PORT=3000
HOST=0.0.0.0
NODE_ENV=development
```

### Restart Backend

```bash
# Stop current backend (Ctrl+C in the terminal)
# Then restart:
cd api
npm run dev
```

You should now see:
```
✅ Solana connection initialized
📊 Network: devnet
🔗 RPC: https://api.devnet.solana.com
```

## Test Faucet

1. Go to: http://localhost:3001/dashboard
2. Connect your Phantom wallet
3. See the faucet banner at top
4. Click "Get 2 SOL"
5. Wait ~5 seconds
6. You'll get an alert with transaction signature!

## What the Faucet Does

```
User clicks button
     ↓
Frontend calls: POST /api/faucet/airdrop
     ↓
Backend calls: connection.requestAirdrop()
     ↓
Solana devnet sends 2 SOL
     ↓
Backend confirms transaction
     ↓
Returns signature + new balance
     ↓
Frontend shows success message
```

## If It Still Doesn't Work

### Rate Limit
The devnet faucet is rate-limited (1 request per minute per wallet).

**Solution:** Use the web faucet instead:
- Go to: https://faucet.solana.com
- Select "Devnet"
- Paste your wallet address
- Get 5 SOL per request

### RPC Not Available
If Solana devnet RPC is down or slow.

**Solution:** The error message will tell you to use web faucet.

## Alternative: Manual Airdrop

```bash
# In your terminal (WSL/Linux)
solana airdrop 2 YOUR_WALLET_ADDRESS

# Your wallet address from Phantom:
# Click on your wallet name → Copy Address
```

---

**After setting up .env and restarting, the faucet will work!** 💰

