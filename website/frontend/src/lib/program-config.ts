// Solana program configuration for devnet
import { PublicKey } from '@solana/web3.js';

export const NETWORK = 'devnet';
export const RPC_ENDPOINT = 'https://api.devnet.solana.com';

// Program IDs from deployed programs (from config.json)
export const ROUTER_PROGRAM_ID = new PublicKey('rPB77V5pFZ3dzwgmUH5jvXDwzK7FTqs92nyFgBecxMh');
export const SLAB_PROGRAM_ID = new PublicKey('sDoE9Fb3H516S5ZuxBD1SUFgP1Z7ddXBpi9fweZb6Mk');
export const AMM_PROGRAM_ID = new PublicKey('aMepWm9uGGtMGpufmUs8xKAVV8ES8QLPHXg1612RCmz');

// Dev wallet for registry
const DEV_WALLET = new PublicKey('dMJN4LVNZyURkXwfE5rv3c3dnD6iEeZBVXHdVxKrXPi');

// Derive registry address (created with createWithSeed, not PDA!)
export async function getRegistryAddress(): Promise<PublicKey> {
  return await PublicKey.createWithSeed(
    DEV_WALLET,
    'registry',
    ROUTER_PROGRAM_ID
  );
}

// Slab accounts (order books) - from config.json
export const SLABS = {
  'SOL': new PublicKey('7pyCSG18qBXipgUCApPxBo48T3a1M7arhrf1AbkMtAFL'), // SOL-USD slab
  'ETH': null, // Not created yet
  'BTC': null, // Not created yet
};

// Oracle accounts (price feeds) - from config.json
export const ORACLES = {
  'SOL': new PublicKey('6YkiHPMfmr3xaxgH13anHr9V4puoRuS15LBpSeto1rDF'), // SOL-USD oracle
  'ETH': null, // Not created yet
  'BTC': null, // Not created yet
};

// PDA seeds (must match router program)
export const AUTHORITY_SEED = Buffer.from('authority');
export const REGISTRY_SEED = Buffer.from('registry');
export const PORTFOLIO_SEED = Buffer.from('portfolio');
export const VAULT_SEED = Buffer.from('vault');
export const RECEIPT_SEED = Buffer.from('receipt');

// Derive portfolio address for a user (using createWithSeed, NOT PDA!)
export async function derivePortfolioAddress(userPubkey: PublicKey): Promise<PublicKey> {
  return await PublicKey.createWithSeed(
    userPubkey,
    'portfolio',
    ROUTER_PROGRAM_ID
  );
}

// Legacy PDA function (not used by router, kept for reference)
export function derivePortfolioPDA(userPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [PORTFOLIO_SEED, userPubkey.toBuffer()],
    ROUTER_PROGRAM_ID
  );
}

// Native SOL mint (wrapped SOL)
export const NATIVE_SOL_MINT = new PublicKey('So11111111111111111111111111111111111111112');

// Derive vault PDA for a specific mint
export function deriveVaultPDA(mint: PublicKey = NATIVE_SOL_MINT): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, mint.toBuffer()],
    ROUTER_PROGRAM_ID
  );
}

// Derive registry address (using createWithSeed, NOT PDA!)
export async function deriveRegistryAddress(devWalletPubkey: PublicKey): Promise<PublicKey> {
  return await PublicKey.createWithSeed(
    devWalletPubkey,
    'registry',
    ROUTER_PROGRAM_ID
  );
}

// Legacy PDA function (NOT used for registry account!)
export function deriveRegistryPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [REGISTRY_SEED],
    ROUTER_PROGRAM_ID
  );
}

// Derive router authority PDA
export function deriveRouterAuthorityPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [AUTHORITY_SEED],
    ROUTER_PROGRAM_ID
  );
}

// Derive receipt PDA for a portfolio+slab
export function deriveReceiptPDA(portfolioPDA: PublicKey, slabPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [RECEIPT_SEED, portfolioPDA.toBuffer(), slabPubkey.toBuffer()],
    ROUTER_PROGRAM_ID
  );
}

// Helper to get slab for coin
export function getSlabForCoin(coin: 'SOL' | 'ETH' | 'BTC'): PublicKey | null {
  return SLABS[coin];
}

// Helper to get oracle for coin
export function getOracleForCoin(coin: 'SOL' | 'ETH' | 'BTC'): PublicKey | null {
  return ORACLES[coin];
}

