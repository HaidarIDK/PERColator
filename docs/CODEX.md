# Percolator Codebase Codex

> This file contains essential context, patterns, and information about the Percolator codebase for AI assistants and developers.

## Table of Contents
- [Project Overview](#project-overview)
- [Project Structure](#project-structure)
- [Technology Stack](#technology-stack)
- [Key Patterns & Conventions](#key-patterns--conventions)
- [Build & Deployment](#build--deployment)
- [API Structure](#api-structure)
- [Frontend Structure](#frontend-structure)
- [Backend Structure](#backend-structure)
- [Important Context](#important-context)

---

## Project Overview

**Percolator** is a Solana-based spot trading DEX (Decentralized Exchange) that supports:
- Orderbook-based trading via Slab
- AMM (Automated Market Maker) liquidity pools
- Cross-margining and portfolio management
- Insurance fund and crisis management
- Real-time market data and charting

**Repository**: Fork of `https://github.com/aeyakovenko/percolator`

---

## Project Structure

```
percolator/
├── website/                    # Frontend and API
│   ├── frontend/              # Next.js frontend application
│   │   ├── src/
│   │   │   ├── app/           # Next.js app router pages
│   │   │   ├── components/    # React components
│   │   │   └── lib/           # Utilities, API clients, configs
│   │   └── public/             # Static assets
│   └── api/                   # Express.js REST API server
│       └── src/
│           ├── routes/        # API route handlers
│           └── services/      # Business logic services
├── programs/                  # Solana on-chain programs (Rust)
│   ├── slab/                  # Orderbook program
│   ├── router/                # Router program (cross-margin)
│   └── amm/                   # AMM program
├── scripts/                   # TypeScript deployment/init scripts
├── test-scripts/              # E2E test shell scripts
├── private/                   # Sensitive files (keypairs, etc.) - gitignored
├── docs/                      # Documentation
└── .gitignore                 # Root gitignore (comprehensive)
```

---

## Technology Stack

### Frontend (`website/frontend/`)
- **Framework**: Next.js 14+ (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **UI Library**: Radix UI, Lucide Icons, React Icons
- **Charts**: Lightweight Charts, TradingView widgets
- **State**: React Hooks (useState, useEffect)
- **Wallet**: @solana/wallet-adapter-react
- **Animations**: Framer Motion (motion/react)

### API (`website/api/`)
- **Framework**: Express.js
- **Language**: TypeScript
- **Runtime**: Node.js with tsx
- **WebSocket**: ws library
- **CORS**: Enabled for frontend origins

### Backend (`programs/`)
- **Language**: Rust
- **Framework**: Anchor (Solana program framework)
- **Build**: Cargo + Solana BPF tools
- **Target**: Solana BPF (Berkeley Packet Filter)

### Development
- **Package Manager**: npm
- **Type Checking**: TypeScript
- **Linting**: ESLint
- **Test Runner**: Vitest (API), Shell scripts (E2E)

---

## Key Patterns & Conventions

### File Naming
- **React Components**: PascalCase (e.g., `OrderBook.tsx`)
- **Hooks**: camelCase with `use` prefix (e.g., `useMarketData.ts`)
- **Utilities**: camelCase (e.g., `api-client.ts`)
- **Pages**: Next.js App Router convention (`page.tsx`)
- **Routes**: camelCase (e.g., `trading.ts`)

### Code Organization
- **Components**: Split by feature/domain (e.g., `components/trading/`, `components/charts/`)
- **Lib Files**: Single responsibility (e.g., `api-client.ts`, `program-config.ts`)
- **Pages**: One main component per page, extract sub-components for reusability

### State Management
- **Frontend**: React hooks (useState, useEffect) - no global state library
- **API State**: In-memory for WebSocket connections, database for persistence (if any)

### Error Handling
- **Frontend**: Try-catch blocks with user-friendly toast notifications
- **API**: Express error middleware
- **Solana**: Transaction error handling with error parsing

### API Communication
- **REST**: `apiClient` instance from `@/lib/api-client`
- **WebSocket**: `ServerWebSocketClient` class for real-time data
- **Base URL**: `http://localhost:5001` (default) or `NEXT_PUBLIC_API_URL` env var

---

## Build & Deployment

### Frontend
```bash
cd website/frontend
npm install
npm run dev          # Dev server on port 5000
npm run build        # Production build
npm start            # Production server on port 3000
```

### API
```bash
cd website/api
npm install
npm run dev          # Dev server with hot reload (tsx watch)
npm run build        # TypeScript compilation
npm start            # Production server
```

### Solana Programs
```bash
# Build all programs
cargo build-sbf

# Deploy to localnet
make deploy

# Or use scripts
./scripts/deploy.sh
```

### Environment Variables
- **Frontend**: `.env.local` (NEXT_PUBLIC_API_URL, NEXT_PUBLIC_WS_URL, etc.)
- **API**: `.env` (PORT, HOST, RPC_URL, etc.)
- **All `.env*` files are gitignored**

---

## API Structure

### Base URL
- **Development**: `http://localhost:5001`
- **WebSocket**: `ws://localhost:5001/ws`
- **Production**: Set via `NEXT_PUBLIC_API_URL` env var

### Main Routes (`website/api/src/routes/`)
- **`health.ts`**: Health check endpoint
- **`marketData.ts`**: Market data and charts
- **`dashboard.ts`**: Dashboard data
- **`trading.ts`**: Trading operations
- **`user.ts`**: User portfolio and positions
- **`slab.ts`**: Slab orderbook operations
- **`router.ts`**: Router program operations
- **`faucet.ts`**: Devnet faucet
- **`claims.ts`**: Claims processing
- **`monitor.ts`**: System monitoring

### Services (`website/api/src/services/`)
- **`solana.ts`**: Solana connection and program initialization
- **`transactions.ts`**: Transaction building and signing helpers
- **`websocket-server.ts`**: WebSocket server management
- **`router.ts`**: Router program service
- **`hyperliquid-websocket.ts`**: External market data integration

### WebSocket Protocol
- **Connection**: `ws://localhost:5001/ws`
- **Message Format**: JSON
- **Subscriptions**: `{type: "subscribe", symbol: "BTC", interval: "15m"}`
- **Data**: `{type: "candle", data: {...}}`

---

## Frontend Structure

### Pages (`website/frontend/src/app/`)
- **`dashboard/page.tsx`**: Main trading dashboard (4,371 lines - needs refactoring!)
- **`trade/page.tsx`**: Simple trading interface
- **`portfolio/page.tsx`**: User portfolio view
- **`monitor/page.tsx`**: System monitoring
- **`dex/page.tsx`**: DEX interface
- **`info/page.tsx`**: Information page
- **`v0/page.tsx`**: Version 0 interface

### Components (`website/frontend/src/components/`)
- **`ui/`**: Reusable UI primitives (Radix UI based)
  - `animated-beam.tsx`, `aurora-text.tsx`, `custom-chart.tsx`, etc.
- **`TransactionStatus.tsx`**: Transaction status display
- **`WalletProvider.tsx`**: Solana wallet context provider

### Lib (`website/frontend/src/lib/`)
- **`api-client.ts`**: REST and WebSocket API client (`PercolatorAPIClient`, `ServerWebSocketClient`)
- **`program-config.ts`**: Solana program IDs, network config, API URLs
- **`data-service.ts`**: Data fetching and transformation utilities
- **`amm-client.ts`**: AMM pool operations client
- **`utils.ts`**: General utilities (cn function for Tailwind, etc.)

### Key Files to Know
- **`dashboard/page.tsx`**: Contains 8+ inline components that should be extracted:
  - `LightweightChart` (lines ~54-518)
  - `TradingViewChartComponent` (lines ~522-792)
  - `OrderBook` (lines ~924-1002)
  - `CrossSlabTrader` (lines ~2088-3075)
  - `OrderForm` (lines ~3076-3484)
  - And more...

---

## Backend Structure

### Solana Programs (`programs/`)

#### Slab Program (`programs/slab/`)
- **Purpose**: Orderbook-based trading
- **Key Instructions**:
  - `place_order`: Place limit/market orders
  - `modify_order`: Modify existing orders
  - `cancel_order`: Cancel orders
  - `commit_fill`: Commit a filled order
  - `halt_trading`: Emergency halt
  - `resume_trading`: Resume after halt
  - `update_funding`: Update funding rates

#### Router Program (`programs/router/`)
- **Purpose**: Cross-margining and portfolio management
- **Key Features**:
  - Cross-slab trading (execute trades across multiple slabs)
  - Portfolio-level margin management
  - LP (Liquidity Provider) operations

#### AMM Program (`programs/amm/`)
- **Purpose**: Automated Market Maker pools
- **Key Features**:
  - Swap operations
  - LP add/remove liquidity
  - Price calculation via constant product formula

### Program IDs
- Stored in `website/frontend/src/lib/program-config.ts`
- Also in `private/` folder (gitignored) for deployment keypairs

---

## Important Context

### Git Structure
- **Root `.gitignore`**: Comprehensive - covers Rust, Node, TypeScript, Next.js globally
- **`website/api/.gitignore`**: Only `/dist/` (API build output)
- **`website/frontend/.gitignore`**: Next.js-specific patterns (`/.next/`, `/out/`)
- **`private/` folder**: Gitignored (contains keypairs and sensitive configs)

### Test Scripts
- **Location**: `test-scripts/` folder (moved from root)
- **Purpose**: E2E integration tests for Solana programs
- **Run**: `./test-scripts/run_all_tests.sh` or individual scripts

### Keypair Management
- **Location**: `private/` folder (gitignored)
- **Contains**: Solana program keypairs, deployment keys
- **Security**: Never commit to git

### Port Configuration
- **Frontend**: Port 5000 (dev), 3000 (prod)
- **API**: Port 5001 (default, configurable via env)
- **WebSocket**: Same as API (ws://localhost:5001/ws)

### Known Issues & TODOs
1. **Dashboard component too large** (4,371 lines) - needs refactoring
2. **108+ console.log statements** - should be removed or use proper logging
3. **Multiple inline components** - should be extracted to separate files
4. **Documentation files** - could be organized into `docs/` folder

### Development Workflow
1. **Start Solana localnet**: `solana-test-validator`
2. **Start API**: `cd website/api && npm run dev`
3. **Start Frontend**: `cd website/frontend && npm run dev`
4. **Deploy Programs**: `make deploy` or use deployment scripts

### Network Configuration
- **Default**: Devnet
- **RPC URL**: `http://localhost:8899` (localnet) or Solana devnet RPC
- **Explorer**: Solana Explorer (configurable in `program-config.ts`)

---

## Adding Context to This File

When adding new features or making significant changes:

1. **Update this file** with:
   - New patterns or conventions
   - New folder structures
   - Important architectural decisions
   - Key context that might not be obvious from code

2. **Keep it organized**:
   - Use clear sections
   - Add examples where helpful
   - Keep file paths accurate

3. **Keep it current**:
   - Remove outdated information
   - Update when refactoring major components
   - Note breaking changes

---

## Quick Reference

### Common Tasks

**Start development environment:**
```bash
# Terminal 1: Start Solana
solana-test-validator

# Terminal 2: Start API
cd website/api && npm run dev

# Terminal 3: Start Frontend
cd website/frontend && npm run dev
```

**Build and deploy programs:**
```bash
cargo build-sbf
make deploy
```

**Run tests:**
```bash
./test-scripts/run_all_tests.sh
```

**Check git status:**
```bash
git status
# Note: private/ folder won't show (gitignored)
```

---

*Last Updated: [Auto-update this when making significant changes]*

