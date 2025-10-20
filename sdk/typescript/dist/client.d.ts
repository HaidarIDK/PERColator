/**
 * PERColator Client - Main SDK interface
 */
import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import BN from 'bn.js';
import { Side, ReserveResult, CommitResult, Portfolio, Orderbook, SlabConfig } from './types';
/**
 * Main PERColator client
 */
export declare class PercolatorClient {
    readonly connection: Connection;
    readonly routerProgramId: PublicKey;
    readonly slabProgramId: PublicKey;
    readonly payer?: Keypair | undefined;
    constructor(connection: Connection, routerProgramId: PublicKey, slabProgramId: PublicKey, payer?: Keypair | undefined);
    /**
     * Reserve liquidity from a slab
     */
    reserve(slabState: PublicKey, user: Keypair, accountIdx: number, instrumentIdx: number, side: Side, qty: BN, limitPx: BN, ttlMs?: number, routeId?: BN): Promise<ReserveResult>;
    /**
     * Commit a reservation
     */
    commit(slabState: PublicKey, user: Keypair, holdId: BN): Promise<CommitResult>;
    /**
     * Cancel a reservation
     */
    cancel(slabState: PublicKey, user: Keypair, holdId: BN): Promise<void>;
    /**
     * Open new batch epoch
     */
    openBatch(slabState: PublicKey, authority: Keypair, instrumentIdx: number): Promise<void>;
    /**
     * Initialize a new slab
     */
    initializeSlab(marketId: string, config: SlabConfig, payer: Keypair): Promise<PublicKey>;
    /**
     * Add instrument to slab
     */
    addInstrument(slabState: PublicKey, authority: Keypair, symbol: string, contractSize: BN, tick: BN, lot: BN, indexPrice: BN): Promise<void>;
    /**
     * Deposit collateral to router vault
     */
    deposit(mint: PublicKey, amount: BN, user: Keypair, userTokenAccount: PublicKey): Promise<void>;
    /**
     * Withdraw collateral from router vault
     */
    withdraw(mint: PublicKey, amount: BN, user: Keypair, userTokenAccount: PublicKey): Promise<void>;
    /**
     * Get user's portfolio
     */
    getPortfolio(user: PublicKey): Promise<Portfolio | null>;
    /**
     * Get orderbook for an instrument
     */
    getOrderbook(slabState: PublicKey, instrumentIdx: number): Promise<Orderbook | null>;
    /**
     * Get slab PDA for a market
     */
    getSlabAddress(marketId: string): PublicKey;
    /**
     * Get portfolio PDA for a user
     */
    getPortfolioAddress(user: PublicKey): PublicKey;
    /**
     * Get vault PDA for a mint
     */
    getVaultAddress(mint: PublicKey): PublicKey;
    /**
     * Get escrow PDA
     */
    getEscrowAddress(slab: PublicKey, user: PublicKey, mint: PublicKey): PublicKey;
}
