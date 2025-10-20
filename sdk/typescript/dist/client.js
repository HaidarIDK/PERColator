"use strict";
/**
 * PERColator Client - Main SDK interface
 */
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PercolatorClient = void 0;
const web3_js_1 = require("@solana/web3.js");
const bn_js_1 = __importDefault(require("bn.js"));
const instructions_1 = require("./instructions");
const pda_1 = require("./pda");
const state_1 = require("./state");
/**
 * Main PERColator client
 */
class PercolatorClient {
    constructor(connection, routerProgramId, slabProgramId, payer) {
        this.connection = connection;
        this.routerProgramId = routerProgramId;
        this.slabProgramId = slabProgramId;
        this.payer = payer;
    }
    /**
     * Reserve liquidity from a slab
     */
    async reserve(slabState, user, accountIdx, instrumentIdx, side, qty, limitPx, ttlMs = 60000, routeId) {
        // Generate commitment hash (zero for now)
        const commitmentHash = Buffer.alloc(32);
        // Generate route ID if not provided
        const rid = routeId || new bn_js_1.default(Date.now());
        const ix = (0, instructions_1.createReserveInstruction)(this.slabProgramId, slabState, user.publicKey, accountIdx, instrumentIdx, side, qty, limitPx, ttlMs, commitmentHash, rid);
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [user]);
        // TODO: Parse return data from transaction logs
        return {
            holdId: new bn_js_1.default(0),
            vwapPx: limitPx,
            worstPx: limitPx,
            maxCharge: new bn_js_1.default(0),
            filledQty: qty,
            expiryMs: new bn_js_1.default(Date.now() + ttlMs),
        };
    }
    /**
     * Commit a reservation
     */
    async commit(slabState, user, holdId) {
        const ix = (0, instructions_1.createCommitInstruction)(this.slabProgramId, slabState, user.publicKey, holdId, Date.now());
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [user]);
        // TODO: Parse return data
        return {
            filledQty: new bn_js_1.default(0),
            avgPrice: new bn_js_1.default(0),
            totalFee: new bn_js_1.default(0),
            totalDebit: new bn_js_1.default(0),
            fillsCount: 0,
        };
    }
    /**
     * Cancel a reservation
     */
    async cancel(slabState, user, holdId) {
        const ix = (0, instructions_1.createCancelInstruction)(this.slabProgramId, slabState, user.publicKey, holdId);
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [user]);
    }
    /**
     * Open new batch epoch
     */
    async openBatch(slabState, authority, instrumentIdx) {
        const ix = (0, instructions_1.createBatchOpenInstruction)(this.slabProgramId, slabState, authority.publicKey, instrumentIdx, Date.now());
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [authority]);
    }
    /**
     * Initialize a new slab
     */
    async initializeSlab(marketId, config, payer) {
        const [slabPDA] = (0, pda_1.deriveSlabPDA)(marketId, this.slabProgramId);
        // Create account for slab (7MB)
        const SLAB_SIZE = 7156336;
        const lamports = await this.connection.getMinimumBalanceForRentExemption(SLAB_SIZE);
        const createAccountIx = web3_js_1.SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: slabPDA,
            lamports,
            space: SLAB_SIZE,
            programId: this.slabProgramId,
        });
        const initIx = (0, instructions_1.createInitializeSlabInstruction)(this.slabProgramId, slabPDA, payer.publicKey, config);
        const tx = new web3_js_1.Transaction().add(createAccountIx, initIx);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [payer]);
        return slabPDA;
    }
    /**
     * Add instrument to slab
     */
    async addInstrument(slabState, authority, symbol, contractSize, tick, lot, indexPrice) {
        const ix = (0, instructions_1.createAddInstrumentInstruction)(this.slabProgramId, slabState, authority.publicKey, symbol, contractSize, tick, lot, indexPrice);
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [authority]);
    }
    /**
     * Deposit collateral to router vault
     */
    async deposit(mint, amount, user, userTokenAccount) {
        const [vault] = (0, pda_1.deriveVaultPDA)(this.routerProgramId, mint);
        const [portfolio] = (0, pda_1.derivePortfolioPDA)(this.routerProgramId, user.publicKey);
        const ix = (0, instructions_1.createDepositInstruction)(this.routerProgramId, vault, userTokenAccount, portfolio, user.publicKey, mint, amount);
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [user]);
    }
    /**
     * Withdraw collateral from router vault
     */
    async withdraw(mint, amount, user, userTokenAccount) {
        const [vault] = (0, pda_1.deriveVaultPDA)(this.routerProgramId, mint);
        const [portfolio] = (0, pda_1.derivePortfolioPDA)(this.routerProgramId, user.publicKey);
        const ix = (0, instructions_1.createWithdrawInstruction)(this.routerProgramId, vault, userTokenAccount, portfolio, user.publicKey, mint, amount);
        const tx = new web3_js_1.Transaction().add(ix);
        await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [user]);
    }
    /**
     * Get user's portfolio
     */
    async getPortfolio(user) {
        const [portfolioPDA] = (0, pda_1.derivePortfolioPDA)(this.routerProgramId, user);
        const account = await this.connection.getAccountInfo(portfolioPDA);
        if (!account)
            return null;
        return (0, state_1.decodePortfolio)(account.data);
    }
    /**
     * Get orderbook for an instrument
     */
    async getOrderbook(slabState, instrumentIdx) {
        const account = await this.connection.getAccountInfo(slabState);
        if (!account)
            return null;
        return (0, state_1.decodeOrderbook)(account.data, instrumentIdx);
    }
    /**
     * Get slab PDA for a market
     */
    getSlabAddress(marketId) {
        const [pda] = (0, pda_1.deriveSlabPDA)(marketId, this.slabProgramId);
        return pda;
    }
    /**
     * Get portfolio PDA for a user
     */
    getPortfolioAddress(user) {
        const [pda] = (0, pda_1.derivePortfolioPDA)(this.routerProgramId, user);
        return pda;
    }
    /**
     * Get vault PDA for a mint
     */
    getVaultAddress(mint) {
        const [pda] = (0, pda_1.deriveVaultPDA)(this.routerProgramId, mint);
        return pda;
    }
    /**
     * Get escrow PDA
     */
    getEscrowAddress(slab, user, mint) {
        const [pda] = (0, pda_1.deriveEscrowPDA)(this.routerProgramId, slab, user, mint);
        return pda;
    }
}
exports.PercolatorClient = PercolatorClient;
