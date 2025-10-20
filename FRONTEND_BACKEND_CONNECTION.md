# 🔌 Frontend ↔ Backend Connection Guide

## ✅ What's Connected

Your **frontend** and **backend** are now fully integrated!

### Architecture:
```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│   Frontend      │         │   Backend API   │         │   Solana        │
│   (Next.js)     │────────▶│   (Express)     │────────▶│   Blockchain    │
│   Port 3001     │         │   Port 3000     │         │   Devnet        │
└─────────────────┘         └─────────────────┘         └─────────────────┘
       │                            │
       │                            │
       └───── WebSocket ───────────┘
              (Real-time data)
```

---

## 📡 How It Works

### 1. **API Client** (`frontend/src/lib/api-client.ts`)
This is your main connection to the backend. It handles:

- **Market Data**: Prices, orderbooks, charts
- **User Data**: Portfolio, positions, PnL
- **Faucet**: Claim test tokens
- **WebSocket**: Real-time updates

**Example Usage:**
```typescript
import { apiClient } from '@/lib/api-client';

// Get market data
const markets = await apiClient.getMarkets();
const btcData = await apiClient.getMarketData('BTC-PERP');

// Get user portfolio
const portfolio = await apiClient.getPortfolio(walletAddress);

// Subscribe to real-time updates
const cleanup = apiClient.connectWebSocket((message) => {
  console.log('Real-time update:', message);
});
```

### 2. **Data Service** (`frontend/src/lib/data-service.ts`)
Now connects to your **real backend** instead of using mock data!

```typescript
import { CustomDataService } from '@/lib/data-service';

// Fetch chart data from your API
const chartData = await CustomDataService.fetchData('BTC-PERP', '15m');

// Subscribe to real-time candles
const cleanup = CustomDataService.subscribeToRealTimeData('BTC-PERP', (candle) => {
  console.log('New candle:', candle);
});
```

### 3. **Environment Variables** (`.env.local`)
```bash
NEXT_PUBLIC_API_URL=http://localhost:3000
NEXT_PUBLIC_WS_URL=ws://localhost:3000/ws
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_ROUTER_PROGRAM_ID=RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr
NEXT_PUBLIC_SLAB_PROGRAM_ID=SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk
```

---

## 🚀 Running Both Servers

### Terminal 1: Backend API
```bash
cd api
npm run dev
```
**Runs on:** `http://localhost:3000`

### Terminal 2: Frontend
```bash
cd frontend
npm run dev
```
**Runs on:** `http://localhost:3001`

---

## 🎯 Available API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/market/list` | Get all available markets |
| `GET /api/market/:symbol` | Get market data for specific symbol |
| `GET /api/market/:symbol/orderbook` | Get orderbook (bids/asks) |
| `GET /api/market/:symbol/candles` | Get chart/candlestick data |
| `GET /api/user/:wallet/portfolio` | Get user's portfolio |
| `GET /api/user/:wallet/positions` | Get user's open positions |
| `GET /api/faucet/info` | Get faucet information |
| `POST /api/faucet/claim` | Claim test tokens |
| `GET /api/claims/total-claimed` | Get total claimed from faucet |
| `WS ws://localhost:3000/ws` | WebSocket for real-time data |

---

## 📝 Example: Adding Real Data to a Component

### Before (Mock Data):
```tsx
const [data, setData] = useState([]);

useEffect(() => {
  // Using fake data
  const fakeData = generateMockData();
  setData(fakeData);
}, []);
```

### After (Real Backend):
```tsx
import { apiClient } from '@/lib/api-client';

const [data, setData] = useState([]);
const [loading, setLoading] = useState(true);

useEffect(() => {
  const loadData = async () => {
    try {
      // Fetch from your real API!
      const chartData = await apiClient.getChartData('BTC-PERP', '15m');
      setData(chartData);
    } catch (error) {
      console.error('Failed to load data:', error);
    } finally {
      setLoading(false);
    }
  };

  loadData();

  // Real-time updates via WebSocket
  const cleanup = apiClient.connectWebSocket((message) => {
    if (message.type === 'candle') {
      setData(prev => [...prev.slice(-99), message.data]);
    }
  });

  return cleanup; // Cleanup on unmount
}, []);
```

---

## 🔗 Next Steps

### 1. **Add Wallet Connection** (Next task!)
We'll integrate Phantom/Solflare wallets so users can:
- Connect their wallet
- View their real portfolio
- Place trades
- Sign transactions

### 2. **Integrate SDK**
Use the Percolator SDK (`sdk/typescript/`) to interact with Solana programs:
```tsx
import { PercolatorClient } from '@percolator/sdk';

const client = new PercolatorClient(connection, routerProgramId, slabProgramId);
await client.reserve(slab, wallet, ...);
await client.commit(slab, wallet, holdId);
```

### 3. **Build New Pages**
Now that everything is connected, you can build:
- Trading page (place orders)
- Portfolio page (view positions)
- Leaderboard page
- Analytics dashboard

---

## 🐛 Troubleshooting

### "Failed to fetch" errors
- Make sure the backend API is running on port 3000
- Check the .env.local file has correct API URL

### CORS errors
- Backend already has CORS enabled, but verify `api/src/index.ts` has:
  ```typescript
  app.use(cors());
  ```

### WebSocket not connecting
- Ensure both HTTP and WS use same port (3000)
- Check firewall isn't blocking WebSocket connections

---

## 📚 Files Changed

1. ✅ `frontend/package.json` - Updated to run on port 3001
2. ✅ `frontend/.env.local` - Added backend configuration
3. ✅ `frontend/src/lib/api-client.ts` - **NEW** API client
4. ✅ `frontend/src/lib/data-service.ts` - Updated to use real backend

---

**Status:** ✅ Frontend connected to Backend  
**Next:** 🔗 Add Wallet Integration

Let me know which page you want to build next! 🚀

