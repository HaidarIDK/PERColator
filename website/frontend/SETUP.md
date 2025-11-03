# Frontend Setup Guide

## Quick Start

### 1. Install Dependencies

```bash
cd website/frontend
npm install
```

### 2. Configure Environment Variables

Create a `.env.local` file in the `website/frontend/` directory:

```bash
# Copy the deployed addresses from your private/slab-account.json and private/router-info.json

# Slab Program
NEXT_PUBLIC_SLAB_PROGRAM_ID=SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep
NEXT_PUBLIC_SLAB_ACCOUNT=5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB
NEXT_PUBLIC_INSTRUMENT_ID=G4Um9dNaWKDwd2bhLTEX3DCLRLVWixKvZ1WdEcq6pgfN

# Router Program
NEXT_PUBLIC_ROUTER_PROGRAM_ID=RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr
NEXT_PUBLIC_ROUTER_REGISTRY=DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx

# Authority
NEXT_PUBLIC_AUTHORITY=pErcnK9NSVQQK54BsKV4tUt8YWiKngubpJ7jxHrFtvL

# Network
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com

# API (backend server)
NEXT_PUBLIC_API_URL=http://localhost:3000
```

### 3. Start Development Server

```bash
npm run dev
```

The frontend will be available at `http://localhost:3001`

## Features

### Trading Interface (`/trade`)

- **Connect Wallet**: Click "Select Wallet" in the top right to connect Phantom, Solflare, or any Solana wallet
- **View Orderbook**: See live orders from the Slab account on-chain
- **Place Orders**: Use the Reserve-Commit workflow:
  1. **Reserve**: Lock liquidity at current price
  2. **Commit**: Execute the reserved trade

### How Transactions Work

1. **User connects wallet** using Solana Wallet Adapter
2. **Frontend fetches orderbook** from your deployed Slab account
3. **User places order**:
   - Frontend calls your backend API (`/api/trade/reserve`)
   - Backend builds a transaction with Slab program instructions
   - Frontend signs the transaction with user's wallet
   - Transaction is sent to Solana devnet
4. **User commits trade**:
   - Frontend calls backend API (`/api/trade/commit`) 
   - Backend builds commit transaction
   - User signs and broadcasts

## Architecture

```
┌─────────────┐         ┌──────────────┐         ┌─────────────────┐
│   Browser   │ ◄──────►│   Frontend   │ ◄──────►│  Backend API    │
│  (Wallet)   │  Signs  │  (Next.js)   │  Build  │   (Node.js)     │
└─────────────┘   TX    └──────────────┘   TX    └─────────────────┘
                                                           │
                                                           ▼
                                                  ┌─────────────────┐
                                                  │ Solana Programs │
                                                  │  Slab + Router  │
                                                  └─────────────────┘
```

## Program Addresses

All program addresses are configured in `src/lib/program-config.ts` and loaded from environment variables.

### Current Deployment (v0 - Devnet)

- **Slab Program**: `SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep`
- **Slab Account**: `5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB`
- **Router Program**: `RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr`
- **Router Registry**: `DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx`

View these accounts on:
- [Solana Explorer](https://explorer.solana.com)
- [Solscan](https://solscan.io)

## Troubleshooting

### "Failed to fetch orderbook"
- Make sure your backend API is running (`npm start` in `api/` directory)
- Check that `NEXT_PUBLIC_API_URL` points to your backend

### "Connect wallet first"
- Install [Phantom Wallet](https://phantom.app/) browser extension
- Switch to **Devnet** in wallet settings
- Click "Select Wallet" button to connect

### "Insufficient SOL balance"
- Get devnet SOL from [Solana Faucet](https://faucet.solana.com/)
- Your wallet address is shown after connecting

### Transaction fails
- Check transaction logs in browser console
- View transaction on [Solscan](https://solscan.io) (link shown after transaction)
- Make sure you're on devnet, not mainnet

## Development

### Key Files

- `src/app/trade/page.tsx` - Main trading interface
- `src/lib/program-config.ts` - Program addresses and configuration
- `src/components/WalletProvider.tsx` - Solana wallet integration

### Adding New Features

1. Update `program-config.ts` with any new addresses
2. Create components in `src/components/`
3. Add pages in `src/app/`
4. Use Solana wallet hooks: `useWallet()`, `useConnection()`

## Deployment

### Production Build

```bash
npm run build
npm start
```

### Deploy to Vercel/Render

1. Connect your Git repository
2. Set environment variables in platform dashboard
3. Build command: `npm run build`
4. Start command: `npm start`

## Resources

- [Solana Web3.js Docs](https://solana-labs.github.io/solana-web3.js/)
- [Wallet Adapter Docs](https://github.com/solana-labs/wallet-adapter)
- [Next.js Docs](https://nextjs.org/docs)

