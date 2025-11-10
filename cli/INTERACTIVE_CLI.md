# Percolator Interactive CLI

A user-friendly, menu-driven CLI for testing and interacting with the Percolator protocol on Solana.

## Features

- **Menu-driven interface** - Easy navigation through all protocol features
- **Balance checking** - Automatically checks SOL balance and prompts for airdrop on devnet
- **Guided workflows** - Step-by-step prompts for all operations
- **Complete protocol coverage** - Test all programs: Router, Slab, AMM, Oracle
- **Real-time status** - View portfolios, orderbooks, and system status

## Quick Start

### Prerequisites

1. **Solana CLI installed** - [Installation Guide](https://docs.solana.com/cli/install-solana-cli-tools)
2. **Keypair configured** - Default keypair at `~/.config/solana/id.json`
3. **Devnet SOL** - Get free devnet SOL for testing:
   ```bash
   solana airdrop 2
   ```

### Running the Interactive CLI

```bash
# On devnet (recommended for testing)
cargo run --release --bin percolator -- -n devnet interactive

# On localnet (requires local validator)
solana-test-validator &
cargo run --release --bin percolator -- -n localnet interactive

# With custom keypair
cargo run --release --bin percolator -- -n devnet -k ~/path/to/keypair.json interactive
```

## Main Menu

The interactive CLI provides the following main menu options:

1. **Setup & Deployment** - Initialize exchange and portfolio
2. **Slab Operations** - Create and manage trading slabs
3. **Trading** - Place orders, view orderbooks
4. **Margin & Portfolio** - Manage collateral and positions
5. **AMM Operations** - Create and manage AMM pools
6. **Liquidity Operations** - Add/remove liquidity
7. **Status & Info** - View system status and accounts
8. **Run Tests** - Execute test suites

## Workflows

### 1. Setup & Deployment

**Initialize Exchange:**
- Creates router registry
- Sets up insurance fund
- Configures margin parameters

**Initialize Portfolio:**
- Creates user portfolio account
- Required before trading
- One-time setup per user

### 2. Slab Operations

**Create Slab:**
- Creates a new orderbook slab
- Configures tick size and lot size
- Returns slab address for trading

**Register Slab:**
- Registers slab in router registry
- Configures margin requirements
- Sets fee parameters

**Manage Slab:**
- View orderbook
- Place/cancel orders
- Update funding rates
- Halt/resume trading

### 3. Trading

**Place Orders:**
- Limit orders (via router)
- Market orders (via router)
- Resting orders (directly on slab)

**View Orderbook:**
- Real-time orderbook depth
- Best bid/ask prices
- Order quantities

**Manage Orders:**
- Cancel orders
- Modify orders
- List open orders

### 4. Margin & Portfolio

**Deposit/Withdraw:**
- Deposit SOL collateral
- Withdraw collateral (if margin allows)
- View portfolio balance

**View Portfolio:**
- Equity and PnL
- Margin requirements
- Open positions
- Health status

### 5. AMM Operations

**Create AMM Pool:**
- Initialize constant product AMM
- Set initial reserves
- Configure trading pair

**List Pools:**
- View all AMM pools
- Check liquidity
- View fees

### 6. Liquidity Operations

**Add Liquidity:**
- AMM mode: Add to liquidity pool
- Orderbook mode: Place maker orders
- Configure price ranges

**Remove Liquidity:**
- Withdraw from AMM pool
- Cancel maker orders
- View LP positions

### 7. Status & Info

**Registry Status:**
- View router configuration
- Check margin parameters
- View insurance fund

**Portfolio Status:**
- Account balance
- Positions
- Margin health

**Balance Check:**
- SOL balance
- Airdrop instructions (devnet)

## Example Workflow

```bash
# 1. Start interactive CLI
cargo run --release --bin percolator -- -n devnet interactive

# 2. Setup (Menu option 1)
#    - Initialize Exchange
#    - Initialize Portfolio

# 3. Create Slab (Menu option 2)
#    - Create new slab
#    - Register slab in router

# 4. Deposit Collateral (Menu option 4)
#    - Deposit 1 SOL

# 5. Place Order (Menu option 3)
#    - Place limit buy order
#    - View orderbook

# 6. Check Status (Menu option 7)
#    - View portfolio
#    - Check positions
```

## Network Configuration

### Devnet (Recommended for Testing)

```bash
# Set Solana CLI to devnet
solana config set --url devnet

# Get devnet SOL
solana airdrop 2

# Run CLI
cargo run --release --bin percolator -- -n devnet interactive
```

### Localnet (Development)

```bash
# Start local validator
solana-test-validator &

# Set Solana CLI to localnet
solana config set --url localhost

# Run CLI
cargo run --release --bin percolator -- -n localnet interactive
```

### Mainnet (Production)

⚠️ **Warning:** Only use with real funds after thorough testing!

```bash
# Set Solana CLI to mainnet
solana config set --url mainnet-beta

# Run CLI
cargo run --release --bin percolator -- -n mainnet-beta interactive
```

## Tips

1. **Start with Devnet** - Test everything on devnet first
2. **Check Balance** - Ensure you have enough SOL for transactions
3. **Initialize Portfolio** - Required before any trading
4. **Use Default Values** - Most prompts have sensible defaults
5. **View Status** - Check portfolio and orderbook frequently
6. **Test Gradually** - Start with small amounts

## Troubleshooting

### "Insufficient balance"
- Get more devnet SOL: `solana airdrop 2`
- Check balance in Status menu

### "Portfolio not initialized"
- Run "Initialize Portfolio" in Setup menu
- One-time setup per user

### "Slab not found"
- Create slab first in Slab Operations menu
- Verify slab address is correct

### "Transaction failed"
- Check SOL balance
- Verify network connection
- Check program deployment status
- Review error messages

## Advanced Usage

### Custom Keypair

```bash
cargo run --release --bin percolator -- \
  -n devnet \
  -k ~/path/to/keypair.json \
  interactive
```

### Custom RPC URL

```bash
cargo run --release --bin percolator -- \
  -n devnet \
  --url https://api.devnet.solana.com \
  interactive
```

### Verbose Output

```bash
cargo run --release --bin percolator -- \
  -n devnet \
  -v \
  interactive
```

## Non-Interactive Mode

For automation and scripting, use the non-interactive CLI commands:

```bash
# Initialize exchange
cargo run --release --bin percolator -- -n devnet init --name test

# Initialize portfolio
cargo run --release --bin percolator -- -n devnet margin init

# Create slab
cargo run --release --bin percolator -- -n devnet matcher create \
  <exchange> BTC-USD --tick-size 1000000 --lot-size 1000000

# Place order
cargo run --release --bin percolator -- -n devnet trade limit \
  <slab> buy 100.0 1.0
```

See `cargo run --release --bin percolator -- --help` for all commands.

## Support

For issues and questions:
- Check the [main README](../README.md)
- Review [documentation](../docs/)
- Open an issue on GitHub

## License

Apache-2.0

