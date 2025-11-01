// Solana program configuration for devnet
import { PublicKey } from '@solana/web3.js';
import configData from '../../../../keypairs/config.json';

export const NETWORK = 'devnet';
export const RPC_ENDPOINT = 'https://api.devnet.solana.com';

// Program IDs from deployed programs
export const ROUTER_PROGRAM_ID = new PublicKey(configData.programs.router.program_id);
export const SLAB_PROGRAM_ID = new PublicKey(configData.programs.slab.program_id);
export const AMM_PROGRAM_ID = new PublicKey(configData.programs.amm.program_id);

// Derive registry address (created with createWithSeed, not PDA!)
const DEV_WALLET = new PublicKey(configData.keypairs.devwallet.pubkey);
export async function getRegistryAddress(): Promise<PublicKey> {
  return await PublicKey.createWithSeed(
    DEV_WALLET,
    'registry',
    ROUTER_PROGRAM_ID
  );
}

// Slab accounts (order books)
export const SLABS = {
  'SOL': configData.trading_pairs['SOL-USD'].slab_address 
    ? new PublicKey(configData.trading_pairs['SOL-USD'].slab_address) 
    : null,
  'ETH': configData.trading_pairs['ETH-USD'].slab_address 
    ? new PublicKey(configData.trading_pairs['ETH-USD'].slab_address) 
    : null,
  'BTC': configData.trading_pairs['BTC-USD'].slab_address 
    ? new PublicKey(configData.trading_pairs['BTC-USD'].slab_address) 
    : null,
};

// Oracle accounts (price feeds)
export const ORACLES = {
  'SOL': configData.oracles?.['SOL-USD']?.address 
    ? new PublicKey(configData.oracles['SOL-USD'].address) 
    : null,
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

