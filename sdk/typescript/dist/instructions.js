"use strict";
/**
 * Instruction builders for PERColator protocol
 */
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createReserveInstruction = createReserveInstruction;
exports.createCommitInstruction = createCommitInstruction;
exports.createCancelInstruction = createCancelInstruction;
exports.createBatchOpenInstruction = createBatchOpenInstruction;
exports.createInitializeSlabInstruction = createInitializeSlabInstruction;
exports.createAddInstrumentInstruction = createAddInstrumentInstruction;
exports.createDepositInstruction = createDepositInstruction;
exports.createWithdrawInstruction = createWithdrawInstruction;
exports.readBN = readBN;
const web3_js_1 = require("@solana/web3.js");
const bn_js_1 = __importDefault(require("bn.js"));
const types_1 = require("./types");
/**
 * Create Reserve instruction
 *
 * Locks liquidity from the orderbook for future execution
 */
function createReserveInstruction(slabProgramId, slabState, user, accountIdx, instrumentIdx, side, qty, limitPx, ttlMs, commitmentHash, routeId) {
    // Build instruction data
    const data = Buffer.alloc(72);
    let offset = 0;
    // Discriminator
    data.writeUInt8(types_1.SlabInstruction.Reserve, offset);
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
    return new web3_js_1.TransactionInstruction({
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
function createCommitInstruction(slabProgramId, slabState, user, holdId, currentTs) {
    const data = Buffer.alloc(17);
    let offset = 0;
    data.writeUInt8(types_1.SlabInstruction.Commit, offset);
    offset += 1;
    writeBN(data, holdId, offset, 8);
    offset += 8;
    data.writeBigUInt64LE(BigInt(currentTs), offset);
    return new web3_js_1.TransactionInstruction({
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
function createCancelInstruction(slabProgramId, slabState, user, holdId) {
    const data = Buffer.alloc(9);
    let offset = 0;
    data.writeUInt8(types_1.SlabInstruction.Cancel, offset);
    offset += 1;
    writeBN(data, holdId, offset, 8);
    return new web3_js_1.TransactionInstruction({
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
function createBatchOpenInstruction(slabProgramId, slabState, authority, instrumentIdx, currentTs) {
    const data = Buffer.alloc(11);
    let offset = 0;
    data.writeUInt8(types_1.SlabInstruction.BatchOpen, offset);
    offset += 1;
    data.writeUInt16LE(instrumentIdx, offset);
    offset += 2;
    data.writeBigUInt64LE(BigInt(currentTs), offset);
    return new web3_js_1.TransactionInstruction({
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
function createInitializeSlabInstruction(slabProgramId, slabState, payer, config) {
    const data = Buffer.alloc(115);
    let offset = 0;
    data.writeUInt8(types_1.SlabInstruction.Initialize, offset);
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
    return new web3_js_1.TransactionInstruction({
        keys: [
            { pubkey: slabState, isSigner: false, isWritable: true },
            { pubkey: payer, isSigner: true, isWritable: true },
            { pubkey: web3_js_1.SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: web3_js_1.SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        ],
        programId: slabProgramId,
        data,
    });
}
/**
 * Create AddInstrument instruction
 */
function createAddInstrumentInstruction(slabProgramId, slabState, authority, symbol, contractSize, tick, lot, indexPrice) {
    const data = Buffer.alloc(41);
    let offset = 0;
    data.writeUInt8(types_1.SlabInstruction.AddInstrument, offset);
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
    return new web3_js_1.TransactionInstruction({
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
function createDepositInstruction(routerProgramId, vault, userTokenAccount, portfolio, user, mint, amount) {
    const data = Buffer.alloc(49);
    let offset = 0;
    data.writeUInt8(types_1.RouterInstruction.Deposit, offset);
    offset += 1;
    mint.toBuffer().copy(data, offset);
    offset += 32;
    writeBN(data, amount, offset, 16);
    return new web3_js_1.TransactionInstruction({
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
function createWithdrawInstruction(routerProgramId, vault, userTokenAccount, portfolio, user, mint, amount) {
    const data = Buffer.alloc(49);
    let offset = 0;
    data.writeUInt8(types_1.RouterInstruction.Withdraw, offset);
    offset += 1;
    mint.toBuffer().copy(data, offset);
    offset += 32;
    writeBN(data, amount, offset, 16);
    return new web3_js_1.TransactionInstruction({
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
function writeBN(buffer, value, offset, length) {
    const bn = new bn_js_1.default(value);
    const bytes = bn.toArrayLike(Buffer, 'le', length);
    bytes.copy(buffer, offset);
}
/**
 * Helper: Read BN from buffer
 */
function readBN(buffer, offset, length) {
    return new bn_js_1.default(buffer.slice(offset, offset + length), 'le');
}
