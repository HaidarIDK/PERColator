/**
 * State account decoders
 */
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { Portfolio, Orderbook } from './types';
/**
 * Decode Portfolio account
 */
export declare function decodePortfolio(data: Buffer): Portfolio;
/**
 * Decode Orderbook from slab state
 *
 * This is a simplified decoder - full implementation would walk linked lists
 */
export declare function decodeOrderbook(data: Buffer, instrumentIdx: number): Orderbook;
/**
 * Decode Vault account
 */
export declare function decodeVault(data: Buffer): {
    routerId: PublicKey;
    mint: PublicKey;
    tokenAccount: PublicKey;
    balance: BN;
    totalPledged: BN;
    available: BN;
};
/**
 * Decode Escrow account
 */
export declare function decodeEscrow(data: Buffer): {
    routerId: PublicKey;
    slabId: PublicKey;
    user: PublicKey;
    mint: PublicKey;
    balance: BN;
    nonce: BN;
    frozen: boolean;
};
