# PERColator TypeScript SDK

Official TypeScript SDK for interacting with the PERColator perpetual exchange protocol on Solana.

## Installation

```bash
npm install @percolator/sdk @solana/web3.js
```

## Quick Start

```typescript
import { Connection, Keypair } from '@solana/web3.js';
import { PercolatorClient, Side, priceToProtocol, qtyToProtocol } from '@percolator/sdk';
import BN from 'bn.js';

// Connect to devnet
const connection = new Connection('https://api.devnet.solana.com');

// Program IDs (replace with your deployed program IDs)
const ROUTER_PROGRAM_ID = new PublicKey('RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr');
const SLAB_PROGRAM_ID = new PublicKey('SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk');

// Initialize client
const client = new PercolatorClient(
  connection,
  ROUTER_PROGRAM_ID,
  SLAB_PROGRAM_ID
);

// Your wallet
const wallet = Keypair.fromSecretKey(/* your secret key */);

// Get slab address for BTC-PERP
const btcSlab = client.getSlabAddress('BTC-PERP');

// Reserve liquidity to buy 1 BTC at $65,000
const reserveResult = await client.reserve(
  btcSlab,
  wallet,
  0,  // account index
  0,  // instrument index (BTC)
  Side.Buy,
  qtyToProtocol(1),  // 1 BTC
  priceToProtocol(65000),  // $65,000 limit
  60000  // 60 second TTL
);

// Commit the reservation
const commitResult = await client.commit(
  btcSlab,
  wallet,
  reserveResult.holdId
);

console.log(`Filled ${commitResult.filledQty} at avg price ${commitResult.avgPrice}`);
```

## Features

### Trading Operations

```typescript
// Reserve + Commit (two-phase execution)
const holdId = await client.reserve(...);
await client.commit(slabState, user, holdId);

// Cancel reservation
await client.cancel(slabState, user, holdId);

// Open new batch
await client.openBatch(slabState, authority, instrumentIdx);
```

### Collateral Management

```typescript
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from '@solana/spl-token';

const usdcMint = new PublicKey('...');
const userTokenAccount = await getAssociatedTokenAddress(usdcMint, wallet.publicKey);

// Deposit 1000 USDC
await client.deposit(
  usdcMint,
  new BN(1000_000_000),  // 1000 USDC (6 decimals)
  wallet,
  userTokenAccount
);

// Withdraw 500 USDC
await client.withdraw(
  usdcMint,
  new BN(500_000_000),
  wallet,
  userTokenAccount
);
```

### Portfolio & Positions

```typescript
// Get user's cross-margin portfolio
const portfolio = await client.getPortfolio(wallet.publicKey);

console.log(`Equity: ${portfolio.equity}`);
console.log(`Free Collateral: ${portfolio.freeCollateral}`);
console.log(`Positions:`, portfolio.exposures);
```

### Market Data

```typescript
// Get orderbook
const orderbook = await client.getOrderbook(btcSlab, 0);

console.log('Bids:', orderbook.bids);
console.log('Asks:', orderbook.asks);
```

### Slab Management (LP Operations)

```typescript
// Initialize new slab
const config = {
  authority: wallet.publicKey,
  oracle: oracleId,
  router: ROUTER_PROGRAM_ID,
  imr: 500,  // 5% initial margin
  mmr: 250,  // 2.5% maintenance margin
  makerFee: -5,  // -0.05% rebate
  takerFee: 20,  // 0.2% fee
  batchMs: 100,  // 100ms batches
  freezeLevels: 3,  // Freeze top 3 levels
};

const slabPDA = await client.initializeSlab('ETH-PERP', config, wallet);

// Add instrument
await client.addInstrument(
  slabPDA,
  wallet,
  'ETH/USDC',
  qtyToProtocol(1),  // Contract size
  priceToProtocol(0.01),  // $0.01 tick
  qtyToProtocol(0.001),  // 0.001 lot size
  priceToProtocol(3000)  // $3000 index price
);
```

## API Reference

### Client Methods

#### `reserve()`
Lock liquidity from the orderbook without executing

**Parameters:**
- `slabState` - Slab account address
- `user` - User keypair
- `accountIdx` - Account index in slab
- `instrumentIdx` - Instrument index
- `side` - Buy or Sell
- `qty` - Quantity to reserve
- `limitPx` - Limit price
- `ttlMs` - Time-to-live in milliseconds
- `routeId` - Optional route identifier

**Returns:** `ReserveResult`

#### `commit()`
Execute trades at reserved prices

**Parameters:**
- `slabState` - Slab account address
- `user` - User keypair
- `holdId` - Reservation ID from reserve()

**Returns:** `CommitResult`

#### `cancel()`
Release a reservation

**Parameters:**
- `slabState` - Slab account address
- `user` - User keypair
- `holdId` - Reservation ID

**Returns:** `void`

#### `deposit()`
Deposit collateral to router vault

**Parameters:**
- `mint` - Token mint address
- `amount` - Amount to deposit
- `user` - User keypair
- `userTokenAccount` - User's token account

**Returns:** `void`

#### `withdraw()`
Withdraw collateral from router vault

**Parameters:**
- `mint` - Token mint address
- `amount` - Amount to withdraw
- `user` - User keypair
- `userTokenAccount` - User's token account

**Returns:** `void`

### Utility Functions

```typescript
import {
  priceToProtocol,
  priceFromProtocol,
  qtyToProtocol,
  qtyFromProtocol,
  calculateVWAP,
  calculatePnL,
  lamportsToSOL,
  solToLamports,
  bpsToPercent,
  percentToBps,
} from '@percolator/sdk';
```

### PDA Helpers

```typescript
import {
  deriveSlabPDA,
  deriveVaultPDA,
  deriveEscrowPDA,
  derivePortfolioPDA,
  deriveCapPDA,
  deriveRegistryPDA,
} from '@percolator/sdk';

const [slabPDA, bump] = deriveSlabPDA('BTC-PERP', SLAB_PROGRAM_ID);
```

## Examples

See `/examples` directory for complete examples:
- `basic-trading.ts` - Simple reserve/commit flow
- `market-making.ts` - LP operations
- `portfolio-management.ts` - Multi-position tracking
- `liquidation-bot.ts` - Automated liquidations

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Watch mode
npm run watch

# Run tests
npm test
```

## License

Apache-2.0

