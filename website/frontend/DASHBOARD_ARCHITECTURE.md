# 📊 Dashboard Architecture & Data Flow

## 🎯 **What You Have**

### **Frontend Framework**
- **Next.js 15.5.6** (React 19.1.0)
- **TypeScript** with strict typing
- **Tailwind CSS 4** for styling
- **Motion/Framer Motion** for animations
- **Lightweight Charts** for candlestick charts

### **Location**
📁 **`frontend/src/app/dashboard/page.tsx`** (4,598 lines - your main dashboard)

---

## 🔌 **API Configuration**

### **Base URLs** (in `frontend/src/lib/api-client.ts`)
```typescript
API_URL: 'http://localhost:5001'  // Your backend
WS_URL: 'ws://localhost:5001/ws'  // WebSocket connection
```

**Environment Variables** (can override):
- `NEXT_PUBLIC_API_URL` → Backend API base URL
- `NEXT_PUBLIC_WS_URL` → WebSocket URL
- `NEXT_PUBLIC_SERVER_WS_URL` → Server WebSocket URL

---

## 📡 **API Endpoints Used**

### **1. Market Data APIs**
```typescript
GET /api/market/list
  → Get all available markets
  → Returns: MarketData[]

GET /api/market/{symbol}
  → Get market data for specific symbol (e.g., "SOL-PERP", "ETH-PERP")
  → Returns: MarketData { price, change24h, volume24h, fundingRate, etc. }

GET /api/market/{symbol}/orderbook
  → Get orderbook for symbol
  → Returns: Orderbook { bids[], asks[] }

GET /api/market/{symbol}/candles
  → Get historical candlestick data
  → Query params: timeframe, limit, from, to
  → Returns: CandlestickData[]
```

### **2. Slab (On-Chain) APIs**
```typescript
GET /api/slab-live/orderbook
  → Get REAL Solana Slab orderbook data
  → Returns: { success, orderbook: { bids, asks }, recentTrades }

GET /api/slab-live/transactions?limit=50
  → Get recent Slab transactions
  → Returns: { success, transactions[] }
```

### **3. Router APIs**
```typescript
GET /api/router/slabs?coin={ethereum|bitcoin|solana}
  → Get available slabs for cross-slab trading
  → Returns: { slabs[] }

POST /api/router/execute-cross-slab
  → Execute cross-slab trade
  → Body: { ...tradeData }
```

### **4. User Portfolio APIs**
```typescript
GET /api/user/{walletAddress}/portfolio
  → Get user's portfolio data
  → Returns: UserPortfolio { equity, freeCollateral, positions[], etc. }

GET /api/user/{walletAddress}/positions
  → Get user's open positions
  → Returns: Position[]
```

### **5. Trading APIs**
```typescript
POST /api/trade/reserve
  → Reserve collateral for trade
  → Body: { symbol, side, size, price, ... }

POST /api/trade/commit
  → Commit to a trade (execute)
  → Body: { reservationId, ... }
```

### **6. Faucet APIs**
```typescript
GET /api/faucet/info
  → Get faucet information
  → Returns: FaucetInfo { isAvailable, amountPerClaim, cooldownSeconds }

POST /api/faucet/claim
  → Claim tokens from faucet
  → Body: { walletAddress }

POST /api/faucet/airdrop
  → Request airdrop
  → Body: { walletAddress, amount }
```

### **7. Health Check**
```typescript
GET /api/health
  → Check API server status
  → Returns: { status, timestamp, ... }
```

---

## 🔄 **WebSocket Connections**

### **1. Server WebSocket** (Primary - Recommended)
**Connection:** `ws://localhost:5001/ws`

**Location:** `frontend/src/lib/api-client.ts` → `ServerWebSocketClient` class

**Message Format:**
```typescript
// Subscribe to candles
{
  type: "subscribe",
  symbol: "SOL",  // or "ETH", "BTC"
  interval: "15m" // or "1h", "4h", "1d"
}

// Unsubscribe
{
  type: "unsubscribe",
  symbol: "SOL",
  interval: "15m"
}
```

**Received Messages:**
```typescript
{
  type: "candle",
  subscriptionId: "...",
  data: {
    symbol: "SOL",
    timeframe: "15m",
    timestamp: 1234567890,
    open: 185.0,
    high: 186.5,
    low: 184.0,
    close: 185.5,
    volume: 1000,
    priceChange: 0.5,
    priceChangePercent: 0.27
  }
}
```

**Usage in Dashboard:**
- Real-time candlestick updates
- Price change notifications
- Auto-reconnect with exponential backoff
- Multiple subscription management

