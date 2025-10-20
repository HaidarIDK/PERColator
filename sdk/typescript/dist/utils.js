"use strict";
/**
 * Utility functions
 */
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.priceToProtocol = priceToProtocol;
exports.priceFromProtocol = priceFromProtocol;
exports.qtyToProtocol = qtyToProtocol;
exports.qtyFromProtocol = qtyFromProtocol;
exports.calculateVWAP = calculateVWAP;
exports.calculatePnL = calculatePnL;
exports.lamportsToSOL = lamportsToSOL;
exports.solToLamports = solToLamports;
exports.sleep = sleep;
exports.retryWithBackoff = retryWithBackoff;
exports.bpsToPercent = bpsToPercent;
exports.percentToBps = percentToBps;
const bn_js_1 = __importDefault(require("bn.js"));
/**
 * Convert price with decimals to protocol format
 *
 * @param price - Price as number (e.g., 50000 for $50k)
 * @param decimals - Decimal places (e.g., 6 for USDC)
 * @returns BN in protocol format
 */
function priceToProtocol(price, decimals = 6) {
    const multiplier = Math.pow(10, decimals);
    return new bn_js_1.default(Math.floor(price * multiplier));
}
/**
 * Convert protocol price to human-readable
 */
function priceFromProtocol(price, decimals = 6) {
    const divisor = Math.pow(10, decimals);
    return price.toNumber() / divisor;
}
/**
 * Convert quantity to protocol format
 */
function qtyToProtocol(qty, decimals = 6) {
    const multiplier = Math.pow(10, decimals);
    return new bn_js_1.default(Math.floor(qty * multiplier));
}
/**
 * Convert protocol quantity to human-readable
 */
function qtyFromProtocol(qty, decimals = 6) {
    const divisor = Math.pow(10, decimals);
    return qty.toNumber() / divisor;
}
/**
 * Calculate VWAP from total notional and quantity
 */
function calculateVWAP(notional, qty) {
    if (qty.isZero())
        return new bn_js_1.default(0);
    return notional.div(qty);
}
/**
 * Calculate PnL for a position
 */
function calculatePnL(qty, // Signed
entryPrice, currentPrice) {
    if (qty.isZero())
        return new bn_js_1.default(0);
    const priceDiff = currentPrice.sub(entryPrice);
    const isLong = !qty.isNeg();
    if (isLong) {
        return qty.mul(priceDiff);
    }
    else {
        return qty.abs().mul(entryPrice.sub(currentPrice));
    }
}
/**
 * Format lamports to SOL
 */
function lamportsToSOL(lamports) {
    const bn = typeof lamports === 'number' ? new bn_js_1.default(lamports) : lamports;
    return bn.toNumber() / 1e9;
}
/**
 * Format SOL to lamports
 */
function solToLamports(sol) {
    return new bn_js_1.default(Math.floor(sol * 1e9));
}
/**
 * Sleep for ms
 */
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}
/**
 * Retry with exponential backoff
 */
async function retryWithBackoff(fn, maxRetries = 3, initialDelayMs = 1000) {
    let lastError;
    for (let i = 0; i < maxRetries; i++) {
        try {
            return await fn();
        }
        catch (error) {
            lastError = error;
            if (i < maxRetries - 1) {
                const delay = initialDelayMs * Math.pow(2, i);
                await sleep(delay);
            }
        }
    }
    throw lastError;
}
/**
 * Format basis points to percentage
 */
function bpsToPercent(bps) {
    return (bps / 100).toFixed(2) + '%';
}
/**
 * Parse percentage to basis points
 */
function percentToBps(percent) {
    return Math.floor(percent * 100);
}
