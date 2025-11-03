# PERColator API Endpoints Reference

Complete API reference for frontend integration.

**Base URL:** `http://localhost:3000`

---

## Root

### GET /
API information and available endpoints

**Response:**
```json
{
  "name": "PERColator API",
  "version": "1.0.0",
  "status": "running",
  "endpoints": {
    "health": "/api/health",
    "market": "/api/market/*",
    "trading": "/api/trade/*",
    "user": "/api/user/*",
    "router": "/api/router/*",
    "websocket": "ws://localhost:3000/ws"
  }
}
```

---

## Health

### GET /api/health
System health and Solana connection status

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1734567890000,
  "solana": {
    "network": "devnet",
    "slot": 12345,
    "latency_ms": 45
  }
}
```

---

## Market Data

### GET /api/market/instruments
Get all trading instruments

**Query Parameters:**
- `slab` (required) - Slab account address

**Example:**
```bash
curl "http://localhost:3000/api/market/instruments?slab=Slab1111111111111111111111111111111"
```

**Response:**
```json
[
  {
    "index": 0,
    "symbol": "BTC/USDC",
    "contract_size": 1,
    "tick_size": 0.01,
    "lot_size": 0.001,
    "mark_price": 65000.00,
    "last_price": 65005.50,
    "volume_24h": 1234567,
    "open_interest": 500,
    "funding_rate": 0.0001,
    "next_funding_ms": 1734571490000
  }
]
```

---

### GET /api/market/orderbook
Get orderbook depth

**Query Parameters:**
- `slab` (required) - Slab account address
- `instrument` (required) - Instrument index

**Example:**
```bash
curl "http://localhost:3000/api/market/orderbook?slab=Slab1111111111111111111111111111111&instrument=0"
```

**Response:**
```json
{
  "instrument_idx": 0,
  "timestamp": 1734567890000,
  "bids": [
    { "price": 64998.00, "qty": 0.5, "num_orders": 2 },
    { "price": 64995.00, "qty": 1.2, "num_orders": 3 }
  ],
  "asks": [
    { "price": 65002.00, "qty": 0.8, "num_orders": 1 },
    { "price": 65005.00, "qty": 1.5, "num_orders": 4 }
  ],
  "spread": 4.00,
  "mid": 65000.00
}
```

---

### GET /api/market/trades
Get recent trades

**Query Parameters:**
- `slab` (required) - Slab account address
- `instrument` (required) - Instrument index
- `limit` (optional) - Max trades to return (default: 50, max: 1000)

**Example:**
```bash
curl "http://localhost:3000/api/market/trades?slab=Slab1111111111111111111111111111111&instrument=0&limit=10"
```

**Response:**
```json
[
  {
    "trade_id": 1000,
    "timestamp": 1734567890000,
    "price": 65000.50,
    "qty": 0.5,
    "side": "buy"
  }
]
```

---

### GET /api/market/stats
Get 24h market statistics

**Query Parameters:**
- `slab` (required) - Slab account address

**Response:**
```json
{
  "volume_24h": 10234567,
  "trades_24h": 5432,
  "high_24h": 65500,
  "low_24h": 64200,
  "open_24h": 64800,
  "last_price": 65005,
  "price_change_24h": 205,
  "price_change_pct_24h": 0.32
}
```

---

## Trading Operations

### POST /api/trade/order
Place a new order

**Request Body:**
```json
{
  "slab": "Slab1111111111111111111111111111111",
  "user": "UserWalletAddress...",
  "instrument": 0,
  "side": "buy",
  "price": 65000,
  "qty": 1.0,
  "order_type": "limit"
}
```

**Response:**
```json
{
  "success": true,
  "order_id": 123456,
  "status": "pending",
  "timestamp": 1734567890000,
  "signature": "TxSignature..."
}
```

---

### POST /api/trade/cancel
Cancel an order

**Request Body:**
```json
{
  "slab": "Slab1111111111111111111111111111111",
  "user": "UserWalletAddress...",
  "order_id": 123456
}
```

**Response:**
```json
{
  "success": true,
  "order_id": 123456,
  "status": "cancelled",
  "timestamp": 1734567890000
}
```

---

### POST /api/trade/reserve
Reserve liquidity (two-phase execution, step 1)

**Request Body:**
```json
{
  "slab": "Slab1111111111111111111111111111111",
  "user": "UserWalletAddress...",
  "instrument": 0,
  "side": "buy",
  "qty": 1.0,
  "limit_px": 65000,
  "ttl_ms": 60000
}
```

**Response:**
```json
{
  "success": true,
  "hold_id": 789012,
  "vwap_px": 65000,
  "worst_px": 65005,
  "max_charge": 65000,
  "expiry_ms": 1734627890000,
  "reserved_qty": 1.0
}
```

---

### POST /api/trade/commit
Commit a reservation (two-phase execution, step 2)

**Request Body:**
```json
{
  "slab": "Slab1111111111111111111111111111111",
  "hold_id": 789012
}
```

**Response:**
```json
{
  "success": true,
  "fills": [
    { "price": 65000, "qty": 0.5, "fee": 3.25 },
    { "price": 65002, "qty": 0.5, "fee": 3.25 }
  ],
  "total_qty": 1.0,
  "vwap": 65001.00,
  "total_fee": 6.50,
  "signature": "TxSignature..."
}
```

---

## User Data

### GET /api/user/balance
Get user balance in slab

**Query Parameters:**
- `slab` (required) - Slab account address
- `user` (required) - User wallet address

**Example:**
```bash
curl "http://localhost:3000/api/user/balance?slab=Slab1111111111111111111111111111111&user=UserWallet..."
```

**Response:**
```json
{
  "user": "UserWallet...",
  "slab": "Slab1111111111111111111111111111111",
  "balance": 10000.00,
  "available": 8500.00,
  "reserved": 1500.00,
  "pnl_unrealized": 250.50,
  "pnl_realized": 1234.00
}
```

---

### GET /api/user/positions
Get user's open positions

**Query Parameters:**
- `slab` (required) - Slab account address
- `user` (required) - User wallet address

**Response:**
```json
[
  {
    "instrument": 0,
    "symbol": "BTC/USDC",
    "qty": 0.5,
    "side": "long",
    "entry_price": 64500,
    "mark_price": 65000,
    "pnl": 250.00,
    "pnl_pct": 0.78
  }
]
```

---

### GET /api/user/orders
Get user's orders

**Query Parameters:**
- `slab` (required) - Slab account address
- `user` (required) - User wallet address
- `status` (optional) - Filter by status: "open" or "all" (default: "open")

**Response:**
```json
[
  {
    "order_id": 123456,
    "instrument": 0,
    "symbol": "BTC/USDC",
    "side": "buy",
    "price": 64900,
    "qty": 1.0,
    "filled_qty": 0,
    "status": "live",
    "created_at": 1734567590000
  }
]
```

---

### GET /api/user/trades
Get user's trade history

**Query Parameters:**
- `slab` (required) - Slab account address
- `user` (required) - User wallet address
- `limit` (optional) - Max trades (default: 50, max: 1000)

**Response:**
```json
[
  {
    "trade_id": 789012,
    "order_id": 123455,
    "instrument": 0,
    "symbol": "BTC/USDC",
    "side": "buy",
    "price": 64800,
    "qty": 0.5,
    "fee": 16.20,
    "role": "taker",
    "timestamp": 1734564290000
  }
]
```

---

### GET /api/user/portfolio
Get portfolio summary

**Query Parameters:**
- `slab` (required) - Slab account address
- `user` (required) - User wallet address

**Response:**
```json
{
  "user": "UserWallet...",
  "total_value": 11484.50,
  "cash": 10000.00,
  "positions_value": 1484.50,
  "pnl_unrealized": 250.50,
  "pnl_realized_24h": 34.00,
  "pnl_realized_total": 1234.00,
  "num_positions": 1,
  "num_open_orders": 1
}
```

---

## Router Operations

### POST /api/router/deposit
Deposit collateral to router vault

**Request Body:**
```json
{
  "user": "UserWalletAddress...",
  "mint": "USDC_MINT_ADDRESS",
  "amount": 1000.00
}
```

**Response:**
```json
{
  "success": true,
  "user": "UserWallet...",
  "mint": "USDC_MINT...",
  "amount": 1000.00,
  "vault_balance": 11000.00,
  "signature": "TxSignature..."
}
```

---

### POST /api/router/withdraw
Withdraw collateral from router vault

**Request Body:**
```json
{
  "user": "UserWalletAddress...",
  "mint": "USDC_MINT_ADDRESS",
  "amount": 500.00
}
```

**Response:**
```json
{
  "success": true,
  "user": "UserWallet...",
  "mint": "USDC_MINT...",
  "amount": 500.00,
  "vault_balance": 9500.00,
  "signature": "TxSignature..."
}
```

---

### GET /api/router/portfolio/:user
Get cross-slab portfolio (aggregated across all slabs)

**URL Parameters:**
- `user` - User wallet address

**Example:**
```bash
curl "http://localhost:3000/api/router/portfolio/UserWallet..."
```

**Response:**
```json
{
  "user": "UserWallet...",
  "equity": 11500.00,
  "im": 2300.00,
  "mm": 1150.00,
  "free_collateral": 9200.00,
  "leverage": 2.5,
  "positions": [
    {
      "slab_id": "Slab1...",
      "instrument": 0,
      "symbol": "BTC/USDC",
      "qty": 0.5,
      "entry_price": 64500,
      "mark_price": 65000,
      "pnl": 250.00
    }
  ],
  "total_collateral": 10000.00,
  "pnl_unrealized": 250.00,
  "pnl_realized": 1250.00
}
```

---

### GET /api/router/slabs
Get all registered slabs

**Response:**
```json
[
  {
    "slab_id": "Slab1111111111111111111111111111111",
    "name": "Alpha Slab",
    "instruments": ["BTC/USDC", "ETH/USDC", "SOL/USDC"],
    "imr": 500,
    "mmr": 250,
    "maker_fee": 10,
    "taker_fee": 20,
    "active": true,
    "volume_24h": 1234567
  }
]
```

---

### POST /api/router/reserve-multi
Reserve across multiple slabs (smart routing)

**Request Body:**
```json
{
  "user": "UserWallet...",
  "instrument": 0,
  "side": "buy",
  "qty": 1.0,
  "limit_px": 65010,
  "slabs": ["Slab1...", "Slab2..."]
}
```

**Response:**
```json
{
  "success": true,
  "route_id": 456789,
  "reservations": [
    {
      "slab_id": "Slab1...",
      "hold_id": 111,
      "qty": 0.7,
      "vwap": 65000,
      "max_charge": 45500
    },
    {
      "slab_id": "Slab2...",
      "hold_id": 222,
      "qty": 0.3,
      "vwap": 65005,
      "max_charge": 19501.5
    }
  ],
  "total_qty": 1.0,
  "blended_vwap": 65002,
  "total_cost": 65002
}
```

---

### POST /api/router/commit-multi
Commit multi-slab reservation atomically

**Request Body:**
```json
{
  "route_id": 456789
}
```

**Response:**
```json
{
  "success": true,
  "route_id": 456789,
  "fills": [
    { "slab": "Slab1...", "qty": 0.7, "price": 65000, "fee": 22.75 },
    { "slab": "Slab2...", "qty": 0.3, "price": 65005, "fee": 9.75 }
  ],
  "total_qty": 1.0,
  "blended_vwap": 65002,
  "total_fee": 32.50,
  "signatures": ["TxSig1...", "TxSig2..."]
}
```

---

### GET /api/router/vault/:mint
Get vault balance for a mint

**URL Parameters:**
- `mint` - Token mint address (e.g., USDC)

**Example:**
```bash
curl "http://localhost:3000/api/router/vault/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
```

**Response:**
```json
{
  "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "balance": 1000000.00,
  "pledged": 250000.00,
  "available": 750000.00,
  "num_users": 42
}
```

---

## WebSocket API

### Connection
```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
  console.log('Connected to PERColator WebSocket');
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

