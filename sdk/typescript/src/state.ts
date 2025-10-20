/**
 * State account decoders
 */

import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { Portfolio, Orderbook, OrderbookLevel } from './types';
import { readBN } from './pda';

/**
 * Decode Portfolio account
 */
export function decodePortfolio(data: Buffer): Portfolio {
  let offset = 0;
  
  const routerId = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  const user = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  
  // Read equity (i128)
  const equity = readBN(data, offset, 16);
  offset += 16;
  
  const im = readBN(data, offset, 16);
  offset += 16;
  const mm = readBN(data, offset, 16);
  offset += 16;
  
  const freeCollateral = readBN(data, offset, 16);
  offset += 16;
  
  const lastMarkTs = readBN(data, offset, 8);
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
    const qty = readBN(data, offset, 8);
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
export function decodeOrderbook(
  data: Buffer,
  instrumentIdx: number
): Orderbook {
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
export function decodeVault(data: Buffer) {
  let offset = 0;
  
  const routerId = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  const mint = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  const tokenAccount = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  
  const balance = readBN(data, offset, 16);
  offset += 16;
  const totalPledged = readBN(data, offset, 16);
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
export function decodeEscrow(data: Buffer) {
  let offset = 0;
  
  const routerId = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  const slabId = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  const user = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  const mint = new PublicKey(data.slice(offset, offset + 32));
  offset += 32;
  
  const balance = readBN(data, offset, 16);
  offset += 16;
  const nonce = readBN(data, offset, 8);
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

