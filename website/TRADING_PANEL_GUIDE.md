# Trading Panel Integration Guide

## What's Been Built

The trading panel is now fully integrated with your Solana programs on devnet! Users can:

### ✅ Portfolio Management
- **Create Portfolio**: Initialize a portfolio account (PDA) for trading
- **View Balances**: See both wallet balance and portfolio balance in real-time
- **Deposit Funds**: Transfer SOL from wallet to portfolio for trading
- **Auto-refresh**: Balances update every 5 seconds

### ✅ Order Form
- **Buy/Sell Orders**: Toggle between buy and sell
- **Limit/Market Orders**: Support for both order types
- **Price Input**: Set your limit price (for limit orders)
- **Size Input**: Specify order size
- **Total Calculation**: Automatically calculates order total

## Program IDs (Devnet)

From `keypairs/config.json`:
- **Router**: `rPB77V5pFZ3dzwgmUH5jvXDwzK7FTqs92nyFgBecxMh`
- **Slab**: `sDoE9Fb3H516S5ZuxBD1SUFgP1Z7ddXBpi9fweZb6Mk`
- **AMM**: `aMepWm9uGGtMGpufmUs8xKAVV8ES8QLPHXg1612RCmz`

## How to Use

### 1. Connect Wallet
Click "Select Wallet" in the top right and choose your Solana wallet (Phantom, Solflare, etc.)

### 2. Create Portfolio
If you don't have a portfolio yet:
- The panel will show "No portfolio account found"
- Click "Create Portfolio" button
- Approve the transaction in your wallet
- Wait ~2 seconds for confirmation

### 3. Deposit Funds
Once portfolio is created:
- Enter amount in SOL (e.g., 0.1)
- Click "Deposit" button
- Approve the transaction
- Funds move from wallet to portfolio

### 4. Place Orders (Coming Soon)
- Select Buy or Sell
- Choose Limit or Market order type
- Enter price (for limit orders) and size
- Click Buy/Sell button
- Order will be sent to the slab program

## File Structure

```
website/frontend/src/
├── lib/
│   ├── solana-config.ts      # Program IDs and constants
│   └── solana-client.ts      # Solana transaction builder
├── components/
│   ├── PortfolioPanel.tsx    # Portfolio management UI
│   ├── OrderForm.tsx          # Order entry form
│   └── TradingChart.tsx       # Price chart
└── app/
    └── dashboard/
        └── page.tsx           # Main dashboard page
```

## Next Steps to Complete

### 1. Order Execution
Need to implement the actual order submission to slab program:
- Create `ExecuteCrossSlab` instruction
- Handle order matching
- Show order confirmation

### 2. Order Book Display
Fetch and display live order book from slab:
- Bid levels (green)
- Ask levels (red)
- Depth visualization

### 3. Recent Trades
Show recent trade history:
- Price
- Size
- Time
- Side (buy/sell)

### 4. Open Orders
Display user's active orders:
- Order ID
- Side
- Price
- Size
- Status
- Cancel button

### 5. Position Management
Show user's open positions:
- Size
- Entry price
- Current PnL
- Unrealized PnL

## CLI Commands Reference

The frontend replicates these CLI commands:

```bash
# Initialize portfolio
percolator margin init

# Deposit funds
percolator margin deposit --amount 100000000  # lamports

# Place limit order
percolator trade limit <matcher> buy --price 186.50 --size 1000000

# Place market order
percolator trade market <matcher> buy --size 1000000

# Show order book
percolator trade book <matcher>
```

## Testing on Devnet

1. **Get Devnet SOL**:
   ```bash
   solana airdrop 2 --url devnet
   ```

2. **Check your balance**:
   ```bash
   solana balance --url devnet
   ```

3. **View portfolio on explorer**:
   - Go to https://explorer.solana.com/?cluster=devnet
   - Search for your wallet address
   - Look for portfolio PDA account

## Troubleshooting

### "Transaction failed"
- Make sure you have enough SOL for transaction fees
- Check that programs are deployed on devnet
- Verify wallet is connected to devnet

### "Portfolio not found"
- Click "Create Portfolio" button first
- Wait for transaction confirmation
- Refresh the page

### "Insufficient balance"
- Deposit more funds to your portfolio
- Make sure you're depositing to portfolio, not just wallet

## Technical Notes

- All transactions use PDAs (Program Derived Addresses)
- Portfolio uses seed: `["portfolio", user_pubkey]`
- Minimum SOL needed: ~0.02 SOL (for rent + fees)
- Network: Devnet
- RPC: https://api.devnet.solana.com

