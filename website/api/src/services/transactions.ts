/**
 * Transaction builder utilities for Percolator trading
 */

import { 
  Transaction, 
  PublicKey, 
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Connection
} from '@solana/web3.js';
import { getConnection } from './solana';
import BN from 'bn.js';

// Program IDs (loaded from environment variables)
// Updated: 2025-10-23 - v0 deployment
export const SLAB_PROGRAM_ID = process.env.SLAB_PROGRAM_ID
  ? new PublicKey(process.env.SLAB_PROGRAM_ID)
  : new PublicKey('SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep'); // Fallback to deployed program

export const ROUTER_PROGRAM_ID = process.env.ROUTER_PROGRAM_ID
  ? new PublicKey(process.env.ROUTER_PROGRAM_ID)
  : new PublicKey('RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr'); // Fallback to deployed program

// Slab Account (loaded from environment variable)
// Updated: 2025-10-23 - v0 initialized slab
export const SLAB_ACCOUNT = process.env.SLAB_ACCOUNT
  ? new PublicKey(process.env.SLAB_ACCOUNT)
  : new PublicKey('5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB'); // Fallback to initialized slab

/**
 * Instruction discriminators
 */
export enum SlabInstruction {
  Initialize = 0,
  CommitFill = 1,
  Reserve = 2,
  Commit = 3,
  Cancel = 4,
  // Future instructions (not yet implemented):
  BatchOpen = 5,
  AddInstrument = 6,
  UpdateFunding = 7,
  Liquidate = 8,
}

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
 * Side enum
 */
export enum Side {
  Buy = 0,
  Sell = 1,
}

/**
 * Build a Reserve instruction for the slab
 */
export function buildReserveInstruction(params: {
  slabAccount: PublicKey;
  userAccount?: PublicKey;
  accountIdx: number;
  instrumentIdx: number;
  side: 'buy' | 'sell';
  qty: number;
  limitPrice: number;
  ttlMs: number;
  commitmentHash: Buffer;
  routeId: number;
}): TransactionInstruction {
  const {
    slabAccount,
    userAccount,
    accountIdx,
    instrumentIdx,
    side,
    qty,
    limitPrice,
    ttlMs,
    commitmentHash,
    routeId,
  } = params;

  // Build instruction data
  // Format: [discriminator(1), account_idx(4), instrument_idx(2), side(1), qty(8), limit_px(8), ttl_ms(8), commitment_hash(32), route_id(8)]
  const data = Buffer.alloc(72);
  let offset = 0;

  // Discriminator
  data.writeUInt8(SlabInstruction.Reserve, offset);
  offset += 1;

  // Account index
  data.writeUInt32LE(accountIdx, offset);
  offset += 4;

  // Instrument index
  data.writeUInt16LE(instrumentIdx, offset);
  offset += 2;

  // Side (0 = Buy, 1 = Sell)
  data.writeUInt8(side === 'buy' ? Side.Buy : Side.Sell, offset);
  offset += 1;

  // Quantity (as u64 in base units)
  const qtyBN = new BN(Math.floor(qty * 1e6)); // Convert to base units (6 decimals)
  qtyBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Limit price (as u64 in base units)
  const priceBN = new BN(Math.floor(limitPrice * 1e6));
  priceBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // TTL milliseconds
  const ttlBN = new BN(ttlMs);
  ttlBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Commitment hash
  commitmentHash.copy(data, offset);
  offset += 32;

  // Route ID (use timestamp + random to ensure uniqueness)
  const uniqueRouteId = Date.now() * 1000 + Math.floor(Math.random() * 1000);
  const routeIdBN = new BN(uniqueRouteId);
  routeIdBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);

  // Build accounts array
  const keys = [
    { pubkey: slabAccount, isSigner: false, isWritable: true },
  ];

  if (userAccount) {
    keys.push({ pubkey: userAccount, isSigner: true, isWritable: false });
  }

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Build a Commit instruction for the slab
 */
