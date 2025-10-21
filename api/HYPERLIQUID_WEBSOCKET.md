# 🔌 Hyperliquid WebSocket Integration

A custom WebSocket service that connects to Hyperliquid's real-time market data API and provides configurable candle data subscriptions for any coin and timeframe of your choice.

## 🚀 Features

- **Real-time Candle Data**: Subscribe to live OHLCV data for any supported coin and timeframe
- **Multiple Subscriptions**: Manage multiple coin/timeframe combinations simultaneously
- **Event-driven Architecture**: Listen to real-time updates via callbacks or Server-Sent Events
- **Automatic Reconnection**: Handles connection drops with exponential backoff
- **REST API Integration**: Full REST API for managing subscriptions
- **TypeScript Support**: Complete type definitions for all data structures

## 📊 Supported Data

### Coins
- **Major**: BTC, ETH, SOL, AVAX, MATIC
- **Layer 2**: ARB, OP
- **Newer**: SUI, APT, NEAR, ATOM, DOT, LINK, UNI, AAVE
- **And many more** (see Hyperliquid's full list)

### Timeframes
- **Minutes**: 1m, 3m, 5m, 15m, 30m
- **Hours**: 1h, 2h, 4h, 8h, 12h
- **Days**: 1d, 3d
- **Weeks**: 1w
- **Months**: 1M

## 🛠️ Usage

### 1. Start the API Server

```bash
cd api
npm install
npm run dev
```

### 2. Subscribe to Candle Data

```bash
# Subscribe to BTC 1-minute candles
curl -X POST http://localhost:3000/api/hyperliquid/subscribe \
  -H "Content-Type: application/json" \
  -d '{"coin": "BTC", "interval": "1m"}'

# Subscribe to ETH 5-minute candles
curl -X POST http://localhost:3000/api/hyperliquid/subscribe \
  -H "Content-Type: application/json" \
  -d '{"coin": "ETH", "interval": "5m"}'
```

### 3. Get Latest Data

```bash
# Get latest candle for BTC-1m subscription
curl http://localhost:3000/api/hyperliquid/candles/BTC-1m/latest

# Get historical candles (last 50)
curl http://localhost:3000/api/hyperliquid/candles/BTC-1m?limit=50
```

### 4. Real-time Updates via Server-Sent Events

```javascript
// Connect to real-time stream
const eventSource = new EventSource('/api/hyperliquid/stream/BTC-1m');

eventSource.onmessage = function(event) {
  const data = JSON.parse(event.data);
  if (data.type === 'candle') {
    console.log('New candle:', data.candle);
    // Update your UI with real-time data
  }
};
```

## 📡 API Endpoints

### WebSocket Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/hyperliquid/status` | Get connection status and active subscriptions |
| `POST` | `/api/hyperliquid/subscribe` | Subscribe to candle data |
| `DELETE` | `/api/hyperliquid/unsubscribe/:id` | Unsubscribe from specific feed |
| `GET` | `/api/hyperliquid/unsubscribe-all` | Unsubscribe from all feeds |

### Data Access

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/hyperliquid/candles/:id` | Get historical candle data |
| `GET` | `/api/hyperliquid/candles/:id/latest` | Get latest candle only |
| `GET` | `/api/hyperliquid/stream/:id` | Real-time updates via SSE |

### Utility

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/hyperliquid/coins` | List popular coins |
| `GET` | `/api/hyperliquid/timeframes` | List supported timeframes |
| `POST` | `/api/hyperliquid/subscribe-multiple` | Subscribe to multiple feeds at once |

## 💻 Programmatic Usage

### TypeScript/JavaScript

```typescript
import { hyperliquidWS } from './src/services/hyperliquid-websocket';

// Connect to WebSocket
await hyperliquidWS.connect();

// Subscribe to BTC 1-minute candles
const subscriptionId = hyperliquidWS.subscribeToCandles('BTC', '1m');

// Listen for real-time updates
hyperliquidWS.onCandleUpdate(subscriptionId, (candle) => {
  console.log(`BTC: $${candle.close} (${candle.priceChangePercent.toFixed(2)}%)`);
});

// Get latest data
const latest = hyperliquidWS.getLatestCandle(subscriptionId);
console.log('Latest BTC price:', latest?.close);

// Get historical data
const history = hyperliquidWS.getCandleHistory(subscriptionId, 100);
console.log('Last 100 candles:', history);

// Unsubscribe when done
hyperliquidWS.unsubscribeFromCandles(subscriptionId);
```

### Python

```python
import requests
import json

# Subscribe to multiple feeds
subscriptions = [
    {"coin": "BTC", "interval": "1m"},
    {"coin": "ETH", "interval": "5m"},
    {"coin": "SOL", "interval": "15m"}
]

response = requests.post(
    'http://localhost:3000/api/hyperliquid/subscribe-multiple',
    json={"subscriptions": subscriptions}
)

print("Subscriptions:", response.json())

# Get latest data for each
for sub in subscriptions:
    coin, interval = sub["coin"], sub["interval"]
    subscription_id = f"{coin}-{interval}"
    
    response = requests.get(f'http://localhost:3000/api/hyperliquid/candles/{subscription_id}/latest')
    data = response.json()
    
    if 'candle' in data:
        candle = data['candle']
        print(f"{coin}: ${candle['close']:.2f} ({candle['priceChangePercent']:+.2f}%)")
```

## 🧪 Testing

### 1. Interactive Web Test

Open `test-hyperliquid.html` in your browser:
```bash
# Start the API server
npm run dev

# Open the test page
open api/test-hyperliquid.html
```

### 2. Command Line Test

```bash
# Run the automated test script
node api/test-hyperliquid.js
```

### 3. Manual API Testing

```bash
# Check status
curl http://localhost:3000/api/hyperliquid/status

# Subscribe to BTC 1m
curl -X POST http://localhost:3000/api/hyperliquid/subscribe \
  -H "Content-Type: application/json" \
  -d '{"coin": "BTC", "interval": "1m"}'

# Get latest data
curl http://localhost:3000/api/hyperliquid/candles/BTC-1m/latest

# Unsubscribe
curl -X DELETE http://localhost:3000/api/hyperliquid/unsubscribe/BTC-1m
```

## 📊 Data Format

### Processed Candle Object

```typescript
interface ProcessedCandle {
  symbol: string;           // Coin symbol (e.g., "BTC")
  timeframe: string;        // Interval (e.g., "1m")
  timestamp: number;        // Unix timestamp (milliseconds)
  open: number;            // Opening price
  high: number;            // Highest price
  low: number;             // Lowest price
  close: number;           // Closing price
  volume: number;          // Volume in base units
  trades: number;          // Number of trades
  priceChange: number;     // Absolute price change
  priceChangePercent: number; // Percentage change
}
```

### Example Response

```json
{
  "subscriptionId": "BTC-1m",
  "candle": {
    "symbol": "BTC",
    "timeframe": "1m",
    "timestamp": 1703123456789,
    "open": 65000.00,
    "high": 65100.00,
    "low": 64950.00,
    "close": 65050.00,
    "volume": 125.5,
    "trades": 42,
    "priceChange": 50.00,
    "priceChangePercent": 0.077
  }
}
```

## 🔧 Configuration

### Environment Variables

```env
# Optional: Use testnet instead of mainnet
HYPERLIQUID_TESTNET=false

# Optional: Custom WebSocket URL
HYPERLIQUID_WS_URL=wss://api.hyperliquid.xyz/ws
```

### Service Configuration

```typescript
// Create custom instance
import { HyperliquidWebSocketService } from './src/services/hyperliquid-websocket';

const customWS = new HyperliquidWebSocketService(
  'wss://api.hyperliquid-testnet.xyz/ws', // testnet URL
  true // testnet flag
);

await customWS.connect();
```

## 🚨 Error Handling

The service includes comprehensive error handling:

- **Connection Errors**: Automatic reconnection with exponential backoff
- **Subscription Errors**: Validation of coin symbols and timeframes
- **Data Errors**: Graceful handling of malformed WebSocket messages
- **API Errors**: Detailed error responses with helpful messages

## 🔄 Reconnection Logic

- **Max Attempts**: 5 reconnection attempts
- **Delay**: 5 seconds between attempts
- **Auto-resubscribe**: Automatically resubscribes to all active feeds after reconnection
- **Graceful Degradation**: Continues serving cached data during disconnections

## 📈 Performance

- **Memory Efficient**: Stores only last 1000 candles per subscription
- **Low Latency**: Direct WebSocket connection to Hyperliquid
- **Scalable**: Supports unlimited concurrent subscriptions
- **Cached Data**: Serves historical data even during disconnections

## 🛡️ Security

- **Input Validation**: All coin symbols and timeframes are validated
- **Rate Limiting**: Built-in protection against excessive API calls
- **CORS Support**: Proper CORS headers for web applications
- **Error Sanitization**: Sensitive information is not exposed in error messages

## 🔗 Integration Examples

### React Component

```tsx
import { useEffect, useState } from 'react';

function PriceTicker({ coin, interval }) {
  const [price, setPrice] = useState(null);
  const [change, setChange] = useState(0);

  useEffect(() => {
    const subscriptionId = `${coin}-${interval}`;
    
    // Subscribe to updates
    fetch('/api/hyperliquid/subscribe', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ coin, interval })
    });

    // Listen for real-time updates
    const eventSource = new EventSource(`/api/hyperliquid/stream/${subscriptionId}`);
    eventSource.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === 'candle') {
        setPrice(data.candle.close);
        setChange(data.candle.priceChangePercent);
      }
    };

    return () => {
      eventSource.close();
      fetch(`/api/hyperliquid/unsubscribe/${subscriptionId}`, { method: 'DELETE' });
    };
  }, [coin, interval]);

  return (
    <div>
      <span>${price?.toFixed(2)}</span>
      <span className={change >= 0 ? 'green' : 'red'}>
        {change >= 0 ? '+' : ''}{change?.toFixed(2)}%
      </span>
    </div>
  );
}
```

### Trading Bot Integration

```typescript
class TradingBot {
  private subscriptions = new Map();

  async start() {
    // Subscribe to multiple timeframes for analysis
    const feeds = [
      { coin: 'BTC', interval: '1m' },
      { coin: 'BTC', interval: '5m' },
      { coin: 'ETH', interval: '1m' }
    ];

    for (const feed of feeds) {
      const id = await this.subscribe(feed.coin, feed.interval);
      this.subscriptions.set(id, feed);
    }
  }

  private async subscribe(coin: string, interval: string) {
    const response = await fetch('/api/hyperliquid/subscribe', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ coin, interval })
    });
    
    const result = await response.json();
    return result.subscriptionId;
  }

  async getLatestPrice(coin: string, interval: string) {
    const subscriptionId = `${coin}-${interval}`;
    const response = await fetch(`/api/hyperliquid/candles/${subscriptionId}/latest`);
    const data = await response.json();
    return data.candle?.close;
  }
}
```

## 📚 Additional Resources

- **Hyperliquid API Docs**: [https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket)
- **WebSocket Test Tool**: `api/test-hyperliquid.html`
- **Command Line Test**: `api/test-hyperliquid.js`
- **API Documentation**: `api/README.md`

## 🎯 Use Cases

- **Real-time Price Feeds**: Live price updates for trading interfaces
- **Technical Analysis**: Historical candle data for charting and indicators
- **Trading Bots**: Automated trading based on real-time market data
- **Portfolio Tracking**: Monitor multiple assets simultaneously
- **Market Research**: Analyze price movements across different timeframes
- **Risk Management**: Real-time monitoring for position management

---

**Ready to start?** Run `npm run dev` in the `api` directory and open `test-hyperliquid.html` to see it in action! 🚀
