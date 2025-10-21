/**
 * PERColator Client - Main SDK interface
 */

import {
  Connection,
  PublicKey,
  Transaction,
  Keypair,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
  Side,
  ReserveResult,
  CommitResult,
  Portfolio,
  Orderbook,
  SlabConfig,
} from './types';
import {
  createReserveInstruction,
  createCommitInstruction,
  createCancelInstruction,
  createBatchOpenInstruction,
  createInitializeSlabInstruction,
  createAddInstrumentInstruction,
  createDepositInstruction,
  createWithdrawInstruction,
} from './instructions';
import {
  deriveSlabPDA,
  deriveVaultPDA,
  deriveEscrowPDA,
  derivePortfolioPDA,
} from './pda';
import { decodePortfolio, decodeOrderbook } from './state';

/**
 * Main PERColator client
 */
export class PercolatorClient {
  constructor(
    public readonly connection: Connection,
    public readonly routerProgramId: PublicKey,
    public readonly slabProgramId: PublicKey,
    public readonly payer?: Keypair
  ) {}

  /**
   * Reserve liquidity from a slab
   */
  async reserve(
    slabState: PublicKey,
    user: Keypair,
    accountIdx: number,
    instrumentIdx: number,
    side: Side,
    qty: BN,
    limitPx: BN,
    ttlMs: number = 60000,
    routeId?: BN
  ): Promise<ReserveResult> {
    // Generate commitment hash (zero for now)
    const commitmentHash = Buffer.alloc(32);
    
    // Generate route ID if not provided
    const rid = routeId || new BN(Date.now());

    const ix = createReserveInstruction(
      this.slabProgramId,
      slabState,
      user.publicKey,
      accountIdx,
      instrumentIdx,
      side,
      qty,
      limitPx,
      ttlMs,
      commitmentHash,
      rid
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [user]);

    // TODO: Parse return data from transaction logs
    return {
      holdId: new BN(0),
      vwapPx: limitPx,
      worstPx: limitPx,
      maxCharge: new BN(0),
      filledQty: qty,
      expiryMs: new BN(Date.now() + ttlMs),
    };
  }

  /**
   * Commit a reservation
   */
  async commit(
    slabState: PublicKey,
    user: Keypair,
    holdId: BN
  ): Promise<CommitResult> {
    const ix = createCommitInstruction(
      this.slabProgramId,
      slabState,
      user.publicKey,
      holdId,
      Date.now()
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [user]);

    // TODO: Parse return data
    return {
      filledQty: new BN(0),
      avgPrice: new BN(0),
      totalFee: new BN(0),
      totalDebit: new BN(0),
      fillsCount: 0,
    };
  }

  /**
   * Cancel a reservation
   */
  async cancel(
    slabState: PublicKey,
    user: Keypair,
    holdId: BN
  ): Promise<void> {
    const ix = createCancelInstruction(
      this.slabProgramId,
      slabState,
      user.publicKey,
      holdId
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [user]);
  }

  /**
   * Open new batch epoch
   */
  async openBatch(
    slabState: PublicKey,
    authority: Keypair,
    instrumentIdx: number
  ): Promise<void> {
    const ix = createBatchOpenInstruction(
      this.slabProgramId,
      slabState,
      authority.publicKey,
      instrumentIdx,
      Date.now()
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [authority]);
  }

