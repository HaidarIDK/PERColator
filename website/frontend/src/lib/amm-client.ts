/**
 * Direct AMM Client - v0 Option A
 * Calls AMM program directly without Router
 * Simple swap interface for immediate functionality
 */

import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  Keypair,
} from '@solana/web3.js';

// AMM Program ID (deployed on devnet)
export const AMM_PROGRAM_ID = new PublicKey('AMMjkEeFdasQ8fs9a9HQyJdciPHtDHVEat8yxiXrTP6p');

// Pool addresses (from initialization)
export const POOLS = {
  SOL: new PublicKey('FvxA93qPzDEGVdkP4PDE1gpXpt9R3u8gBgh9FTovsHJm'),
  ETH: new PublicKey('GPLmAVdfwE6zD1acgd5mZ7Myfq57oCQeJ9KGnco58YdQ'),
  BTC: new PublicKey('6vpuVH6SZX5a9PZSgMNBoZQAsTZtNJrEtd87RQCUHsPC'),
} as const;

export type PoolSymbol = keyof typeof POOLS;

/**
 * Side enum
 */
export enum Side {
  Buy = 0,
  Sell = 1,
}

/**
 * AMM Pool State (read from on-chain)
 */
export interface AMMPoolState {
  xReserve: bigint;  // Base token reserve (scaled by 1e6)
  yReserve: bigint;  // USDC reserve (scaled by 1e6)
  feeBps: number;    // Fee in basis points
  seqno: number;     // Sequence number
}

/**
 * Swap result
 */
export interface SwapResult {
  vwapPrice: number;      // VWAP price (1e6 scale)
  notional: number;       // Notional value
  fee: number;            // Fee charged
  newXReserve: bigint;    // New X reserve after swap
  newYReserve: bigint;    // New Y reserve after swap
}

/**
 * Read AMM pool state from on-chain account
 */
export async function readPoolState(
  connection: Connection,
  poolAddress: PublicKey
): Promise<AMMPoolState> {
  const accountInfo = await connection.getAccountInfo(poolAddress);
  
  if (!accountInfo) {
    throw new Error(`Pool account ${poolAddress.toBase58()} not found`);
  }

  const data = accountInfo.data;

  // Parse AMM state layout:
  // [0..200] SlabHeader
  // [200..336] QuoteCache
  // [336..] AmmPool
  
  // Read seqno from SlabHeader (offset 12, u32)
  const seqno = data.readUInt32LE(12);
  
  // Read AmmPool data (starts at offset 336)
  const poolOffset = 336;
  
  // x_reserve (i64 at offset 0 from AmmPool start)
  const xReserve = data.readBigInt64LE(poolOffset);
  
  // y_reserve (i64 at offset 8 from AmmPool start)
  const yReserve = data.readBigInt64LE(poolOffset + 8);
  
  // fee_bps (i64 at offset 16 from AmmPool start)
  const feeBps = Number(data.readBigInt64LE(poolOffset + 16));

  return {
    xReserve,
    yReserve,
    feeBps,
    seqno,
  };
}

/**
 * Calculate swap output using constant product formula (x·y=k)
 * For Buy: User gives Y (USDC), gets X (token)
 * For Sell: User gives X (token), gets Y (USDC)
 */
export function calculateSwapOutput(
  side: Side,
  amountIn: number,      // Amount user is sending (1e6 scale)
  xReserve: bigint,      // Current X reserve
  yReserve: bigint,      // Current Y reserve
  feeBps: number         // Fee in basis points
): SwapResult {
  const amountInBig = BigInt(Math.floor(amountIn * 1_000_000));
  
  // Apply fee (0.3% = 30 bps)
  const fee = (amountInBig * BigInt(feeBps)) / BigInt(10_000);
  const amountInAfterFee = amountInBig - fee;
  
  let newX: bigint;
  let newY: bigint;
  let amountOut: bigint;
  
  if (side === Side.Buy) {
    // User buys X with Y
    // x_out = (x * y_in) / (y + y_in)
    newY = yReserve + amountInAfterFee;
    newX = (xReserve * yReserve) / newY;
    amountOut = xReserve - newX;
  } else {
    // User sells X for Y
    // y_out = (y * x_in) / (x + x_in)
    newX = xReserve + amountInAfterFee;
    newY = (xReserve * yReserve) / newX;
    amountOut = yReserve - newY;
  }
  
  // Calculate VWAP: total value / qty
  const vwapPrice = side === Side.Buy
    ? Number(amountInBig * BigInt(1_000_000) / amountOut)
    : Number(amountOut * BigInt(1_000_000) / amountInBig);
  
  const notional = Number(amountInBig) / 1_000_000;
  
  return {
    vwapPrice,
    notional,
    fee: Number(fee) / 1_000_000,
    newXReserve: newX,
    newYReserve: newY,
  };
}

/**
 * Build CommitFill instruction for direct AMM call
 * 
 * NOTE: This is a simplified v0 version that calls AMM directly.
 * In v1 with Router, this would go through Router program.
 */
