"use strict";
/**
 * Program Derived Address (PDA) helpers
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.deriveSlabPDA = deriveSlabPDA;
exports.deriveVaultPDA = deriveVaultPDA;
exports.deriveEscrowPDA = deriveEscrowPDA;
exports.derivePortfolioPDA = derivePortfolioPDA;
exports.deriveCapPDA = deriveCapPDA;
exports.deriveRegistryPDA = deriveRegistryPDA;
const web3_js_1 = require("@solana/web3.js");
/**
 * Derive Slab PDA
 * Seeds: ["slab", market_id]
 */
function deriveSlabPDA(marketId, programId) {
    const seeds = [
        Buffer.from('slab'),
        Buffer.from(marketId.padEnd(8, '\0').slice(0, 8)),
    ];
    return web3_js_1.PublicKey.findProgramAddressSync(seeds, programId);
}
/**
 * Derive Vault PDA
 * Seeds: ["vault", router_id, mint]
 */
function deriveVaultPDA(routerId, mint) {
    const seeds = [
        Buffer.from('vault'),
        routerId.toBuffer(),
        mint.toBuffer(),
    ];
    return web3_js_1.PublicKey.findProgramAddressSync(seeds, routerId);
}
/**
 * Derive Escrow PDA
 * Seeds: ["escrow", router_id, slab_id, user_id, mint]
 */
function deriveEscrowPDA(routerId, slabId, user, mint) {
    const seeds = [
        Buffer.from('escrow'),
        routerId.toBuffer(),
        slabId.toBuffer(),
        user.toBuffer(),
        mint.toBuffer(),
    ];
    return web3_js_1.PublicKey.findProgramAddressSync(seeds, routerId);
}
/**
 * Derive Portfolio PDA
 * Seeds: ["portfolio", router_id, user]
 */
function derivePortfolioPDA(routerId, user) {
    const seeds = [
        Buffer.from('portfolio'),
        routerId.toBuffer(),
        user.toBuffer(),
    ];
    return web3_js_1.PublicKey.findProgramAddressSync(seeds, routerId);
}
/**
 * Derive Cap (Capability) PDA
 * Seeds: ["cap", router_id, route_id]
 */
function deriveCapPDA(routerId, routeId) {
    const routeIdBuffer = Buffer.alloc(8);
    routeIdBuffer.writeBigUInt64LE(routeId);
    const seeds = [
        Buffer.from('cap'),
        routerId.toBuffer(),
        routeIdBuffer,
    ];
    return web3_js_1.PublicKey.findProgramAddressSync(seeds, routerId);
}
/**
 * Derive Registry PDA
 * Seeds: ["registry", router_id]
 */
function deriveRegistryPDA(routerId) {
    const seeds = [
        Buffer.from('registry'),
        routerId.toBuffer(),
    ];
    return web3_js_1.PublicKey.findProgramAddressSync(seeds, routerId);
}
