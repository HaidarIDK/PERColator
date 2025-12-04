# How to Run the Interactive CLI

## Step-by-Step Instructions

### Step 1: Fix OpenSSL (One-time Setup)

The CLI needs OpenSSL to build. Choose the easiest option for you:

#### Option A: Use WSL (Recommended - No OpenSSL setup needed)

```powershell
# Open WSL terminal (if installed)
wsl

# Navigate to project
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator

# Build (OpenSSL works automatically in WSL)
cargo build --release -p percolator-cli

# Run
./target/release/percolator -n devnet interactive
```

#### Option B: Install OpenSSL on Windows

```powershell
# If you have Chocolatey:
choco install openssl

# OR download and install from:
# https://slproweb.com/products/Win32OpenSSL.html
# Install to: C:\Program Files\OpenSSL-Win64
```

### Step 2: Build the CLI

```powershell
# Build in release mode (optimized)
cargo build --release -p percolator-cli

# This creates: target/release/percolator.exe
```

### Step 3: Get Devnet SOL (for testing)

```powershell
# Make sure you're on devnet
solana config set --url devnet

# Get free test SOL
solana airdrop 2

# Check your balance
solana balance
```

### Step 4: Run the Interactive CLI

```powershell
# Method 1: Using cargo run (easiest)
cargo run --release -p percolator-cli -- -n devnet interactive

# Method 2: Run the executable directly
.\target\release\percolator.exe -n devnet interactive

# Method 3: On localnet (if you have local validator running)
solana-test-validator &
cargo run --release -p percolator-cli -- -n localnet interactive
```

## What You'll See

```
=== Balance Check ===
Address: [your address]
Balance: X.XXXX SOL

╔═══════════════════════════════════════════════════════╗
║       Percolator Protocol - Interactive CLI          ║
╚═══════════════════════════════════════════════════════╝

Network: devnet
Address: [your address]

Main Menu:

  1.  Setup & Deployment
  2.  Slab Operations (Create, Manage)
  3.  Trading (Place Orders, View Orderbook)
  4.  Margin & Portfolio
  5.  AMM Operations
  6.  Liquidity Operations
  7.  Status & Info
  8.  Run Tests

  0.  Quit

Enter your choice:
```

## Quick Test Workflow

Once the CLI is running:

1. **Setup** (Menu 1):
   - Option 1: Initialize Exchange
   - Option 2: Initialize Portfolio

2. **Create Slab** (Menu 2):
   - Option 1: Create New Slab
   - Option 2: Register Slab

3. **Deposit** (Menu 4):
   - Option 2: Deposit Collateral (e.g., 1 SOL)

4. **Trade** (Menu 3):
   - Option 1: Place Limit Order
   - Option 6: View Orderbook

5. **Check Status** (Menu 7):
   - Option 2: View Portfolio

## Troubleshooting

### "OpenSSL not found" Error

**Solution:** Install OpenSSL or use WSL

```powershell
# Check if OpenSSL is installed
Test-Path "C:\Program Files\OpenSSL-Win64\lib"

# If false, install it (see Step 1)
```

### "Insufficient balance" Warning

**Solution:** Get devnet SOL

```powershell
solana airdrop 2
```

### "Portfolio not initialized" Error

**Solution:** Initialize portfolio first

```
1. Go to Menu 1 (Setup & Deployment)
2. Choose Option 2 (Initialize Portfolio)
```

### "Network connection failed"

**Solution:** Check your network and RPC URL

```powershell
# Check Solana config
solana config get

# Should show devnet URL
# RPC URL: https://api.devnet.solana.com
```

## Alternative: Non-Interactive Mode

If you prefer command-line arguments instead of menus:

```powershell
# Initialize exchange
cargo run --release -p percolator-cli -- -n devnet init --name test

# Initialize portfolio
cargo run --release -p percolator-cli -- -n devnet margin init

# Deposit collateral
cargo run --release -p percolator-cli -- -n devnet margin deposit 1000000000

# View portfolio
cargo run --release -p percolator-cli -- -n devnet margin show
```

## Need Help?

- See `QUICK_START.md` for quick setup
- See `BUILD_INSTRUCTIONS.md` for detailed build help
- See `INTERACTIVE_CLI.md` for full documentation

## Example Session

```powershell
# 1. Build (one time, or when code changes)
cargo build --release -p percolator-cli

# 2. Get devnet SOL
solana airdrop 2

# 3. Run interactive CLI
cargo run --release -p percolator-cli -- -n devnet interactive

# 4. In the menu:
#    - Choose 1 (Setup)
#    - Choose 2 (Initialize Portfolio)
#    - Choose 4 (Margin)
#    - Choose 2 (Deposit)
#    - Enter amount: 1.0
#    - Enjoy testing! 🎉
```










