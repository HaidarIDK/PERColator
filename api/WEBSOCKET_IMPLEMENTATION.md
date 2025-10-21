# WebSocket Server Implementation

This document describes the custom WebSocket server that routes Hyperliquid candle data through your API to your frontend.

## Architecture Overview

```
Frontend (React) 
    ↓ WebSocket Connection
Your API Server (Express + WebSocket)
    ↓ WebSocket Connection  
Hyperliquid WebSocket API
```

## Features

- **Real-time Candle Data**: Routes live candle data from Hyperliquid to your frontend
- **Multi-client Support**: Handles multiple frontend connections simultaneously
- **Subscription Management**: Efficiently manages subscriptions to avoid duplicate Hyperliquid connections
- **Auto-reconnection**: Handles connection drops and reconnects automatically
- **Error Handling**: Comprehensive error handling and logging
- **Health Monitoring**: WebSocket status available via `/api/health` endpoint

## WebSocket Server

### Location
- **File**: `api/src/services/websocket-server.ts`
- **Endpoint**: `ws://localhost:3000/ws`

### Message Format

#### Client → Server (Subscription)
```json
{
  "type": "subscribe",
  "symbol": "SOL",
  "interval": "1m"
}
```

#### Client → Server (Unsubscription)
```json
{
  "type": "unsubscribe", 
  "symbol": "SOL",
  "interval": "1m"
}
```

#### Server → Client (Candle Data)
```json
{
  "type": "candle",
  "subscriptionId": "SOL-1m",
  "data": {
    "symbol": "SOL",
    "timeframe": "1m",
    "timestamp": 1703123456789,
    "open": 100.50,
    "high": 101.20,
    "low": 99.80,
    "close": 100.90,
    "volume": 1500000,
    "trades": 1250,
    "priceChange": 0.40,
    "priceChangePercent": 0.40
  }
}
```

#### Server → Client (Subscription Confirmation)
```json
{
  "type": "subscription",
  "subscriptionId": "SOL-1m",
  "data": {
    "symbol": "SOL",
    "timeframe": "1m",
    "timestamp": 1703123456789,
    "open": 0,
    "high": 0,
    "low": 0,
    "close": 0,
    "volume": 0,
    "trades": 0,
    "priceChange": 0,
    "priceChangePercent": 0
  }
}
```

#### Server → Client (Error)
```json
{
  "type": "error",
  "error": "Invalid message format"
}
```

## Supported Symbols and Intervals

### Symbols
- `SOL` - Solana
- `ETH` - Ethereum  
- `BTC` - Bitcoin
- Any symbol supported by Hyperliquid

### Intervals
- `1m` - 1 minute
- `5m` - 5 minutes
- `15m` - 15 minutes
- `1h` - 1 hour
- `4h` - 4 hours
- `1d` - 1 day

## Integration

### Server Integration
The WebSocket server is automatically started when the Express server starts:

```typescript
// api/src/index.ts
import { initializeWebSocketServer } from './services/websocket-server';

// Start WebSocket server
initializeWebSocketServer(server);
```

### Frontend Integration
Your frontend already has the WebSocket client implemented in `frontend/src/lib/api-client.ts`:

```typescript
// Connect to WebSocket
await apiClient.connectServerWebSocket();

// Subscribe to candle data
apiClient.subscribeToServerCandle('SOL', '1m');

// Listen for messages
const cleanup = apiClient.onServerMessage((data) => {
  console.log('Received candle data:', data);
});
```

## Health Monitoring

Check WebSocket server status via the health endpoint:

```bash
curl http://localhost:3000/api/health
```

Response includes WebSocket statistics:
```json
{
  "status": "healthy",
  "websocket": {
    "server": {
      "connectedClients": 2,
      "activeSubscriptions": 3,
      "hyperliquidSubscriptions": 2
    },
    "hyperliquid": {
      "connected": true,
      "subscriptions": 2
    }
  }
}
```

## Testing

### Manual Testing
Use the provided test script:

```bash
cd api
node test-websocket.js
```

### Frontend Testing
Use the WebSocket test component in your frontend:
- Navigate to `/monitor` page
- Use the WebSocketTest component to test connections

## Error Handling

The WebSocket server handles various error scenarios:

1. **Invalid Message Format**: Returns error message to client
2. **Connection Drops**: Automatically cleans up subscriptions
3. **Hyperliquid Disconnection**: Attempts reconnection with exponential backoff
4. **Client Disconnection**: Removes client from all subscriptions

## Performance Considerations

- **Subscription Deduplication**: Multiple clients subscribing to the same symbol/interval share one Hyperliquid subscription
- **Memory Management**: Keeps only the last 1000 candles per subscription
- **Connection Limits**: No hard limits, but monitors connection count via health endpoint

## Logging

The WebSocket server provides comprehensive logging:

- `🔌` - Connection events
- `📊` - Subscription events  
- `📡` - Data broadcasting
- `❌` - Error events
- `✅` - Success events

## Troubleshooting

### Common Issues

1. **WebSocket Connection Failed**
   - Check if API server is running on port 3000
   - Verify WebSocket endpoint is accessible at `ws://localhost:3000/ws`

2. **No Candle Data Received**
   - Check Hyperliquid WebSocket connection status via `/api/health`
   - Verify symbol and interval are supported
   - Check browser console for error messages

3. **Multiple Subscriptions**
   - Each unique symbol-interval combination creates one Hyperliquid subscription
   - Multiple clients can subscribe to the same data

### Debug Steps

1. Check server logs for WebSocket events
2. Use browser dev tools to inspect WebSocket messages
3. Test with the provided test script
4. Monitor health endpoint for connection status

## Future Enhancements

Potential improvements:
- WebSocket authentication
- Rate limiting per client
- Historical data caching
- Additional data types (trades, orderbook)
- WebSocket compression
- Metrics and monitoring dashboard
