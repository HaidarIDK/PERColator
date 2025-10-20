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
export declare function priceToProtocol(price: number, decimals?: number): BN;
/**
 * Convert protocol price to human-readable
 */
export declare function priceFromProtocol(price: BN, decimals?: number): number;
/**
 * Convert quantity to protocol format
 */
export declare function qtyToProtocol(qty: number, decimals?: number): BN;
/**
 * Convert protocol quantity to human-readable
 */
export declare function qtyFromProtocol(qty: BN, decimals?: number): number;
/**
 * Calculate VWAP from total notional and quantity
 */
export declare function calculateVWAP(notional: BN, qty: BN): BN;
/**
 * Calculate PnL for a position
 */
export declare function calculatePnL(qty: BN, // Signed
entryPrice: BN, currentPrice: BN): BN;
/**
 * Format lamports to SOL
 */
export declare function lamportsToSOL(lamports: number | BN): number;
/**
 * Format SOL to lamports
 */
export declare function solToLamports(sol: number): BN;
/**
 * Sleep for ms
 */
export declare function sleep(ms: number): Promise<void>;
/**
 * Retry with exponential backoff
 */
export declare function retryWithBackoff<T>(fn: () => Promise<T>, maxRetries?: number, initialDelayMs?: number): Promise<T>;
/**
 * Format basis points to percentage
 */
export declare function bpsToPercent(bps: number): string;
/**
 * Parse percentage to basis points
 */
export declare function percentToBps(percent: number): number;