**Code Location:** 
- Dashboard: Lines 236-373, 725-780 (in `LightweightChart` component)
- API Client: Lines 145-311 (`ServerWebSocketClient` class)

### **2. Standard WebSocket** (Legacy/Fallback)
**Connection:** `ws://localhost:5001/ws` (same URL, different protocol)

**Usage:**
- Market data subscriptions
- Orderbook updates
- Fallback if ServerWebSocket fails

**Code Location:**
- Dashboard: Lines 591-639 (`connectPriceWebSocket` function)
- API Client: Lines 435-463 (`connectWebSocket` method)

---

## 📁 **File Structure & Code Locations**

### **Main Dashboard File**
📄 **`frontend/src/app/dashboard/page.tsx`** (4,598 lines)
- **Main export:** Default dashboard component
- **Components inside:**
  - `LightweightChart` (lines 54-518) - Candlestick chart with WebSocket
  - `TradingViewChartComponent` (lines 522-792) - Alternative chart view
  - `OrderBook` (lines 925-1002) - Orderbook display
  - `CrossSlabTrader` (lines 2088-3075) - Cross-slab trading UI
  - `OrderForm` (lines 3076-3484) - Portfolio-based trading form
  - Main dashboard layout (lines 3485+)

### **API Client Library**
📄 **`frontend/src/lib/api-client.ts`** (504 lines)
- **Exports:**
  - `apiClient` - Singleton instance
  - Types: `MarketData`, `Orderbook`, `Position`, `UserPortfolio`, etc.
- **Classes:**
  - `ServerWebSocketClient` - WebSocket connection manager
  - `PercolatorAPIClient` - REST API client

### **Program Configuration**
📄 **`frontend/src/lib/program-config.ts`**
- Solana program IDs
- Network configuration
- API base URLs
- Explorer URLs

### **Data Service**
📄 **`frontend/src/lib/data-service.ts`**
- Helper functions for data processing
- Chart data transformations

---

## 🔄 **Data Flow in Dashboard**

### **1. Initial Page Load**
```
Dashboard mounts
  ↓
Fetch market data: GET /api/market/list
  ↓
Fetch orderbook: GET /api/slab-live/orderbook
  ↓
Fetch transactions: GET /api/slab-live/transactions
  ↓
Connect WebSocket: ws://localhost:5001/ws
  ↓
Subscribe to candle updates: { type: "subscribe", symbol: "SOL", interval: "15m" }
  ↓
Load historical chart data: GET /api/market/{symbol}/candles
```

### **2. Real-Time Updates Flow**
```
WebSocket receives candle data
  ↓
Parse message: { type: "candle", data: {...} }
  ↓
Update chart: chart.update(newCandle)
  ↓
Update price display
  ↓
Calculate price change %
```

### **3. Trading Flow**
```
User submits trade in OrderForm
  ↓
Check wallet connection
  ↓
Fetch portfolio: GET /api/user/{wallet}/portfolio
  ↓
Reserve collateral: POST /api/trade/reserve
  ↓
Execute trade: POST /api/trade/commit
  ↓
Confirm transaction on Solana
  ↓
Refresh portfolio data
```

### **4. Cross-Slab Trading Flow**
```
User selects coin (ETH/BTC/SOL)
  ↓
Fetch available slabs: GET /api/router/slabs?coin={coin}
  ↓
User selects slabs and size
  ↓
Execute cross-slab: POST /api/router/execute-cross-slab
  ↓
Router orchestrates multi-slab trade
  ↓
Update UI with result
```

---

## 🎨 **Dashboard Components Breakdown**

### **Chart Component** (`LightweightChart`)
- **Location:** Lines 54-518
- **Features:**
  - Candlestick visualization
  - Multiple timeframes (15m, 1h, 4h, 1D)
  - Real-time WebSocket updates
  - Historical data loading (90 days)
  - Lazy loading on scroll
  - Price change indicators

### **OrderBook Component**
- **Location:** Lines 925-1002
- **Data Sources:**
  - `GET /api/slab-live/orderbook` - Real Slab data
  - `GET /api/slab-live/transactions` - Recent trades
- **Features:**
  - Bid/Ask display
  - Depth visualization
  - Recent trades table
  - Wallet-specific trade filtering

### **Order Form** (`OrderForm`)
- **Location:** Lines 3076-3484
- **Data Sources:**
  - `GET /api/user/{wallet}/portfolio` - Portfolio balance
  - Direct Solana connection - SOL balance
  - `POST /api/trade/reserve` - Reserve collateral
  - `POST /api/trade/commit` - Execute trade
