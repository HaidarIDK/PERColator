# Percolator API - Vercel Deployment Guide

## Prerequisites
- Vercel account
- Vercel CLI installed: `npm i -g vercel`

## Deployment Steps

### 1. Install Dependencies
```bash
cd website/api
npm install
```

### 2. Set Up Environment Variables
Copy `.env.example` to `.env.local` and configure:
```bash
cp .env.example .env.local
```

Required environment variables:
- `SOLANA_RPC_URL` - Your Solana RPC endpoint
- `SOLANA_NETWORK` - Network (mainnet-beta, devnet, testnet)
- `FRONTEND_URL` - Your frontend deployment URL (for CORS)
- `PORT` - Server port (default: 5001)

### 3. Test Locally
```bash
npm run dev
```

Visit http://localhost:5001 to verify the API is running.

### 4. Deploy to Vercel
```bash
vercel --prod
```

Or use the Vercel dashboard:
1. Go to https://vercel.com/new
2. Import the `website/api` directory
3. Configure environment variables in the Vercel dashboard
4. Deploy

### 5. Configure Environment Variables in Vercel
In the Vercel dashboard, add these environment variables:
- `SOLANA_RPC_URL`
- `SOLANA_NETWORK`
- `FRONTEND_URL`
- `NODE_ENV=production`

## API Endpoints
- Health check: `/api/health`
- Market data: `/api/market/*`
- Trading: `/api/trade/*`
- User data: `/api/user/*`
- Full list: See root endpoint `/`

## Notes
- WebSocket connections may require additional Vercel configuration
- Ensure your Solana RPC endpoint is accessible from Vercel's infrastructure
- The API runs in serverless mode on Vercel