export function buildCommitFillInstruction(
  poolAddress: PublicKey,
  userAuthority: PublicKey,
  side: Side,
  qty: number,          // Quantity to trade (human-readable, e.g., 1.5)
  limitPrice: number,   // Limit price (human-readable, e.g., 4130.50)
  expectedSeqno: number // Current seqno for TOCTOU protection
): TransactionInstruction {
  // Convert to 1e6 scale
  const qtyScaled = BigInt(Math.floor(qty * 1_000_000));
  const limitPxScaled = BigInt(Math.floor(limitPrice * 1_000_000));
  
  // Build instruction data
  // Layout: [discriminator (1)] + [expected_seqno (4)] + [side (1)] + [qty (8)] + [limit_px (8)]
  const data = Buffer.alloc(22);
  let offset = 0;
  
  // Discriminator for CommitFill = 1
  data.writeUInt8(1, offset);
  offset += 1;
  
  // Expected seqno (u32) - TOCTOU protection
  data.writeUInt32LE(expectedSeqno, offset);
  offset += 4;
  
  // Side (u8): 0 = Buy, 1 = Sell
  data.writeUInt8(side, offset);
  offset += 1;
  
  // Qty (i64)
  data.writeBigInt64LE(qtyScaled, offset);
  offset += 8;
  
  // Limit Price (i64)
  data.writeBigInt64LE(limitPxScaled, offset);
  
  // Generate receipt PDA (where fill receipt will be written)
  // For v0 simplified: use a temporary keypair
  // In production, this would be a PDA derived from user + seqno
  const receiptKeypair = Keypair.generate();
  
  return new TransactionInstruction({
    keys: [
      { pubkey: poolAddress, isSigner: false, isWritable: true },
      { pubkey: receiptKeypair.publicKey, isSigner: false, isWritable: true },
      { pubkey: userAuthority, isSigner: true, isWritable: false },
    ],
    programId: AMM_PROGRAM_ID,
    data,
  });
}

/**
 * Execute a swap on the AMM
 * 
 * @param connection Solana connection
 * @param poolAddress AMM pool address
 * @param userKeypair User's keypair (will sign transaction)
 * @param side Buy or Sell
 * @param qty Quantity to trade (human-readable)
 * @param limitPrice Maximum price willing to pay (human-readable)
 * @returns Transaction signature
 */
export async function executeSwap(
  connection: Connection,
  poolAddress: PublicKey,
  userKeypair: Keypair,
  side: Side,
  qty: number,
  limitPrice: number
): Promise<string> {
  // 1. Read current pool state
  const poolState = await readPoolState(connection, poolAddress);
  
  console.log('Current pool state:', {
    xReserve: poolState.xReserve.toString(),
    yReserve: poolState.yReserve.toString(),
    feeBps: poolState.feeBps,
    seqno: poolState.seqno,
  });
  
  // 2. Calculate expected output
  const swapResult = calculateSwapOutput(
    side,
    side === Side.Buy ? qty * limitPrice : qty, // For buy, qty is in Y; for sell, qty is in X
    poolState.xReserve,
    poolState.yReserve,
    poolState.feeBps
  );
  
  console.log('Expected swap result:', swapResult);
  
  // 3. Build CommitFill instruction
  const instruction = buildCommitFillInstruction(
    poolAddress,
    userKeypair.publicKey,
    side,
    qty,
    limitPrice,
    poolState.seqno
  );
  
  // 4. Create transaction
  const transaction = new Transaction().add(instruction);
  
  // 5. Get recent blockhash
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = userKeypair.publicKey;
  
  // 6. Sign transaction
  transaction.sign(userKeypair);
  
  // 7. Send transaction
  const signature = await connection.sendRawTransaction(transaction.serialize(), {
    skipPreflight: false,
    preflightCommitment: 'confirmed',
  });
  
  console.log('Swap transaction sent:', signature);
  
  // 8. Confirm transaction
  await connection.confirmTransaction({
    signature,
    blockhash,
    lastValidBlockHeight,
  });
  
  console.log('Swap transaction confirmed!');
  
  return signature;
}

/**
 * Helper: Get pool address by symbol
 */
export function getPoolAddress(symbol: PoolSymbol): PublicKey {
  return POOLS[symbol];
}

/**
 * Helper: Simulate swap without executing
 */
export async function simulateSwap(
  connection: Connection,
  poolSymbol: PoolSymbol,
  side: Side,
  qty: number
): Promise<SwapResult> {
  const poolAddress = getPoolAddress(poolSymbol);
  const poolState = await readPoolState(connection, poolAddress);
  
  // For simulation, use current market price as limit
  const currentPrice = Number(poolState.yReserve) / Number(poolState.xReserve);
  
  return calculateSwapOutput(
    side,
    side === Side.Buy ? qty * currentPrice : qty,
    poolState.xReserve,
    poolState.yReserve,
    poolState.feeBps
  );
}