- **Features:**
  - Market/Limit orders
  - Long/Short positions
  - Leverage selection
  - Portfolio margin display
  - Transaction status toasts

### **Cross-Slab Trader**
- **Location:** Lines 2088-3075
- **Data Sources:**
  - `GET /api/router/slabs?coin={coin}` - Available slabs
  - `POST /api/router/execute-cross-slab` - Execute trade
- **Features:**
  - Multi-slab routing
  - VWAP calculations
  - Slab price comparison
  - Atomic execution

---

## 🔍 **Key Data Fetching Patterns**

### **Pattern 1: REST API (Fetch)**
```typescript
// Direct fetch (used in some places)
const response = await fetch(`${API_URL}/api/slab-live/orderbook`)
const data = await response.json()
```

### **Pattern 2: API Client (Recommended)**
```typescript
// Using apiClient singleton
import { apiClient } from '@/lib/api-client'

const markets = await apiClient.getMarkets()
const chartData = await apiClient.getChartData('SOL', '15m', 1000)
const portfolio = await apiClient.getPortfolio(walletAddress)
```

### **Pattern 3: WebSocket Real-Time**
```typescript
// Server WebSocket (recommended)
await apiClient.connectServerWebSocket()
apiClient.subscribeToServerCandle('SOL', '15m')
apiClient.onServerMessage((message) => {
  // Handle candle updates
})

// Standard WebSocket (fallback)
const cleanup = apiClient.connectWebSocket((data) => {
  // Handle market updates
})
```

---

## 🚀 **How to Find Code**

### **Dashboard Entry Point**
📄 `frontend/src/app/dashboard/page.tsx` - **Line 1** (4598 total lines)

### **API Configuration**
📄 `frontend/src/lib/api-client.ts` - **Lines 5-7** (API/WS URLs)

### **Chart Component**
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 54-518**

### **OrderBook Component**
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 925-1002**

### **Trading Form**
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 3076-3484**

### **WebSocket Logic**
📄 `frontend/src/lib/api-client.ts` - **Lines 145-311** (ServerWebSocketClient)
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 236-373** (Chart WebSocket)
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 725-780** (Server WebSocket)

### **Portfolio Data Fetching**
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 3228-3252**

### **Orderbook Data Fetching**
📄 `frontend/src/app/dashboard/page.tsx` - **Lines 934-1002**

---

## 📊 **Tech Stack Summary**

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Framework** | Next.js 15.5.6 | React SSR framework |
| **UI** | Tailwind CSS 4 | Styling |
| **Charts** | Lightweight Charts 5.0.9 | Candlestick visualization |
| **Animations** | Motion/Framer Motion | UI animations |
| **Solana** | @solana/web3.js 1.98.4 | Blockchain interaction |
| **Wallet** | @solana/wallet-adapter | Wallet connections |
| **HTTP Client** | Fetch API / Axios | REST API calls |
| **WebSocket** | Native WebSocket API | Real-time updates |
| **State** | React Hooks (useState, useEffect) | Component state |

---

## 🔗 **Quick Reference: All API Endpoints**

```
Market Data:
  GET  /api/market/list
  GET  /api/market/{symbol}
  GET  /api/market/{symbol}/orderbook
  GET  /api/market/{symbol}/candles

Slab (On-Chain):
  GET  /api/slab-live/orderbook
  GET  /api/slab-live/transactions

Router:
  GET  /api/router/slabs
  POST /api/router/execute-cross-slab

User:
  GET  /api/user/{wallet}/portfolio
  GET  /api/user/{wallet}/positions

Trading:
  POST /api/trade/reserve
  POST /api/trade/commit

Faucet:
  GET  /api/faucet/info
  POST /api/faucet/claim
  POST /api/faucet/airdrop

System:
  GET  /api/health
```

---

## 🎯 **Summary**

**Your dashboard is a sophisticated trading interface that:**
1. ✅ Fetches real-time market data via REST APIs
2. ✅ Connects to WebSocket for live price updates
3. ✅ Displays on-chain Slab orderbook data
4. ✅ Allows portfolio-based trading
5. ✅ Supports cross-slab trading via Router
6. ✅ Shows candlestick charts with historical data
7. ✅ Integrates with Solana wallets

**Main File:** `frontend/src/app/dashboard/page.tsx` (4,598 lines)

**API Client:** `frontend/src/lib/api-client.ts` (504 lines)

**Backend Running On:** `http://localhost:5001`
**Frontend Running On:** `http://localhost:5000`

