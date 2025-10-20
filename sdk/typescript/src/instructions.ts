/**
 * Instruction builders for PERColator protocol
 */

import {
  PublicKey,
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import BN from 'bn.js';
import { Side, SlabInstruction, RouterInstruction, SlabConfig } from './types';

/**
 * Create Reserve instruction
 * 
 * Locks liquidity from the orderbook for future execution
 */
export function createReserveInstruction(
  slabProgramId: PublicKey,
  slabState: PublicKey,
  user: PublicKey,
  accountIdx: number,
  instrumentIdx: number,
  side: Side,
  qty: BN,
  limitPx: BN,
  ttlMs: number,
  commitmentHash: Buffer,
  routeId: BN
): TransactionInstruction {
  // Build instruction data
  const data = Buffer.alloc(72);
  let offset = 0;
  
  // Discriminator
  data.writeUInt8(SlabInstruction.Reserve, offset);
  offset += 1;
  
  // Parameters
  data.writeUInt32LE(accountIdx, offset);
  offset += 4;
  data.writeUInt16LE(instrumentIdx, offset);
  offset += 2;
  data.writeUInt8(side, offset);
  offset += 1;
  writeBN(data, qty, offset, 8);
  offset += 8;
  writeBN(data, limitPx, offset, 8);
  offset += 8;
  data.writeBigUInt64LE(BigInt(ttlMs), offset);
  offset += 8;
  commitmentHash.copy(data, offset);
  offset += 32;
  writeBN(data, routeId, offset, 8);

  return new TransactionInstruction({
    keys: [
      { pubkey: slabState, isSigner: false, isWritable: true },
      { pubkey: user, isSigner: true, isWritable: false },
    ],
    programId: slabProgramId,
    data,
  });
}

/**
 * Create Commit instruction
 * 
 * Executes trades at reserved prices
 */
export function createCommitInstruction(
  slabProgramId: PublicKey,
  slabState: PublicKey,
  user: PublicKey,
  holdId: BN,
  currentTs: number
): TransactionInstruction {
  const data = Buffer.alloc(17);
  let offset = 0;
  
  data.writeUInt8(SlabInstruction.Commit, offset);
  offset += 1;
  writeBN(data, holdId, offset, 8);
  offset += 8;
  data.writeBigUInt64LE(BigInt(currentTs), offset);

  return new TransactionInstruction({
    keys: [
      { pubkey: slabState, isSigner: false, isWritable: true },
      { pubkey: user, isSigner: true, isWritable: false },
    ],
    programId: slabProgramId,
    data,
  });
}

/**
 * Create Cancel instruction
 * 
 * Releases a reservation
 */
export function createCancelInstruction(
  slabProgramId: PublicKey,
  slabState: PublicKey,
  user: PublicKey,
  holdId: BN
): TransactionInstruction {
  const data = Buffer.alloc(9);
  let offset = 0;
  
  data.writeUInt8(SlabInstruction.Cancel, offset);
  offset += 1;
  writeBN(data, holdId, offset, 8);

  return new TransactionInstruction({
    keys: [
      { pubkey: slabState, isSigner: false, isWritable: true },
      { pubkey: user, isSigner: true, isWritable: false },
    ],
    programId: slabProgramId,
    data,
  });
}

/**
 * Create BatchOpen instruction
 * 
 * Opens new batch/epoch and promotes pending orders
 */
export function createBatchOpenInstruction(
  slabProgramId: PublicKey,
  slabState: PublicKey,
  authority: PublicKey,
  instrumentIdx: number,
  currentTs: number
): TransactionInstruction {
  const data = Buffer.alloc(11);
  let offset = 0;
  
  data.writeUInt8(SlabInstruction.BatchOpen, offset);
  offset += 1;
  data.writeUInt16LE(instrumentIdx, offset);
  offset += 2;
  data.writeBigUInt64LE(BigInt(currentTs), offset);

  return new TransactionInstruction({
    keys: [
      { pubkey: slabState, isSigner: false, isWritable: true },
      { pubkey: authority, isSigner: true, isWritable: false },
    ],
    programId: slabProgramId,
    data,
  });
}

/**
 * Create Initialize Slab instruction
 */
export function createInitializeSlabInstruction(
  slabProgramId: PublicKey,
  slabState: PublicKey,
  payer: PublicKey,
  config: SlabConfig
): TransactionInstruction {
  const data = Buffer.alloc(115);
  let offset = 0;
  
  data.writeUInt8(SlabInstruction.Initialize, offset);
  offset += 1;
  config.authority.toBuffer().copy(data, offset);
  offset += 32;
  config.oracle.toBuffer().copy(data, offset);
  offset += 32;
  config.router.toBuffer().copy(data, offset);
  offset += 32;
  data.writeUInt16LE(config.imr, offset);
  offset += 2;
  data.writeUInt16LE(config.mmr, offset);
  offset += 2;
  data.writeBigInt64LE(BigInt(config.makerFee), offset);
  offset += 8;
  data.writeBigInt64LE(BigInt(config.takerFee), offset);
  offset += 8;
  data.writeBigUInt64LE(BigInt(config.batchMs), offset);
  offset += 8;
  data.writeUInt16LE(config.freezeLevels, offset);

  return new TransactionInstruction({
    keys: [
      { pubkey: slabState, isSigner: false, isWritable: true },
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ],
    programId: slabProgramId,
    data,
  });
}

/**
 * Create AddInstrument instruction
 */
export function createAddInstrumentInstruction(
  slabProgramId: PublicKey,
  slabState: PublicKey,
  authority: PublicKey,
  symbol: string,
  contractSize: BN,
  tick: BN,
  lot: BN,
  indexPrice: BN
): TransactionInstruction {
  const data = Buffer.alloc(41);
  let offset = 0;
  
  data.writeUInt8(SlabInstruction.AddInstrument, offset);
  offset += 1;
  Buffer.from(symbol.padEnd(8, '\0').slice(0, 8)).copy(data, offset);
  offset += 8;
  writeBN(data, contractSize, offset, 8);
  offset += 8;
  writeBN(data, tick, offset, 8);
  offset += 8;
  writeBN(data, lot, offset, 8);
  offset += 8;
  writeBN(data, indexPrice, offset, 8);

  return new TransactionInstruction({
    keys: [
      { pubkey: slabState, isSigner: false, isWritable: true },
      { pubkey: authority, isSigner: true, isWritable: false },
    ],
    programId: slabProgramId,
    data,
  });
}

/**
 * Create Router Deposit instruction
 */
export function createDepositInstruction(
  routerProgramId: PublicKey,
  vault: PublicKey,
  userTokenAccount: PublicKey,
  portfolio: PublicKey,
  user: PublicKey,
  mint: PublicKey,
  amount: BN
): TransactionInstruction {
  const data = Buffer.alloc(49);
  let offset = 0;
  
  data.writeUInt8(RouterInstruction.Deposit, offset);
  offset += 1;
  mint.toBuffer().copy(data, offset);
  offset += 32;
  writeBN(data, amount, offset, 16);

  return new TransactionInstruction({
    keys: [
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: portfolio, isSigner: false, isWritable: true },
      { pubkey: user, isSigner: true, isWritable: false },
    ],
    programId: routerProgramId,
    data,
  });
}

