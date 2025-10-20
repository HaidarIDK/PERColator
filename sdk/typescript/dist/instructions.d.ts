/**
 * Instruction builders for PERColator protocol
 */
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import BN from 'bn.js';
import { Side, SlabConfig } from './types';
/**
 * Create Reserve instruction
 *
 * Locks liquidity from the orderbook for future execution
 */
export declare function createReserveInstruction(slabProgramId: PublicKey, slabState: PublicKey, user: PublicKey, accountIdx: number, instrumentIdx: number, side: Side, qty: BN, limitPx: BN, ttlMs: number, commitmentHash: Buffer, routeId: BN): TransactionInstruction;
/**
 * Create Commit instruction
 *
 * Executes trades at reserved prices
 */
export declare function createCommitInstruction(slabProgramId: PublicKey, slabState: PublicKey, user: PublicKey, holdId: BN, currentTs: number): TransactionInstruction;
/**
 * Create Cancel instruction
 *
 * Releases a reservation
 */
export declare function createCancelInstruction(slabProgramId: PublicKey, slabState: PublicKey, user: PublicKey, holdId: BN): TransactionInstruction;
/**
 * Create BatchOpen instruction
 *
 * Opens new batch/epoch and promotes pending orders
 */
export declare function createBatchOpenInstruction(slabProgramId: PublicKey, slabState: PublicKey, authority: PublicKey, instrumentIdx: number, currentTs: number): TransactionInstruction;
/**
 * Create Initialize Slab instruction
 */
export declare function createInitializeSlabInstruction(slabProgramId: PublicKey, slabState: PublicKey, payer: PublicKey, config: SlabConfig): TransactionInstruction;
/**
 * Create AddInstrument instruction
 */
export declare function createAddInstrumentInstruction(slabProgramId: PublicKey, slabState: PublicKey, authority: PublicKey, symbol: string, contractSize: BN, tick: BN, lot: BN, indexPrice: BN): TransactionInstruction;
/**
 * Create Router Deposit instruction
 */
export declare function createDepositInstruction(routerProgramId: PublicKey, vault: PublicKey, userTokenAccount: PublicKey, portfolio: PublicKey, user: PublicKey, mint: PublicKey, amount: BN): TransactionInstruction;
/**
 * Create Router Withdraw instruction
 */
export declare function createWithdrawInstruction(routerProgramId: PublicKey, vault: PublicKey, userTokenAccount: PublicKey, portfolio: PublicKey, user: PublicKey, mint: PublicKey, amount: BN): TransactionInstruction;
/**
 * Helper: Read BN from buffer
 */
export declare function readBN(buffer: Buffer, offset: number, length: number): BN;
