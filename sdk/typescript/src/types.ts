/**
 * Type definitions for PERColator protocol
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

/**
 * Trading side
 */
export enum Side {
  Buy = 0,
  Sell = 1,
}

/**
 * Order time-in-force
 */
export enum TimeInForce {
  GTC = 0,  // Good-til-canceled
  IOC = 1,  // Immediate-or-cancel
  FOK = 2,  // Fill-or-kill
}

/**
 * Maker class (DLP = Designated Liquidity Provider)
 */
export enum MakerClass {
  Regular = 0,
  DLP = 1,
}

/**
 * Order state
 */
export enum OrderState {
  Live = 0,
  Pending = 1,
}

/**
 * Instrument configuration
 */
export interface Instrument {
  symbol: string;
  contractSize: BN;
  tick: BN;
  lot: BN;
  indexPrice: BN;
  fundingRate: number;
  cumFunding: BN;
  lastFundingTs: BN;
}

/**
 * Order information
 */
export interface Order {
  orderId: BN;
  accountIdx: number;
  instrumentIdx: number;
  side: Side;
  price: BN;
  qty: BN;
  qtyOrig: BN;
  reservedQty: BN;
  timeInForce: TimeInForce;
  makerClass: MakerClass;
  state: OrderState;
  eligibleEpoch: number;
  createdMs: BN;
}

/**
 * Position information
 */
export interface Position {
  instrumentIdx: number;
  qty: BN;  // Signed: positive = long, negative = short
  entryPx: BN;
  lastFunding: BN;
  realizedPnl: BN;
}

/**
 * Reserve result from slab
 */
export interface ReserveResult {
  holdId: BN;
  vwapPx: BN;
  worstPx: BN;
  maxCharge: BN;
  filledQty: BN;
  expiryMs: BN;
}

/**
 * Commit result from slab
 */
export interface CommitResult {
  filledQty: BN;
  avgPrice: BN;
  totalFee: BN;
  totalDebit: BN;
  fillsCount: number;
}

/**
 * Portfolio state
 */
export interface Portfolio {
  routerId: PublicKey;
  user: PublicKey;
  equity: BN;  // Signed
  im: BN;
  mm: BN;
  freeCollateral: BN;  // Signed
  lastMarkTs: BN;
  exposures: Array<{
    slabIdx: number;
    instrumentIdx: number;
    qty: BN;  // Signed
  }>;
}

/**
 * Capability token
 */
export interface Cap {
  routerId: PublicKey;
  routeId: BN;
  scopeUser: PublicKey;
  scopeSlab: PublicKey;
  scopeMint: PublicKey;
  amountMax: BN;
  remaining: BN;
  expiryTs: BN;
  nonce: BN;
  burned: boolean;
}

/**
 * Escrow account
 */
export interface Escrow {
  routerId: PublicKey;
  slabId: PublicKey;
  user: PublicKey;
  mint: PublicKey;
  balance: BN;
  nonce: BN;
  frozen: boolean;
}

/**
 * Vault account
 */
export interface Vault {
  routerId: PublicKey;
  mint: PublicKey;
  tokenAccount: PublicKey;
  balance: BN;
  totalPledged: BN;
}

/**
 * Market statistics
 */
export interface MarketStats {
  symbol: string;
  indexPrice: BN;
  markPrice: BN;
  fundingRate: number;
  openInterest: BN;
  volume24h: BN;
  high24h: BN;
  low24h: BN;
  change24h: number;
}

/**
 * Orderbook level
 */
export interface OrderbookLevel {
  price: BN;
  qty: BN;
  orders: number;
}

/**
 * Orderbook snapshot
 */
export interface Orderbook {
  instrumentIdx: number;
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  lastUpdate: number;
}

/**
 * Trade execution
 */
export interface Trade {
  timestamp: BN;
  instrumentIdx: number;
  side: Side;
  price: BN;
  qty: BN;
  makerOrderId: BN;
  takerOrderId: BN;
}

/**
 * Liquidation result
 */
export interface LiquidationResult {
  closedQty: BN;  // Signed
  realizedPnl: BN;  // Signed
  closedNotional: BN;
  liquidationFee: BN;
  remainingDeficit: BN;
}

/**
 * Configuration for slab initialization
 */
export interface SlabConfig {
  authority: PublicKey;
  oracle: PublicKey;
  router: PublicKey;
  imr: number;  // Initial margin ratio in bps
  mmr: number;  // Maintenance margin ratio in bps
  makerFee: number;  // Maker fee in bps (can be negative)
  takerFee: number;  // Taker fee in bps
  batchMs: number;  // Batch window in milliseconds
  freezeLevels: number;  // Number of price levels to freeze
}

/**
 * Instruction discriminators
 */
export enum SlabInstruction {
  Reserve = 0,
  Commit = 1,
  Cancel = 2,
  BatchOpen = 3,
  Initialize = 4,
  AddInstrument = 5,
  UpdateFunding = 6,
  Liquidate = 7,
}

export enum RouterInstruction {
  Initialize = 0,
  Deposit = 1,
  Withdraw = 2,
  MultiReserve = 3,
  MultiCommit = 4,
  Liquidate = 5,
}

