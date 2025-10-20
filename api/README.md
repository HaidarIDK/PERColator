# PERColator API Server

REST API server for PERColator perpetual exchange protocol on PERCS.

## 🚀 Quick Start

```bash
cd api
npm install
cp .env.example .env
# Edit .env with your configuration
npm run dev
```

The server will start on `http://localhost:3000`

## 📡 API Endpoints

### Health & Status

#### `GET /api/health`
Check API server and Solana network health.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1234567890,
  "solana": {
    "network": "localnet",
    "slot": 12345,
    "latency_ms": 45
  }
}
```

---

### Market Data

#### `GET /api/market/instruments?slab={address}`
Get all trading instruments.

**Query Parameters:**
- `slab` (required): Slab program account address

**Response:**
```json
[
  {
    "index": 0,
    "symbol": "BTC/USDC",
    "mark_price": 65000.00,
    "volume_24h": 1234567,
    "funding_rate": 0.0001
  }
]
```

#### `GET /api/market/orderbook?slab={address}&instrument={idx}`
Get orderbook for an instrument.

**Query Parameters:**
- `slab` (required): Slab account address
- `instrument` (required): Instrument index

**Response:**
```json
{
  "instrument_idx": 0,
  "timestamp": 1234567890,
  "bids": [
    { "price": 64998.00, "qty": 0.5, "num_orders": 2 }
  ],
  "asks": [
    { "price": 65002.00, "qty": 0.8, "num_orders": 1 }
  ],
  "spread": 4.00,
  "mid": 65000.00
}
```

#### `GET /api/market/trades?slab={address}&instrument={idx}&limit=50`
Get recent trades.

**Response:**
```json
[
  {
    "trade_id": 1000,
    "timestamp": 1234567890,
    "price": 65000,
    "qty": 0.5,
    "side": "buy"
  }
]
```

---

### Trading

#### `POST /api/trade/order`
Place a new order (simplified for MVP).

**Request Body:**
```json
{
  "slab": "SlabAddress...",
  "user": "UserAddress...",
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
  "timestamp": 1234567890,
  "signature": "TxSignature..."
}
```

#### `POST /api/trade/cancel`
Cancel an order.

**Request Body:**
```json
{
  "slab": "SlabAddress...",
  "user": "UserAddress...",
  "order_id": 123456
}
```

#### `POST /api/trade/reserve`
Reserve liquidity (two-phase, step 1).

**Request Body:**
```json
{
  "slab": "SlabAddress...",
  "user": "UserAddress...",
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
  "expiry_ms": 1234627890,
  "reserved_qty": 1.0
}
```

#### `POST /api/trade/commit`
Commit a reservation (two-phase, step 2).

**Request Body:**
```json
{
  "slab": "SlabAddress...",
  "hold_id": 789012
}
```

**Response:**
```json
{
  "success": true,
  "fills": [
    { "price": 65000, "qty": 0.5, "fee": 3.25 }
  ],
  "total_qty": 0.5,
  "vwap": 65000,
  "signature": "TxSignature..."
}
```

---

### User Data

#### `GET /api/user/balance?slab={address}&user={address}`
Get user's balance.

**Response:**
```json
{
  "user": "UserAddress...",
  "balance": 10000.00,
  "available": 8500.00,
  "reserved": 1500.00,
  "pnl_unrealized": 250.50
}
```

#### `GET /api/user/positions?slab={address}&user={address}`
Get user's positions.

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
    "pnl": 250.00
  }
]
```

#### `GET /api/user/orders?slab={address}&user={address}&status=open`
Get user's orders.

**Response:**
```json
[
  {
    "order_id": 123456,
    "instrument": 0,
    "side": "buy",
    "price": 64900,
    "qty": 1.0,
    "filled_qty": 0,
    "status": "live"
  }
]
```

#### `GET /api/user/trades?slab={address}&user={address}&limit=50`
Get user's trade history.

#### `GET /api/user/portfolio?slab={address}&user={address}`
Get portfolio summary.

**Response:**
```json
{
  "total_value": 11484.50,
  "cash": 10000.00,
  "positions_value": 1484.50,
  "pnl_unrealized": 250.50,
  "num_positions": 1,
  "num_open_orders": 1
}
```

---

### WebSocket API

Connect to `ws://localhost:3000/ws`

**Subscribe to channels:**
```json
{
  "type": "subscribe",
  "channel": "orderbook:BTC/USDC"
}
```

**Channels:**
- `orderbook:{symbol}` - Orderbook updates
- `trades:{symbol}` - Trade feed
- `user:{address}` - User updates (orders, fills, positions)

**Unsubscribe:**
```json
{
  "type": "unsubscribe",
  "channel": "orderbook:BTC/USDC"
}
```

---

## 🔧 Configuration

Edit `.env`:

```env
SOLANA_RPC_URL=http://localhost:8899
SOLANA_NETWORK=localnet
PORT=3000
SLAB_PROGRAM_ID=YourProgramId
```

## 📝 Notes

- Currently returns **mock data** for testing frontend integration
- Real slab parsing coming next (need to parse 10MB SlabState)
- Transaction signing requires wallet keypair
- WebSocket broadcasts mock updates every 2-3 seconds

## 🧪 Testing

```bash
# Start dev server
npm run dev

# Test health endpoint
curl http://localhost:3000/api/health

# Test orderbook
curl "http://localhost:3000/api/market/orderbook?slab=11111111111111111111111111111111&instrument=0"
```

