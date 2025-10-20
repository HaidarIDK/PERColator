"use strict";
/**
 * State account decoders
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.decodePortfolio = decodePortfolio;
exports.decodeOrderbook = decodeOrderbook;
exports.decodeVault = decodeVault;
exports.decodeEscrow = decodeEscrow;
const web3_js_1 = require("@solana/web3.js");
const instructions_1 = require("./instructions");
/**
 * Decode Portfolio account
 */
function decodePortfolio(data) {
    let offset = 0;
    const routerId = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const user = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    // Read equity (i128)
    const equity = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    const im = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    const mm = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    const freeCollateral = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    const lastMarkTs = (0, instructions_1.readBN)(data, offset, 8);
    offset += 8;
    const exposureCount = data.readUInt16LE(offset);
    offset += 2;
    // Skip bump + padding
    offset += 6;
    // Read exposures
    const exposures = [];
    for (let i = 0; i < exposureCount; i++) {
        const slabIdx = data.readUInt16LE(offset);
        offset += 2;
        const instrumentIdx = data.readUInt16LE(offset);
        offset += 2;
        const qty = (0, instructions_1.readBN)(data, offset, 8);
        offset += 8;
        exposures.push({ slabIdx, instrumentIdx, qty });
    }
    return {
        routerId,
        user,
        equity,
        im,
        mm,
        freeCollateral,
        lastMarkTs,
        exposures,
    };
}
/**
 * Decode Orderbook from slab state
 *
 * This is a simplified decoder - full implementation would walk linked lists
 */
function decodeOrderbook(data, instrumentIdx) {
    // For now, return mock orderbook
    // Full implementation would:
    // 1. Find instrument in slab.instruments array
    // 2. Walk bids_head linked list
    // 3. Walk asks_head linked list
    // 4. Aggregate by price level
    return {
        instrumentIdx,
        bids: [],
        asks: [],
        lastUpdate: Date.now(),
    };
}
/**
 * Decode Vault account
 */
function decodeVault(data) {
    let offset = 0;
    const routerId = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const mint = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const tokenAccount = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const balance = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    const totalPledged = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    return {
        routerId,
        mint,
        tokenAccount,
        balance,
        totalPledged,
        available: balance.sub(totalPledged),
    };
}
/**
 * Decode Escrow account
 */
function decodeEscrow(data) {
    let offset = 0;
    const routerId = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const slabId = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const user = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const mint = new web3_js_1.PublicKey(data.slice(offset, offset + 32));
    offset += 32;
    const balance = (0, instructions_1.readBN)(data, offset, 16);
    offset += 16;
    const nonce = (0, instructions_1.readBN)(data, offset, 8);
    offset += 8;
    const frozen = data.readUInt8(offset) === 1;
    return {
        routerId,
        slabId,
        user,
        mint,
        balance,
        nonce,
        frozen,
    };
}