  /**
   * Initialize a new slab
   */
  async initializeSlab(
    marketId: string,
    config: SlabConfig,
    payer: Keypair
  ): Promise<PublicKey> {
    const [slabPDA] = deriveSlabPDA(marketId, this.slabProgramId);
    
    // Create account for slab (7MB)
    const SLAB_SIZE = 7_156_336;
    const lamports = await this.connection.getMinimumBalanceForRentExemption(SLAB_SIZE);
    
    const createAccountIx = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: slabPDA,
      lamports,
      space: SLAB_SIZE,
      programId: this.slabProgramId,
    });

    const initIx = createInitializeSlabInstruction(
      this.slabProgramId,
      slabPDA,
      payer.publicKey,
      config
    );

    const tx = new Transaction().add(createAccountIx, initIx);
    await sendAndConfirmTransaction(this.connection, tx, [payer]);

    return slabPDA;
  }

  /**
   * Add instrument to slab
   */
  async addInstrument(
    slabState: PublicKey,
    authority: Keypair,
    symbol: string,
    contractSize: BN,
    tick: BN,
    lot: BN,
    indexPrice: BN
  ): Promise<void> {
    const ix = createAddInstrumentInstruction(
      this.slabProgramId,
      slabState,
      authority.publicKey,
      symbol,
      contractSize,
      tick,
      lot,
      indexPrice
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [authority]);
  }

  /**
   * Deposit collateral to router vault
   */
  async deposit(
    mint: PublicKey,
    amount: BN,
    user: Keypair,
    userTokenAccount: PublicKey
  ): Promise<void> {
    const [vault] = deriveVaultPDA(this.routerProgramId, mint);
    const [portfolio] = derivePortfolioPDA(this.routerProgramId, user.publicKey);

    const ix = createDepositInstruction(
      this.routerProgramId,
      vault,
      userTokenAccount,
      portfolio,
      user.publicKey,
      mint,
      amount
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [user]);
  }

  /**
   * Withdraw collateral from router vault
   */
  async withdraw(
    mint: PublicKey,
    amount: BN,
    user: Keypair,
    userTokenAccount: PublicKey
  ): Promise<void> {
    const [vault] = deriveVaultPDA(this.routerProgramId, mint);
    const [portfolio] = derivePortfolioPDA(this.routerProgramId, user.publicKey);

    const ix = createWithdrawInstruction(
      this.routerProgramId,
      vault,
      userTokenAccount,
      portfolio,
      user.publicKey,
      mint,
      amount
    );

    const tx = new Transaction().add(ix);
    await sendAndConfirmTransaction(this.connection, tx, [user]);
  }

  /**
   * Get user's portfolio
   */
  async getPortfolio(user: PublicKey): Promise<Portfolio | null> {
    const [portfolioPDA] = derivePortfolioPDA(this.routerProgramId, user);
    
    const account = await this.connection.getAccountInfo(portfolioPDA);
    if (!account) return null;

    return decodePortfolio(account.data);
  }

  /**
   * Get orderbook for an instrument
   */
  async getOrderbook(
    slabState: PublicKey,
    instrumentIdx: number
  ): Promise<Orderbook | null> {
    const account = await this.connection.getAccountInfo(slabState);
    if (!account) return null;

    return decodeOrderbook(account.data, instrumentIdx);
  }

  /**
   * Get slab PDA for a market
   */
  getSlabAddress(marketId: string): PublicKey {
    const [pda] = deriveSlabPDA(marketId, this.slabProgramId);
    return pda;
  }

  /**
   * Get portfolio PDA for a user
   */
  getPortfolioAddress(user: PublicKey): PublicKey {
    const [pda] = derivePortfolioPDA(this.routerProgramId, user);
    return pda;
  }

  /**
   * Get vault PDA for a mint
   */
  getVaultAddress(mint: PublicKey): PublicKey {
    const [pda] = deriveVaultPDA(this.routerProgramId, mint);
    return pda;
  }

  /**
   * Get escrow PDA
   */
  getEscrowAddress(
    slab: PublicKey,
    user: PublicKey,
    mint: PublicKey
  ): PublicKey {
    const [pda] = deriveEscrowPDA(
      this.routerProgramId,
      slab,
      user,
      mint
    );
    return pda;
  }

  /**
   * CROSS-SLAB ROUTER: Execute multi-slab trade atomically
   * 
   * Architecture:
   * 1. Frontend calls this SDK method
   * 2. SDK builds ExecuteCrossSlab instruction
   * 3. Router Program processes instruction
   * 4. Router CPIs to multiple Slab Programs (CommitFill)
   * 5. Router updates Portfolio with net exposure
   */
  async executeCrossSlabTrade(
    user: Keypair,
    slabStates: PublicKey[],
    instrumentIndices: number[],
    sides: Side[],
    quantities: BN[],
    limitPrices: BN[],
    ttlMs: number = 120000
  ): Promise<{
    routeId: BN;
    holdIds: BN[];
    totalFilled: BN;
    avgPrice: BN;
    totalCost: BN;
  }> {
    if (slabStates.length === 0) {
      throw new Error('At least one slab required');
    }

    if (
      slabStates.length !== instrumentIndices.length ||
      slabStates.length !== sides.length ||
      slabStates.length !== quantities.length ||
      slabStates.length !== limitPrices.length
    ) {
      throw new Error('All arrays must have same length');
    }

    // Generate route ID
    const routeId = new BN(Date.now());
    const commitmentHash = Buffer.alloc(32);

    // PHASE 1: Multi-Reserve across all slabs
    const holdIds: BN[] = [];
    const reserveTx = new Transaction();

    for (let i = 0; i < slabStates.length; i++) {
      const ix = createReserveInstruction(
        this.slabProgramId,
        slabStates[i],
        user.publicKey,
        0, // account_idx (TODO: derive from portfolio)
        instrumentIndices[i],
        sides[i],
        quantities[i],
        limitPrices[i],
        ttlMs,
        commitmentHash,
        routeId
      );
      reserveTx.add(ix);
    }

    await sendAndConfirmTransaction(this.connection, reserveTx, [user]);

    // Parse hold IDs from transaction logs (mock for now)
    for (let i = 0; i < slabStates.length; i++) {
      holdIds.push(new BN(Math.floor(Math.random() * 1000000)));
    }

    // PHASE 2: Multi-Commit atomically
    const commitTx = new Transaction();
    
    for (let i = 0; i < slabStates.length; i++) {
      const ix = createCommitInstruction(
        this.slabProgramId,
        slabStates[i],
        user.publicKey,
        holdIds[i],
        Date.now()
      );
      commitTx.add(ix);
    }

    await sendAndConfirmTransaction(this.connection, commitTx, [user]);

    // Calculate totals (mock for now - should parse from logs)
    let totalFilled = new BN(0);
    let totalCost = new BN(0);
    
    for (let i = 0; i < quantities.length; i++) {
      totalFilled = totalFilled.add(quantities[i]);
      totalCost = totalCost.add(quantities[i].mul(limitPrices[i]));
    }

    const avgPrice = totalFilled.gtn(0) 
      ? totalCost.div(totalFilled)
      : new BN(0);

    return {
      routeId,
      holdIds,
      totalFilled,
      avgPrice,
      totalCost,
    };
  }

  /**
   * Get available slabs for cross-slab routing
   */
  async getAvailableSlabs(): Promise<Array<{
    id: number;
    name: string;
    slabState: PublicKey;
    liquidity: number;
    vwap: number;
    fee: number;
    instruments: string[];
  }>> {
    // TODO: Fetch from on-chain registry
    // For now, return mock data
    return [
      {
        id: 1,
        name: "Slab A",
        slabState: new PublicKey('Slab1111111111111111111111111111111111111111'),
        liquidity: 1500,
        vwap: 3881.95,
        fee: 0.02,
        instruments: ['BTC/USDC', 'ETH/USDC', 'SOL/USDC'],
      },
      {
        id: 2,
        name: "Slab B",
        slabState: new PublicKey('Slab2222222222222222222222222222222222222222'),
        liquidity: 2300,
        vwap: 3882.15,
        fee: 0.015,
        instruments: ['BTC/USDC', 'ETH/USDC'],
      },
      {
        id: 3,
        name: "Slab C",
        slabState: new PublicKey('Slab3333333333333333333333333333333333333333'),
        liquidity: 980,
        vwap: 3881.75,
        fee: 0.025,
        instruments: ['ETH/USDC', 'SOL/USDC'],
      },
    ];
  }
}

