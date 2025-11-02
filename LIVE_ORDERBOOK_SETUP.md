# 📊 Live OrderBook Setup

Your orderbook now looks **alive** with continuously changing orders!

## 🎯 What We Built

### **1. Dynamic OrderBook Simulator**
- ✅ **Generates new orders** on every API request
- ✅ **Random price volatility** (±2% movement)
- ✅ **Realistic spread** (0.5% between best bid/ask)
- ✅ **Variable depth** (8-10 orders per side)
- ✅ **Random order sizes** (1-5 SOL each)
- ✅ **Multi-coin support** (SOL @ $186, ETH @ $2500, BTC @ $45000)

### **2. Market Maker Bot** (Optional)
- 🤖 Posts **real on-chain orders** to the slab
- 🔄 Refreshes every 30 seconds
- 📈 Creates actual blockchain activity
- 📊 Can be used when slab parser is ready

## 🚀 How It Works

```
User loads OrderBook
       ↓
API generates dynamic data
       ↓
Random prices/quantities
       ↓
OrderBook updates every 5s
       ↓
Looks like live trading!
```

## 🎨 Visual Features

- **Trading Pair Badge**: Shows "SOL-USD", "ETH-USD", "BTC-USD"
- **Slab Address**: Displays truncated slab account
- **Status Indicator**: 
  - 🔵 "Live Simulated" - Dynamic generated data
  - 🟢 "On-Chain" - Real slab orders (when parser works)

## 📱 What You See

Every time the orderbook refreshes (5 seconds):
- ✅ **Prices change** slightly (±2%)
- ✅ **Order count varies** (8-10 per side)
- ✅ **Sizes are random** (1-5 SOL)
- ✅ **Spread updates** dynamically

## 🔥 Switch Between Coins

**SOL**: ~$186
```
Bids: $185.07, $184.23, $183.44, ...
Asks: $186.93, $187.79, $188.56, ...
```

**ETH**: ~$2500
```
Bids: $2487, $2473, $2461, ...
Asks: $2513, $2526, $2539, ...
```

**BTC**: ~$45000
```
Bids: $44,775, $44,595, $44,370, ...
Asks: $45,225, $45,405, $45,630, ...
```

## 🤖 Using the Market Maker Bot

If you want **real on-chain orders** instead of simulated:

### Start the Bot:
```bash
node scripts/market-maker-bot.js
```

The bot will:
- Post 16 orders (8 bids + 8 asks)
- Refresh every 30 seconds
- Create real blockchain transactions

### Stop the Bot:
Press `Ctrl+C`

**Note:** Once the slab parser is working, the orderbook will automatically switch to showing these real orders!

## ⚙️ Configuration

### Change Base Prices
Edit `website/api/src/routes/slab.ts`:

```typescript
generateDynamicOrderbook(
  coin === 'BTC' ? 45000 :   // BTC price
  coin === 'ETH' ? 2500 :    // ETH price
  186                        // SOL price
);
```

### Adjust Volatility
In `generateDynamicOrderbook()`:

```typescript
const priceVolatility = (Math.random() - 0.5) * 0.04;  // ±2%
// Change to 0.08 for ±4%, or 0.02 for ±1%
```

### Change Refresh Rate
Edit `website/frontend/src/components/OrderBook.tsx`:

```typescript
const interval = setInterval(fetchOrderbook, 5000)  // 5 seconds
// Change to 3000 for 3s, 10000 for 10s, etc.
```

## 📊 Current Status

- ✅ **Dynamic orderbook** - Working!
- ✅ **Multi-coin support** - Working!
- ✅ **Live updates** - Working!
- ✅ **Market maker bot** - Ready to use!
- 🔄 **Real slab parsing** - TODO (complex binary structure)

## 🎉 Result

Your orderbook now **looks professional** with:
- 📈 Moving prices
- 🔄 Changing quantities
- 💹 Dynamic depth
- ⚡ Real-time updates

**Refresh your browser and watch the magic! ✨**

---

## 📝 Notes

The simulated data is:
- ✅ **Realistic** - Mimics real trading activity
- ✅ **Dynamic** - Changes on every request
- ✅ **Multi-coin** - Different prices for SOL/ETH/BTC
- ✅ **Client-friendly** - No blockchain parsing needed

When the slab parser is implemented, the orderbook will automatically switch from "Live Simulated" to "On-Chain" and show real orders!


