/**
 * Program Derived Address (PDA) helpers
 */

import { PublicKey } from '@solana/web3.js';

/**
 * Derive Slab PDA
 * Seeds: ["slab", market_id]
 */
export function deriveSlabPDA(
  marketId: string,
  programId: PublicKey
): [PublicKey, number] {
  const seeds = [
    Buffer.from('slab'),
    Buffer.from(marketId.padEnd(8, '\0').slice(0, 8)),
  ];
  return PublicKey.findProgramAddressSync(seeds, programId);
}

/**
 * Derive Vault PDA
 * Seeds: ["vault", router_id, mint]
 */
export function deriveVaultPDA(
  routerId: PublicKey,
  mint: PublicKey
): [PublicKey, number] {
  const seeds = [
    Buffer.from('vault'),
    routerId.toBuffer(),
    mint.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seeds, routerId);
}

/**
 * Derive Escrow PDA
 * Seeds: ["escrow", router_id, slab_id, user_id, mint]
 */
export function deriveEscrowPDA(
  routerId: PublicKey,
  slabId: PublicKey,
  user: PublicKey,
  mint: PublicKey
): [PublicKey, number] {
  const seeds = [
    Buffer.from('escrow'),
    routerId.toBuffer(),
    slabId.toBuffer(),
    user.toBuffer(),
    mint.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seeds, routerId);
}

/**
 * Derive Portfolio PDA
 * Seeds: ["portfolio", router_id, user]
 */
export function derivePortfolioPDA(
  routerId: PublicKey,
  user: PublicKey
): [PublicKey, number] {
  const seeds = [
    Buffer.from('portfolio'),
    routerId.toBuffer(),
    user.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seeds, routerId);
}

/**
 * Derive Cap (Capability) PDA
 * Seeds: ["cap", router_id, route_id]
 */
export function deriveCapPDA(
  routerId: PublicKey,
  routeId: bigint
): [PublicKey, number] {
  const routeIdBuffer = Buffer.alloc(8);
  routeIdBuffer.writeBigUInt64LE(routeId);
  
  const seeds = [
    Buffer.from('cap'),
    routerId.toBuffer(),
    routeIdBuffer,
  ];
  return PublicKey.findProgramAddressSync(seeds, routerId);
}

/**
 * Derive Registry PDA
 * Seeds: ["registry", router_id]
 */
export function deriveRegistryPDA(
  routerId: PublicKey
): [PublicKey, number] {
  const seeds = [
    Buffer.from('registry'),
    routerId.toBuffer(),
  ];
  return PublicKey.findProgramAddressSync(seeds, routerId);
}

