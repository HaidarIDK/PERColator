import { PublicKey } from '@solana/web3.js';

export const NETWORK = 'devnet';
export const RPC_ENDPOINT = 'https://api.devnet.solana.com';

// Deployed Program IDs from keypairs/config.json
export const PROGRAM_IDS = {
  ROUTER: new PublicKey('rPB77V5pFZ3dzwgmUH5jvXDwzK7FTqs92nyFgBecxMh'),
  SLAB: new PublicKey('sDoE9Fb3H516S5ZuxBD1SUFgP1Z7ddXBpi9fweZb6Mk'),
  AMM: new PublicKey('aMepWm9uGGtMGpufmUs8xKAVV8ES8QLPHXg1612RCmz'),
} as const;

// Trading pairs configuration
export const TRADING_PAIRS = {
  'SOL-USD': {
    symbol: 'SOL-USD',
    displayName: 'SOL/USDC',
    description: 'SOL/USDC perpetual',
  },
  'ETH-USD': {
    symbol: 'ETH-USD',
    displayName: 'ETH/USDC',
    description: 'ETH/USDC perpetual',
  },
  'BTC-USD': {
    symbol: 'BTC-USD',
    displayName: 'BTC/USDC',
    description: 'BTC/USDC perpetual',
  },
} as const;

// Program-derived address seeds (from router/src/pda.rs)
export const PDA_SEEDS = {
  AUTHORITY: 'authority',
  REGISTRY: 'registry',
  PORTFOLIO: 'portfolio',
  VAULT: 'vault',
  ESCROW: 'escrow',
} as const;

