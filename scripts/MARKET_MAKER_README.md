# 🤖 Market Maker Bot

Continuously posts maker orders (bids and asks) to create a live, moving orderbook.

## 🎯 Features

- ✅ Posts **8 orders per side** (16 total orders per cycle)
- ✅ **Refreshes every 30 seconds** with new prices
- ✅ **Dynamic pricing** - Mark price moves ±2% randomly
- ✅ **Realistic spread** - 0.5% between best bid/ask
- ✅ **Depth simulation** - Orders spread from 0.5% to 4% away from mark
- ✅ **Random order sizes** - Between 1-5 SOL per order

## 🚀 Usage

### Start the Bot
```bash
node scripts/market-maker-bot.js
```

### Stop the Bot
Press `Ctrl+C`

**Note:** Orders remain on-chain after stopping. They don't expire automatically.

## 📊 What It Does

```
Every 30 seconds:
  1. Update mark price (±2% random walk)
  2. Generate 8 bid prices below mark
  3. Generate 8 ask prices above mark
  4. Post all 16 orders to the slab
  5. Wait 30 seconds
  6. Repeat
```

## 🎨 Example Output

```
🤖 Market Maker Bot Starting...
   Wallet: dMJN4LV...
   Balance: 9.9908 SOL

🚀 Market Maker Bot Active!
   Slab: 7pyCSG18q...
   Refresh: Every 30s
   Orders per side: 8

📊 Posting Order Cycle #1
   Mark Price: $186.00
   ✅ Posted 16 orders
   📈 8 bids from $185.07 to $178.56
   📉 8 asks from $186.93 to $193.44

📊 Posting Order Cycle #2
   Mark Price: $187.23
   ✅ Posted 14 orders
   ⚠️  2 orders failed (likely duplicates)
   📈 8 bids from $186.29 to $179.74
   📉 8 asks from $188.17 to $194.72
```

## ⚙️ Configuration

Edit `scripts/market-maker-bot.js` to customize:

```javascript
const CONFIG = {
  BASE_PRICE: 186.00,           // Base price for SOL
  PRICE_VOLATILITY: 0.02,       // 2% price movement
  SPREAD_PERCENTAGE: 0.005,     // 0.5% spread
  NUM_ORDERS_PER_SIDE: 8,       // Orders per side
  ORDER_SIZE_MIN: 1.0,          // Min SOL per order
  ORDER_SIZE_MAX: 5.0,          // Max SOL per order
  REFRESH_INTERVAL_MS: 30000,   // 30 seconds
  TICK_SIZE: 1.00,              // $1.00 increments
};
```

## 💡 Tips

1. **Run continuously** - Leave it running to maintain a live orderbook
2. **Monitor balance** - The bot needs SOL to pay transaction fees
3. **Adjust refresh rate** - Faster = more activity, but costs more in fees
4. **Realistic pricing** - Adjust `PRICE_VOLATILITY` for more/less movement

## 🔧 Troubleshooting

**"Orders failed"**: Normal! Means duplicate orders or low balance.

**"Low balance"**: Request devnet SOL from faucet:
```bash
solana airdrop 2 YOUR_ADDRESS --url devnet
```

**Too many orders**: The slab may fill up. Orders will fail gracefully.

## 🎬 Running in Background

### Linux/Mac:
```bash
nohup node scripts/market-maker-bot.js > logs/market-maker.log 2>&1 &
```

### Windows (PowerShell):
```powershell
Start-Process node -ArgumentList "scripts/market-maker-bot.js" -WindowStyle Hidden
```

## 🛑 Stopping Background Process

Find and kill the process:
```bash
# Linux/Mac
ps aux | grep market-maker-bot
kill <PID>

# Windows
tasklist | findstr node
taskkill /PID <PID> /F
```

---

**Now your orderbook will look alive! 📈✨**


