# PERColator CLI

**✨ NOW IN RUST!** Production-grade command-line interface for PERColator DEX.

## 🚀 **What Changed**

**Before:** TypeScript stub with 410 lines of `// TODO: Implement`  
**Now:** Complete Rust implementation with all features working!

---

## 📦 **Installation**

### **Build from Source:**

```bash
# From workspace root
cargo build --release -p percolator-cli

# Binary location: target/release/perc (or perc.exe on Windows)
```

### **Install Globally:**

```bash
cd cli
cargo install --path .

# Now available as: perc --help
```

### **⚠️ Windows Note:**

If OpenSSL build fails, install OpenSSL:
- Download: https://slproweb.com/products/Win32OpenSSL.html  
- Install to: `C:\Program Files\OpenSSL-Win64`

---

## 📖 **Usage**

### LP Operations

```bash
# Create new slab
perc lp create-slab --market BTC-PERP --imr 500 --mmr 250

# Add instrument to slab
perc lp add-instrument -s <ADDRESS> --symbol BTC/USDC --price 65000

# Update slab parameters
perc lp set-params -s <ADDRESS> --imr 600 --mmr 300
```

### Trading

```bash
# Reserve liquidity
perc trade reserve -s <ADDRESS> --side buy --qty 1 --price 65000

# Commit reservation
perc trade commit -s <ADDRESS> --hold-id 12345

# Cancel reservation
perc trade cancel -s <ADDRESS> --hold-id 12345
```

### Portfolio

```bash
# Show portfolio summary
perc portfolio show

# List all positions
perc portfolio positions
```

### Market Making

```bash
# Post two-sided quote
perc mm quote -s <ADDRESS> --mid 65000 --spread 10 --size 1

# Run market making bot
perc mm watch -s <ADDRESS> --spread 10 --size 1
```

### Monitoring

```bash
# Monitor equity
perc monitor equity --interval 5

# Monitor liquidation opportunities
perc monitor liquidations --min-profit 100
```

### Admin

```bash
# Initialize router
perc admin initialize-router

# Register slab
perc admin register-slab -s <ADDRESS>
```

### Utilities

```bash
# Show configuration
perc config

# Check SOL balance
perc balance

# Request airdrop (devnet)
perc airdrop --amount 2
```

---

## ⚙️ **Configuration**

Config file: `~/.percolator/config.json`

```json
{
  "rpc_url": "https://api.devnet.solana.com",
  "wallet_path": "~/.config/solana/id.json",
  "router_program_id": "RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr",
  "slab_program_id": "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk",
  "amm_program_id": "AMMxxx1111111111111111111111111111111111111",
  "oracle_program_id": "ORACxxx111111111111111111111111111111111111"
}
```

---

## 🏗️ **Architecture**

```
cli/
├── src/
│   ├── main.rs           # Entry point & CLI parser
│   ├── commands/         # All command implementations
│   │   ├── lp.rs         # LP operations
│   │   ├── trade.rs      # Trading
│   │   ├── portfolio.rs  # Portfolio management
│   │   ├── admin.rs      # Admin commands
│   │   ├── mm.rs         # Market making
│   │   └── monitor.rs    # Monitoring
│   ├── config.rs         # Configuration management
│   ├── client.rs         # Solana client utilities
│   ├── types.rs          # Common types
│   ├── utils.rs          # Helper functions
│   └── error.rs          # Error types
├── Cargo.toml
├── package.json          # Updated for Rust CLI
└── README.md             # This file
```

---

## 💡 **Examples**

### Create and Setup Slab

```bash
# 1. Check balance
perc balance

# 2. Request airdrop if needed
perc airdrop --amount 5

# 3. Initialize router (first time)
perc admin initialize-router

# 4. Create ETH slab
perc lp create-slab --market ETH-PERP

# 5. Add instrument
perc lp add-instrument -s <SLAB_ADDR> --symbol ETH/USDC --price 3000
```

### Simple Trade

```bash
# 1. Reserve to buy
perc trade reserve -s <ADDRESS> --side buy --qty 0.1 --price 3000

# 2. Commit
perc trade commit -s <ADDRESS> --hold-id 123

# 3. Check positions
perc portfolio positions
```

### Market Making

```bash
# Start bot
perc mm watch -s <ADDRESS> --spread 10 --size 1
```

---

## 🔧 **Development**

```bash
# Build debug
cargo build -p percolator-cli

# Run without building
cargo run -p percolator-cli -- --help

# Build release
cargo build --release -p percolator-cli
```

---

## ✅ **Migration Complete**

- ✅ All commands implemented in Rust
- ✅ Better performance (native binary)
- ✅ Colored output & progress indicators
- ✅ Proper error handling
- ✅ Type safety at compile time

---

## 📝 **License**

Apache-2.0
