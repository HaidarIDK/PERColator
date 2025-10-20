/**
 * Program Derived Address (PDA) helpers
 */
import { PublicKey } from '@solana/web3.js';
/**
 * Derive Slab PDA
 * Seeds: ["slab", market_id]
 */
export declare function deriveSlabPDA(marketId: string, programId: PublicKey): [PublicKey, number];
/**
 * Derive Vault PDA
 * Seeds: ["vault", router_id, mint]
 */
export declare function deriveVaultPDA(routerId: PublicKey, mint: PublicKey): [PublicKey, number];
/**
 * Derive Escrow PDA
 * Seeds: ["escrow", router_id, slab_id, user_id, mint]
 */
export declare function deriveEscrowPDA(routerId: PublicKey, slabId: PublicKey, user: PublicKey, mint: PublicKey): [PublicKey, number];
/**
 * Derive Portfolio PDA
 * Seeds: ["portfolio", router_id, user]
 */
export declare function derivePortfolioPDA(routerId: PublicKey, user: PublicKey): [PublicKey, number];
/**
 * Derive Cap (Capability) PDA
 * Seeds: ["cap", router_id, route_id]
 */
export declare function deriveCapPDA(routerId: PublicKey, routeId: bigint): [PublicKey, number];
/**
 * Derive Registry PDA
 * Seeds: ["registry", router_id]
 */
export declare function deriveRegistryPDA(routerId: PublicKey): [PublicKey, number];