### Subscribe to Channel
```javascript
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'orderbook:BTC/USDC'
}));
```

### Available Channels
- `orderbook:{symbol}` - Real-time orderbook updates
- `trades:{symbol}` - Live trade feed
- `user:{address}` - User-specific updates (fills, orders, positions)

### Orderbook Update
```json
{
  "type": "update",
  "channel": "orderbook:BTC/USDC",
  "data": {
    "bids": [
      { "price": 65000, "qty": 0.5 }
    ],
    "asks": [
      { "price": 65005, "qty": 0.8 }
    ]
  },
  "timestamp": 1734567890000
}
```

### Trade Update
```json
{
  "type": "update",
  "channel": "trades:BTC/USDC",
  "data": {
    "trade_id": 12345,
    "price": 65000,
    "qty": 0.5,
    "side": "buy"
  },
  "timestamp": 1734567890000
}
```

### Unsubscribe
```javascript
ws.send(JSON.stringify({
  type: 'unsubscribe',
  channel: 'orderbook:BTC/USDC'
}));
```

### Ping/Pong
```javascript
ws.send(JSON.stringify({ type: 'ping' }));
// Response: { type: 'pong', timestamp: ... }
```

---

## Error Responses

All errors follow this format:
```json
{
  "error": "Error message describing what went wrong"
}
```

