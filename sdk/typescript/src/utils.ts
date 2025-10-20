/**
 * Utility functions
 */

import BN from 'bn.js';

/**
 * Convert price with decimals to protocol format
 * 
 * @param price - Price as number (e.g., 50000 for $50k)
 * @param decimals - Decimal places (e.g., 6 for USDC)
 * @returns BN in protocol format
 */
export function priceToProtocol(price: number, decimals: number = 6): BN {
  const multiplier = Math.pow(10, decimals);
  return new BN(Math.floor(price * multiplier));
}

/**
 * Convert protocol price to human-readable
 */
export function priceFromProtocol(price: BN, decimals: number = 6): number {
  const divisor = Math.pow(10, decimals);
  return price.toNumber() / divisor;
}

/**
 * Convert quantity to protocol format
 */
export function qtyToProtocol(qty: number, decimals: number = 6): BN {
  const multiplier = Math.pow(10, decimals);
  return new BN(Math.floor(qty * multiplier));
}

/**
 * Convert protocol quantity to human-readable
 */
export function qtyFromProtocol(qty: BN, decimals: number = 6): number {
  const divisor = Math.pow(10, decimals);
  return qty.toNumber() / divisor;
}

/**
 * Calculate VWAP from total notional and quantity
 */
export function calculateVWAP(notional: BN, qty: BN): BN {
  if (qty.isZero()) return new BN(0);
  return notional.div(qty);
}

/**
 * Calculate PnL for a position
 */
export function calculatePnL(
  qty: BN,  // Signed
  entryPrice: BN,
  currentPrice: BN
): BN {
  if (qty.isZero()) return new BN(0);
  
  const priceDiff = currentPrice.sub(entryPrice);
  const isLong = !qty.isNeg();
  
  if (isLong) {
    return qty.mul(priceDiff);
  } else {
    return qty.abs().mul(entryPrice.sub(currentPrice));
  }
}

/**
 * Format lamports to SOL
 */
export function lamportsToSOL(lamports: number | BN): number {
  const bn = typeof lamports === 'number' ? new BN(lamports) : lamports;
  return bn.toNumber() / 1e9;
}

/**
 * Format SOL to lamports
 */
export function solToLamports(sol: number): BN {
  return new BN(Math.floor(sol * 1e9));
}

/**
 * Sleep for ms
 */
export function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Retry with exponential backoff
 */
export async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries: number = 3,
  initialDelayMs: number = 1000
): Promise<T> {
  let lastError: Error;
  
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error as Error;
      if (i < maxRetries - 1) {
        const delay = initialDelayMs * Math.pow(2, i);
        await sleep(delay);
      }
    }
  }
  
  throw lastError!;
}

/**
 * Format basis points to percentage
 */
export function bpsToPercent(bps: number): string {
  return (bps / 100).toFixed(2) + '%';
}

/**
 * Parse percentage to basis points
 */
export function percentToBps(percent: number): number {
  return Math.floor(percent * 100);
}

