# PERColator CLI

Command-line tools for PERColator protocol operations.

## Installation

```bash
npm install -g @percolator/cli
```

Or run locally:

```bash
cd cli
npm install
npm run build
npm link
```

## Usage

```bash
perc --help
```

## Commands

### LP Operations

```bash
# Create new slab
perc lp create-slab --market BTC-PERP --imr 500 --mmr 250

# Add instrument to slab
perc lp add-instrument --slab <ADDRESS> --symbol BTC/USDC --price 65000

# Update slab parameters
perc lp set-params --slab <ADDRESS> --imr 600 --mmr 300
```

### Trading

```bash
# Reserve liquidity
perc trade reserve --slab <ADDRESS> --side buy --qty 1 --price 65000

# Commit reservation
perc trade commit --slab <ADDRESS> --hold-id 12345

# Cancel reservation
perc trade cancel --slab <ADDRESS> --hold-id 12345
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
perc mm quote --slab <ADDRESS> --mid 65000 --spread 10 --size 1

# Run market making bot
perc mm watch --slab <ADDRESS> --spread 10 --size 1
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
# Deploy programs
perc admin deploy --network devnet

# Initialize router
perc admin initialize-router

# Register slab
perc admin register-slab --slab <ADDRESS>
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

## Configuration

Create `~/.percolator/config.json`:

```json
{
  "rpcUrl": "https://api.devnet.solana.com",
  "routerProgramId": "RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr",
  "slabProgramId": "SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk"
}
```

## Wallet

Set wallet path:

```bash
export SOLANA_WALLET=~/.config/solana/id.json
```

Or specify with `--wallet` flag on any command.

## Examples

### Create and Setup Slab

```bash
# 1. Create slab
perc lp create-slab --market ETH-PERP

# 2. Add ETH/USDC instrument
perc lp add-instrument --slab <ADDRESS> --symbol ETH/USDC --price 3000

# 3. Verify
perc portfolio show
```

### Simple Trade

```bash
# 1. Reserve to buy
perc trade reserve --slab <ADDRESS> --side buy --qty 0.1 --price 3000

# 2. Commit (use hold-id from step 1)
perc trade commit --slab <ADDRESS> --hold-id 123

# 3. Check positions
perc portfolio positions
```

### Market Making

```bash
# Start market making bot
perc mm watch --slab <ADDRESS> --spread 10 --size 1
```

## Development

```bash
npm install
npm run build
npm link
```

## License

Apache-2.0