**Common HTTP Status Codes:**
- `200` - Success
- `400` - Bad request (missing/invalid parameters)
- `404` - Route or resource not found
- `500` - Internal server error
- `503` - Service unavailable

---

## Testing

### cURL Examples

```bash
# Health check
curl http://localhost:3000/api/health

# Get orderbook
curl "http://localhost:3000/api/market/orderbook?slab=Slab1111111111111111111111111111111&instrument=0"

# Place order
curl -X POST http://localhost:3000/api/trade/order \
  -H "Content-Type: application/json" \
  -d '{
    "slab": "Slab1111111111111111111111111111111",
    "user": "UserWallet...",
    "instrument": 0,
    "side": "buy",
    "price": 65000,
    "qty": 1.0
  }'

# Get user portfolio
curl "http://localhost:3000/api/user/portfolio?slab=Slab1111111111111111111111111111111&user=UserWallet..."

# Get router portfolio (cross-slab)
curl "http://localhost:3000/api/router/portfolio/UserWallet..."
```

### JavaScript/TypeScript Example

```typescript
// Fetch orderbook
const response = await fetch(
  'http://localhost:3000/api/market/orderbook?slab=Slab1...&instrument=0'
);
const orderbook = await response.json();

// Place order
const orderResponse = await fetch('http://localhost:3000/api/trade/order', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    slab: 'Slab1...',
    user: walletAddress,
    instrument: 0,
    side: 'buy',
    price: 65000,
    qty: 1.0
  })
});
const order = await orderResponse.json();

// WebSocket
const ws = new WebSocket('ws://localhost:3000/ws');
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'orderbook:BTC/USDC'
  }));
};
```

---

## Current Status

**Implementation:** All endpoints working with mock data  
**Next Step:** Connect to deployed Solana programs on devnet  
**Testing:** Frontend can build complete UI now using mock data

---

## Notes for Frontend Developer

1. All endpoints return realistic mock data for UI development
2. WebSocket streams update every 2-3 seconds for testing
3. No wallet signing required yet (transactions return mock signatures)
4. Once programs deployed to devnet, only backend code changes - frontend stays same
5. Error handling is consistent across all endpoints
6. All amounts in decimal format (not lamports/smallest units)

