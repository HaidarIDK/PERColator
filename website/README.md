# Percolator Website

Dashboard with real-time charts powered by Hyperliquid API, ready for Solana program integration.

## Structure

```
website/
├── api/          # Fast Express API with Hyperliquid integration
└── frontend/     # Next.js dashboard with TradingView charts
```

## Quick Start

### API Server

```bash
cd website/api
npm install
npm run dev
```

API runs on `http://localhost:5001`

### Frontend

```bash
cd website/frontend
npm install
npm run dev
```

Frontend runs on `http://localhost:5000`

## Features

- ✅ Real-time charts for SOL/USDC, ETH/USDC, BTC/USDC
- ✅ Timeframes: 1min, 15min, 1hr, 12hr
- ✅ Historical data from Hyperliquid
- ✅ Live WebSocket updates
- ✅ Ready for Solana program integration (coming soon)

## Environment Variables

Copy `.env.example` to `.env` and configure:

**API:**
- `PORT` - API server port (default: 5001)
- `FRONTEND_URL` - Frontend URL for CORS

**Frontend:**
- `NEXT_PUBLIC_API_URL` - API server URL
- `NEXT_PUBLIC_WS_URL` - WebSocket URL

## Next Steps

Once charts are working, we'll integrate:
- Router program for trading
- Slab order books
- AMM liquidity pools
- Portfolio management

