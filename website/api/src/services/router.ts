/**
 * Router Program Transaction Builders
 * Manages user portfolios, collateral deposits/withdrawals, and cross-slab execution
 */

import { 
  Transaction, 
  PublicKey, 
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import BN from 'bn.js';

// Router Program ID (set via environment variable)
// Will be set when you deploy your new router program
export const ROUTER_PROGRAM_ID = process.env.ROUTER_PROGRAM_ID 
  ? new PublicKey(process.env.ROUTER_PROGRAM_ID)
  : null;

// Slab Program ID (set via environment variable)
// Will be set when you deploy your new slab program
export const SLAB_PROGRAM_ID = process.env.SLAB_PROGRAM_ID
  ? new PublicKey(process.env.SLAB_PROGRAM_ID)
  : null;

// Slab Account (set via environment variable)
// Will be created when you initialize your new slab
export const SLAB_ACCOUNT = process.env.SLAB_ACCOUNT
  ? new PublicKey(process.env.SLAB_ACCOUNT)
  : null;

/**
 * Router instruction discriminators
 */
export enum RouterInstruction {
  Initialize = 0,
  Deposit = 1,
  Withdraw = 2,
  MultiReserve = 3,
  MultiCommit = 4,
  LockCap = 5,
  UnlockCap = 6,
  Liquidate = 7,
}

/**
 * Derive Portfolio PDA for a user
 */
export function derivePortfolioPDA(
  userAuthority: PublicKey,
  routerProgramId: PublicKey = ROUTER_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from('portfolio'),
      userAuthority.toBuffer(),
    ],
    routerProgramId
  );
}

/**
 * Build a Deposit instruction
 * Deposits SOL/USDC collateral into user's Router portfolio
 */