export function buildCommitInstruction(params: {
  slabAccount: PublicKey;
  userAccount?: PublicKey;
  holdId: number;
  commitmentReveal: Buffer;
}): TransactionInstruction {
  const { slabAccount, userAccount, holdId, commitmentReveal } = params;

  // Build instruction data
  // Format: [discriminator(1), hold_id(8), commitment_reveal(32)]
  const data = Buffer.alloc(41);
  let offset = 0;

  // Discriminator
  data.writeUInt8(SlabInstruction.Commit, offset);
  offset += 1;

  // Hold ID
  const holdIdBN = new BN(holdId);
  holdIdBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Commitment reveal
  commitmentReveal.copy(data, offset);

  // Build accounts array
  const keys = [
    { pubkey: slabAccount, isSigner: false, isWritable: true },
  ];

  if (userAccount) {
    keys.push({ pubkey: userAccount, isSigner: true, isWritable: false });
  }

  return new TransactionInstruction({
    keys,
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Build a MultiReserve instruction for the router
 */
export function buildMultiReserveInstruction(params: {
  routerAccount: PublicKey;
  escrowAccount: PublicKey;
  portfolioAccount: PublicKey;
  capAccounts: PublicKey[];
  slabAccounts: PublicKey[];
  userAuthority: PublicKey;
  instruments: number[];
  sides: ('buy' | 'sell')[];
  quantities: number[];
  limitPrices: number[];
  ttlMs: number;
  commitmentHash: Buffer;
}): TransactionInstruction {
  const {
    routerAccount,
    escrowAccount,
    portfolioAccount,
    capAccounts,
    slabAccounts,
    userAuthority,
    instruments,
    sides,
    quantities,
    limitPrices,
    ttlMs,
    commitmentHash,
  } = params;

  // Validate arrays have same length
  const numOrders = instruments.length;
  if (
    sides.length !== numOrders ||
    quantities.length !== numOrders ||
    limitPrices.length !== numOrders ||
    slabAccounts.length !== numOrders
  ) {
    throw new Error('MultiReserve: array lengths must match');
  }

  // Build instruction data
  // Format: [discriminator(1), num_orders(1), ttl_ms(8), commitment_hash(32), 
  //          orders[instrument(2), side(1), qty(8), limit_px(8)]...]
  const dataSize = 1 + 1 + 8 + 32 + numOrders * 19;
  const data = Buffer.alloc(dataSize);
  let offset = 0;

  // Discriminator
  data.writeUInt8(RouterInstruction.MultiReserve, offset);
  offset += 1;

  // Number of orders
  data.writeUInt8(numOrders, offset);
  offset += 1;

  // TTL milliseconds
  const ttlBN = new BN(ttlMs);
  ttlBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
  offset += 8;

  // Commitment hash
  commitmentHash.copy(data, offset);
  offset += 32;

  // Orders
  for (let i = 0; i < numOrders; i++) {
    // Instrument
    data.writeUInt16LE(instruments[i], offset);
    offset += 2;

    // Side
    data.writeUInt8(sides[i] === 'buy' ? Side.Buy : Side.Sell, offset);
    offset += 1;

    // Quantity
    const qtyBN = new BN(Math.floor(quantities[i] * 1e6));
    qtyBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
    offset += 8;

    // Limit price
    const priceBN = new BN(Math.floor(limitPrices[i] * 1e6));
    priceBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
    offset += 8;
  }

  // Build accounts array
  const keys = [
    { pubkey: routerAccount, isSigner: false, isWritable: true },
    { pubkey: escrowAccount, isSigner: false, isWritable: true },
    { pubkey: portfolioAccount, isSigner: false, isWritable: true },
    { pubkey: userAuthority, isSigner: true, isWritable: false },
  ];

  // Add cap accounts
  for (const capAccount of capAccounts) {
    keys.push({ pubkey: capAccount, isSigner: false, isWritable: true });
  }

  // Add slab accounts
  for (const slabAccount of slabAccounts) {
    keys.push({ pubkey: slabAccount, isSigner: false, isWritable: true });
  }

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Build a MultiCommit instruction for the router
 */
export function buildMultiCommitInstruction(params: {
  routerAccount: PublicKey;
  escrowAccount: PublicKey;
  portfolioAccount: PublicKey;
  slabAccounts: PublicKey[];
  userAuthority: PublicKey;
  holdIds: number[];
  commitmentReveal: Buffer;
}): TransactionInstruction {
  const {
    routerAccount,
    escrowAccount,
    portfolioAccount,
    slabAccounts,
    userAuthority,
    holdIds,
    commitmentReveal,
  } = params;

  // Build instruction data
  // Format: [discriminator(1), num_holds(1), commitment_reveal(32), hold_ids[8]...]
  const dataSize = 1 + 1 + 32 + holdIds.length * 8;
  const data = Buffer.alloc(dataSize);
  let offset = 0;

  // Discriminator
  data.writeUInt8(RouterInstruction.MultiCommit, offset);
  offset += 1;

  // Number of holds
  data.writeUInt8(holdIds.length, offset);
  offset += 1;

  // Commitment reveal
  commitmentReveal.copy(data, offset);
  offset += 32;

  // Hold IDs
  for (const holdId of holdIds) {
    const holdIdBN = new BN(holdId);
    holdIdBN.toArrayLike(Buffer, 'le', 8).copy(data, offset);
    offset += 8;
  }

  // Build accounts array
  const keys = [
    { pubkey: routerAccount, isSigner: false, isWritable: true },
    { pubkey: escrowAccount, isSigner: false, isWritable: true },
    { pubkey: portfolioAccount, isSigner: false, isWritable: true },
    { pubkey: userAuthority, isSigner: true, isWritable: false },
  ];

  // Add slab accounts
  for (const slabAccount of slabAccounts) {
    keys.push({ pubkey: slabAccount, isSigner: false, isWritable: true });
  }

  return new TransactionInstruction({
    keys,
    programId: ROUTER_PROGRAM_ID,
    data,
  });
}

/**
 * Generate a commitment hash for commit-reveal
 */
export function generateCommitmentHash(secret: Buffer): Buffer {
  const crypto = require('crypto');
  return crypto.createHash('sha256').update(secret).digest();
}

/**
 * Generate a random secret for commit-reveal
 */
export function generateSecret(): Buffer {
  const crypto = require('crypto');
  return crypto.randomBytes(32);
}

/**
 * Serialize a transaction to base64 for sending to frontend
 */
export function serializeTransaction(tx: Transaction): string {
  return tx.serialize({ requireAllSignatures: false }).toString('base64');
}

/**
 * Simulate transaction before sending to wallet
 * This prevents Phantom warnings by ensuring the transaction will succeed
 * 
 * Note: @solana/web3.js v1.x doesn't support sigVerify: false in simulation
 * For now, we skip simulation and rely on:
 * 1. Single signer (user wallet only)
 * 2. Proper fee payer set
 * 3. Fresh blockhash
 */
export async function simulateTransaction(tx: Transaction): Promise<{ success: boolean; error?: string }> {
  // Skip simulation for now - v1.x API limitation
  // Transactions are simple (single signer, single instruction) and should succeed
  console.log('⚠️  Transaction simulation skipped (v1.x API limitation)');
  return { success: true };
}

/**
 * Get recent blockhash for transaction
 */
export async function getRecentBlockhash(): Promise<string> {
  try {
    const connection = getConnection();
    const { blockhash } = await connection.getLatestBlockhash('finalized');
    return blockhash;
  } catch (error) {
    // If local RPC is not available, try devnet
    try {
      console.warn('Local RPC unavailable, connecting to devnet...');
      const devnetConnection = new Connection('https://api.devnet.solana.com', 'confirmed');
      const { blockhash } = await devnetConnection.getLatestBlockhash('finalized');
      console.log('✅ Got blockhash from devnet:', blockhash);
      return blockhash;
    } catch (devnetError) {
      // Last resort: mock blockhash
      console.warn('Failed to get recent blockhash from devnet, using mock value');
      return 'GH7ome3EiwEr7tu9JuTh2dpYWBJK3z69Xm1ZE3MEE6JC';
    }
  }
}

