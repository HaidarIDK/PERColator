/**
 * Percolator Program Configuration
 * Centralized configuration for deployed Solana programs
 */

import { PublicKey } from '@solana/web3.js';

export const PROGRAM_IDS = {
  // Slab Program (v0 deployed on devnet)
  slab: new PublicKey(process.env.NEXT_PUBLIC_SLAB_PROGRAM_ID || 'SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep'),
  
  // Router Program (v0 deployed on devnet)
  router: new PublicKey(process.env.NEXT_PUBLIC_ROUTER_PROGRAM_ID || 'RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr'),
  
  // AMM Program (deployed on devnet)
  amm: new PublicKey(process.env.NEXT_PUBLIC_AMM_PROGRAM_ID || 'AMMjkEeFdasQ8fs9a9HQyJdciPHtDHVEat8yxiXrTP6p'),
  
  // Oracle Program (deployed on devnet)
  oracle: new PublicKey(process.env.NEXT_PUBLIC_ORACLE_PROGRAM_ID || 'oracpooXY8Nnpx2JTLkrLiJsDaMefERUFFRktkAZ3ki'),
} as const;

export const ACCOUNTS = {
  // Slab Account (initialized v0)
  slab: new PublicKey(process.env.NEXT_PUBLIC_SLAB_ACCOUNT || '5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB'),
  
  // Router Registry (initialized v0)
  registry: new PublicKey(process.env.NEXT_PUBLIC_ROUTER_REGISTRY || 'DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx'),
  
  // Instrument ID
  instrument: new PublicKey(process.env.NEXT_PUBLIC_INSTRUMENT_ID || 'G4Um9dNaWKDwd2bhLTEX3DCLRLVWixKvZ1WdEcq6pgfN'),
  
  // Authority
  authority: new PublicKey(process.env.NEXT_PUBLIC_AUTHORITY || 'pErcnK9NSVQQK54BsKV4tUt8YWiKngubpJ7jxHrFtvL'),
} as const;

export const NETWORK = {
  cluster: (process.env.NEXT_PUBLIC_SOLANA_NETWORK || 'devnet') as 'devnet' | 'mainnet-beta' | 'testnet',
  rpcUrl: process.env.NEXT_PUBLIC_SOLANA_RPC_URL || 'https://api.devnet.solana.com',
} as const;

export const API_CONFIG = {
  baseUrl: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001',
} as const;

// Explorer URLs
export const EXPLORERS = {
  solscan: (address: string, cluster: string = 'devnet') => 
    `https://solscan.io/address/${address}?cluster=${cluster}`,
  solanaExplorer: (address: string, cluster: string = 'devnet') =>
    `https://explorer.solana.com/address/${address}?cluster=${cluster}`,
  transaction: (signature: string, cluster: string = 'devnet') =>
    `https://solscan.io/tx/${signature}?cluster=${cluster}`,
} as const;

// Market Configuration
export const MARKET_CONFIG = {
  markPrice: 100_000_000, // $100 in 1e6 scale
  contractSize: 1_000_000, // 1.0 in 1e6 scale
  takerFeeBps: 5, // 5 basis points = 0.05%
  tickSize: 1_000_000, // $1.00
  lotSize: 1_000_000, // 1.0
} as const;

// Display helpers
export const formatAddress = (address: PublicKey | string): string => {
  const addr = typeof address === 'string' ? address : address.toBase58();
  return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
};

export const formatPrice = (price: number, decimals: number = 2): string => {
  return `$${price.toFixed(decimals)}`;
};

export const formatAmount = (amount: number, decimals: number = 4): string => {
  return amount.toFixed(decimals);
};

