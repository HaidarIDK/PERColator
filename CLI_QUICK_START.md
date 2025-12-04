# 🚀 Percolator CLI - Quick Start Guide

## What's New! ✨

### 1. **AI Assistant** 🤖
Ask questions in plain English and get the exact CLI commands you need!

Examples:
- "How do I create a slab?" → `matcher create --tick-size 100 --lot-size 1000 ...`
- "How do I initialize?" → `init --name MyExchange`
- "How do I place an order?" → `matcher place-order --side buy ...`

### 2. **Auto-Funding** 💰
The CLI automatically funds your account before running commands! No more "insufficient funds" errors.

### 3. **Web Interface** 🌐
Beautiful terminal interface with:
- Real-time command execution
- Command history (↑/↓ arrows)
- Example commands
- AI assistant suggestions

## 🎯 How to Start

### Step 1: Build the CLI (if not already done)
```bash
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator
cargo build --release --package percolator-cli
```

### Step 2: Start Backend API
```bash
wsl
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator/website/api
npm run dev
```

### Step 3: Start Frontend (in a new terminal)
```bash
wsl
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator/website/frontend
npm run dev
```

### Step 4: Open Your Browser
Go to: **http://localhost:3000/cli**

## 📝 Common Commands

### Initialize Exchange
```bash
-n localnet init --name MyExchange
```

### Create a Market/Slab
```bash
-n localnet matcher create --tick-size 100 --lot-size 1000 <EXCHANGE_ADDRESS> SOL-PERP
```

### List All Markets
```bash
-n localnet matcher list <EXCHANGE_ADDRESS>
```

### Place an Order
```bash
-n localnet matcher place-order --side buy --price 100000 --size 1000 <SLAB_ADDRESS>
```

### View Orderbook
```bash
-n localnet matcher get-orderbook <SLAB_ADDRESS>
```

### Check Status
```bash
-n localnet status <EXCHANGE_ADDRESS>
```

### Run Tests
```bash
-n localnet test --crisis
```

## 💡 Tips

1. **Use the AI Assistant** - Just type your question in the purple box!
2. **Click Example Commands** - Instantly copy them to your terminal
3. **Press ↑/↓** - Navigate through command history
4. **Use `--help`** - Any command with `--help` shows all options

## 🔥 Example Workflow

1. **Ask AI**: "How do I initialize?"
2. **AI suggests**: `init --name MyExchange`
3. **Click "Copy to Terminal"**
4. **Press Enter**
5. **Get your exchange address!**

Then:
1. **Ask AI**: "How do I create a slab?"
2. **AI suggests**: `matcher create --tick-size 100 --lot-size 1000 <EXCHANGE_ADDRESS> SOL-PERP`
3. **Replace** `<EXCHANGE_ADDRESS>` with your exchange address
4. **Press Enter**
5. **Start trading!**

## 🐛 Troubleshooting

### CLI Not Found
```bash
cargo build --release --package percolator-cli
```

### API Not Running
Check if port 5001 is free:
```bash
wsl bash -c "lsof -i :5001"
```

### Frontend Not Running
Check if port 3000 is free:
```bash
wsl bash -c "lsof -i :3000"
```

## 🎮 Advanced Features

### Wallet Management
Use standard Solana CLI:
```bash
solana address          # Show your wallet address
solana balance          # Check your balance
solana airdrop 10       # Get 10 SOL (localnet/devnet)
```

### Liquidity Operations
```bash
-n localnet liquidity --help
```

### AMM Operations
```bash
-n localnet amm --help
```

### Crisis Simulations
```bash
-n localnet crisis --help
```

### Keeper Operations
```bash
-n localnet keeper --help
```

---

## 🌟 Have Fun!

The AI assistant is here to help you learn and execute commands easily. Just ask anything in plain English!

Examples to try:
- "What can I do with the CLI?"
- "How do I add liquidity?"
- "How do I run tests?"
- "How do I check my balance?"