export function buildDepositInstruction(params: {
  userAuthority: PublicKey;
  amount: number; // In lamports or smallest token units
}): TransactionInstruction {
  const { userAuthority, amount } = params;

  // Derive portfolio PDA
  const [portfolioPDA, bump] = derivePortfolioPDA(userAuthority);

  // Use the Vault account created during Router initialization
  const VAULT_ACCOUNT = new PublicKey(
    process.env.ROUTER_VAULT_ACCOUNT || 'EBpPx3bNvRwnvrfeF34KiRm33mBWwaCGtNDycJLKdWxg'
  );

  // Mock USDC mint for devnet (using native SOL for now)
  const SOL_MINT = PublicKey.default; // Will use native SOL

  // Build instruction data
  // Format: [discriminator(1), mint(32), amount(16)]
  const data = Buffer.alloc(49);
  let offset = 0;

  // Discriminator
  data.writeUInt8(RouterInstruction.Deposit, offset);
  offset += 1;

  // Mint (32 bytes) - using default pubkey for SOL
  SOL_MINT.toBuffer().copy(data, offset);
  offset += 32;

  // Amount (u128 - 16 bytes)
  const amountBN = new BN(amount);
  amountBN.toArrayLike(Buffer, 'le', 16).copy(data, offset);

  // Build accounts array per Router program expectation:
  // 0. Vault account (writable)
  // 1. User token account (writable) - the user's SOL wallet
  // 2. User portfolio account (writable)
  // 3. User authority (signer)
  const keys = [
    { pubkey: VAULT_ACCOUNT, isSigner: false, isWritable: true }, // 0: Vault
    { pubkey: userAuthority, isSigner: false, isWritable: true }, // 1: User token account (their wallet holds SOL)
    { pubkey: portfolioPDA, isSigner: false, isWritable: true }, // 2: Portfolio
    { pubkey: userAuthority, isSigner: true, isWritable: false }, // 3: User authority (signer)
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Build a Withdraw instruction
 * Withdraws SOL collateral from user's Router portfolio
 */
export function buildWithdrawInstruction(params: {
  userAuthority: PublicKey;
  amount: number; // In lamports
}): TransactionInstruction {
  const { userAuthority, amount } = params;

  // Derive portfolio PDA
  const [portfolioPDA, bump] = derivePortfolioPDA(userAuthority);

  // Build instruction data
  // Format: [discriminator(1), amount(8), bump(1)]
  const data = Buffer.alloc(10);
  let offset = 0;

  // Discriminator
  data.writeUInt8(RouterInstruction.Withdraw, offset);
  offset += 1;

  // Amount
  const amountBN = new BN(amount);
  amountBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Bump
  data.writeUInt8(bump, offset);

  // Build accounts array
  const keys = [
    { pubkey: portfolioPDA, isSigner: false, isWritable: true }, // Portfolio account
    { pubkey: userAuthority, isSigner: true, isWritable: true }, // User (receives SOL)
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Build ExecuteCrossSlab instruction
 * Executes a trade across one or more slabs via Router
 */
export function buildExecuteCrossSlabInstruction(params: {
  userAuthority: PublicKey;
  slabAccount: PublicKey;
  instrumentIdx: number;
  side: 'buy' | 'sell';
  quantity: number;
  limitPrice: number;
}): TransactionInstruction {
  const { userAuthority, slabAccount, instrumentIdx, side, quantity, limitPrice } = params;

  // Derive portfolio PDA
  const [portfolioPDA, bump] = derivePortfolioPDA(userAuthority);

  // Build instruction data
  // Format: [discriminator(1), instrument_idx(2), side(1), qty(8), limit_px(8), bump(1)]
  const data = Buffer.alloc(21);
  let offset = 0;

  // Discriminator - using MultiCommit as ExecuteCrossSlab
  data.writeUInt8(RouterInstruction.MultiCommit, offset);
  offset += 1;

  // Instrument index
  data.writeUInt16LE(instrumentIdx, offset);
  offset += 2;

  // Side (0 = Buy, 1 = Sell)
  data.writeUInt8(side === 'buy' ? 0 : 1, offset);
  offset += 1;

  // Quantity (in base units)
  const qtyBN = new BN(Math.floor(quantity * 1e9)); // Convert to smallest unit
  qtyBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Limit price (in price units)
  const priceBN = new BN(Math.floor(limitPrice * 1e9));
  priceBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Bump
  data.writeUInt8(bump, offset);

  // Build accounts array
  const keys = [
    { pubkey: portfolioPDA, isSigner: false, isWritable: true }, // Portfolio (manages positions)
    { pubkey: slabAccount, isSigner: false, isWritable: true }, // Slab to trade on
    { pubkey: userAuthority, isSigner: true, isWritable: false }, // User signature
    { pubkey: SLAB_PROGRAM_ID, isSigner: false, isWritable: false }, // Slab program for CPI
  ];

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Check if a portfolio exists for a user
 */
export async function getPortfolio(
  connection: any,
  userAuthority: PublicKey
): Promise<any | null> {
  try {
    const [portfolioPDA] = derivePortfolioPDA(userAuthority);
    const accountInfo = await connection.getAccountInfo(portfolioPDA);
    
    if (!accountInfo) {
      return null;
    }

    // TODO: Parse portfolio data structure
    // For now, return raw data
    return {
      address: portfolioPDA.toBase58(),
      exists: true,
      lamports: accountInfo.lamports,
      dataSize: accountInfo.data.length,
      owner: accountInfo.owner.toBase58(),
    };
  } catch (error) {
    console.error('Error fetching portfolio:', error);
    return null;
  }
}

/**
 * Parse portfolio account data
 */
export function parsePortfolioData(data: Buffer): any {
  // Portfolio structure (from programs/router/src/state/portfolio.rs):
  // - authority: Pubkey (32 bytes)
  // - cash: i128 (16 bytes)
  // - position_count: u16 (2 bytes)
  // - positions: [Position; MAX_POSITIONS]
  
  if (data.length < 50) {
    return null;
  }

  let offset = 0;

  // Authority
  const authority = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;

  // Cash balance (i128 - signed 128-bit integer)
  const cashBytes = data.slice(offset, offset + 16);
  const cash = cashBytes.readBigInt64LE(0); // Simplified - proper i128 parsing needed
  offset += 16;

  // Position count
  const positionCount = data.readUInt16LE(offset);
  offset += 2;

  return {
    authority: authority.toBase58(),
    cash: Number(cash) / 1e9, // Convert to decimal
    positionCount,
    // TODO: Parse positions array
  };
}

export default {
  buildDepositInstruction,
  buildWithdrawInstruction,
  buildExecuteCrossSlabInstruction,
  derivePortfolioPDA,
  getPortfolio,
  parsePortfolioData,
};