/**
 * Create Router Withdraw instruction
 */
export function createWithdrawInstruction(
  routerProgramId: PublicKey,
  vault: PublicKey,
  userTokenAccount: PublicKey,
  portfolio: PublicKey,
  user: PublicKey,
  mint: PublicKey,
  amount: BN
): TransactionInstruction {
  const data = Buffer.alloc(49);
  let offset = 0;
  
  data.writeUInt8(RouterInstruction.Withdraw, offset);
  offset += 1;
  mint.toBuffer().copy(data, offset);
  offset += 32;
  writeBN(data, amount, offset, 16);

  return new TransactionInstruction({
    keys: [
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: portfolio, isSigner: false, isWritable: true },
      { pubkey: user, isSigner: true, isWritable: false },
    ],
    programId: routerProgramId,
    data,
  });
}

/**
 * Helper: Write BN to buffer
 */
function writeBN(buffer: Buffer, value: BN, offset: number, length: number): void {
  const bn = new BN(value);
  const bytes = bn.toArrayLike(Buffer, 'le', length);
  bytes.copy(buffer, offset);
}

/**
 * Helper: Read BN from buffer
 */
export function readBN(buffer: Buffer, offset: number, length: number): BN {
  return new BN(buffer.slice(offset, offset + length), 'le');
}

