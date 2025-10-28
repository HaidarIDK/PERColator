/**
 * Initialize AMM Pools for PERColator DEX
 * Creates three pools: SOL/USDC, ETH/USDC, BTC/USDC
 */

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
  clusterApiUrl,
} from '@solana/web3.js';
import fs from 'fs';

// Program IDs (deployed on devnet with vanity addresses)
const AMM_PROGRAM_ID = new PublicKey('AMMjkEeFdasQ8fs9a9HQyJdciPHtDHVEat8yxiXrTP6p');
const ROUTER_PROGRAM_ID = new PublicKey('RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr');

// AMM instruction discriminator
const AmmInstruction = {
  Initialize: 0,
} as const;

// AMM State size: SlabHeader (200) + QuoteCache (136) + AmmPool (64) = 400 bytes
const AMM_STATE_SIZE = 400;

// Pool configurations
interface PoolConfig {
  name: string;
  symbol: string;
  instrumentPubkey: string; // For future use - oracle/instrument account
  markPrice: number; // Initial mark price (1e6 scale)
  xReserve: number; // Base token reserve (1e6 scale)
  yReserve: number; // USDC reserve (1e6 scale)
  takerFeeBps: number; // Taker fee in basis points
  contractSize: number; // Contract size (1e6 scale)
}

const POOLS: PoolConfig[] = [
  {
    name: 'SOL/USDC Pool',
    symbol: 'SOL',
    instrumentPubkey: '11111111111111111111111111111111', // Placeholder
    markPrice: 199_000_000, // $199 (1e6 scale)
    xReserve: 10_000_000_000_000, // 10,000 SOL (1e6 scale)
    yReserve: 2_000_000_000_000_000, // 2M USDC (1e6 scale)
    takerFeeBps: 30, // 0.30%
    contractSize: 1_000_000, // 1.0 contracts per SOL
  },
  {
    name: 'ETH/USDC Pool',
    symbol: 'ETH',
    instrumentPubkey: '11111111111111111111111111111112', // Placeholder
    markPrice: 4_130_000_000, // $4,130 (1e6 scale)
    xReserve: 100_000_000_000, // 100 ETH (1e6 scale)
    yReserve: 413_000_000_000_000, // 413k USDC (1e6 scale)
    takerFeeBps: 30, // 0.30%
    contractSize: 1_000_000, // 1.0 contracts per ETH
  },
  {
    name: 'BTC/USDC Pool',
    symbol: 'BTC',
    instrumentPubkey: '11111111111111111111111111111113', // Placeholder
    markPrice: 114_300_000_000, // $114,300 (1e6 scale)
    xReserve: 10_000_000_000, // 10 BTC (1e6 scale)
    yReserve: 1_143_000_000_000_000, // 1.143M USDC (1e6 scale)
    takerFeeBps: 30, // 0.30%
    contractSize: 1_000_000, // 1.0 contracts per BTC
  },
];

async function initializeAMMPool(
  connection: Connection,
  payer: Keypair,
  poolConfig: PoolConfig
): Promise<Keypair> {
  console.log(`\n🏊 Initializing ${poolConfig.name}...`);

  // Generate keypair for AMM account
  const ammKeypair = Keypair.generate();

  console.log(`   AMM Account: ${ammKeypair.publicKey.toBase58()}`);
  console.log(`   Size: ${AMM_STATE_SIZE} bytes`);

  // Calculate rent
  const rent = await connection.getMinimumBalanceForRentExemption(AMM_STATE_SIZE);
  console.log(`   Rent: ${(rent / 1e9).toFixed(8)} SOL`);

  // Create account
  const createIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: ammKeypair.publicKey,
    lamports: rent,
    space: AMM_STATE_SIZE,
    programId: AMM_PROGRAM_ID,
  });

  // Build Initialize instruction data
  // Layout:
  // [0] discriminator (u8)
  // [1..33] lp_owner (Pubkey)
  // [33..65] router_id (Pubkey)
  // [65..97] instrument (Pubkey)
  // [97..105] mark_px (i64)
  // [105..113] taker_fee_bps (i64)
  // [113..121] contract_size (i64)
  // [121] bump (u8)
  // [122..130] x_reserve (i64)
  // [130..138] y_reserve (i64)
  const data = Buffer.alloc(138);
  let offset = 0;

  // Discriminator
  data.writeUInt8(AmmInstruction.Initialize, offset);
  offset += 1;

  // LP Owner (payer)
  payer.publicKey.toBuffer().copy(data, offset);
  offset += 32;

  // Router ID
  ROUTER_PROGRAM_ID.toBuffer().copy(data, offset);
  offset += 32;

  // Instrument (placeholder pubkey)
  const instrumentPubkey = new PublicKey(poolConfig.instrumentPubkey);
  instrumentPubkey.toBuffer().copy(data, offset);
  offset += 32;

  // Mark Price (i64)
  data.writeBigInt64LE(BigInt(poolConfig.markPrice), offset);
  offset += 8;

  // Taker Fee BPS (i64)
  data.writeBigInt64LE(BigInt(poolConfig.takerFeeBps), offset);
  offset += 8;

  // Contract Size (i64)
  data.writeBigInt64LE(BigInt(poolConfig.contractSize), offset);
  offset += 8;

  // Bump (u8) - not used for regular accounts
  data.writeUInt8(255, offset);
  offset += 1;

  // X Reserve (i64)
  data.writeBigInt64LE(BigInt(poolConfig.xReserve), offset);
  offset += 8;

  // Y Reserve (i64)
  data.writeBigInt64LE(BigInt(poolConfig.yReserve), offset);

  const initializeIx = new TransactionInstruction({
    keys: [
      { pubkey: ammKeypair.publicKey, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
    ],
    programId: AMM_PROGRAM_ID,
    data,
  });

  const transaction = new Transaction()
    .add(createIx)
    .add(initializeIx);

  console.log(`   📡 Sending transaction...`);

  try {
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [payer, ammKeypair],
      {
        commitment: 'confirmed',
        preflightCommitment: 'confirmed',
      }
    );

    console.log(`   ✅ Pool initialized!`);
    console.log(`   Signature: ${signature}`);
    console.log(`   🔗 View on Solscan: https://solscan.io/tx/${signature}?cluster=devnet`);

    return ammKeypair;
  } catch (error: any) {
    console.error(`   ❌ Failed to initialize ${poolConfig.name}:`, error.message);
    if (error.logs) {
      console.log(`   📋 Transaction logs:`);
      error.logs.forEach((log: string) => console.log(`      ${log}`));
    }
    throw error;
  }
}

async function main() {
  console.log('🚀 PERColator AMM Pool Initialization\n');

  // Connect to devnet
  const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
  console.log('✅ Connected to Solana devnet');

  // Load payer keypair
  const keypairPath = 'perc-keypair.json';
  let payer: Keypair;

  try {
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
    payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
    console.log('📂 Loaded PERC vanity wallet');
  } catch {
    console.error('❌ Could not load perc-keypair.json');
    console.log('\n💡 Make sure scripts/perc-keypair.json exists.\n');
    process.exit(1);
  }

  console.log(`👛 Payer: ${payer.publicKey.toBase58()}`);

  // Check balance
  const balance = await connection.getBalance(payer.publicKey);
  console.log(`💰 Balance: ${(balance / 1e9).toFixed(8)} SOL`);

  // Check if we need airdrop
  if (balance < 1e9) {
    console.log(`\n🚰 Requesting airdrop...`);
    try {
      const signature = await connection.requestAirdrop(payer.publicKey, 2e9);
      await connection.confirmTransaction(signature);
      console.log(`✅ Airdropped 2 SOL`);
    } catch (error: any) {
      console.error(`❌ Airdrop failed: ${error.message}`);
      console.log(`\n💡 Get SOL from: https://faucet.solana.com/`);
      process.exit(1);
    }
  }

  console.log(`\n📊 Initializing ${POOLS.length} AMM pools...\n`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);

  const poolAccounts: Record<string, string> = {};

  // Initialize each pool
  for (const poolConfig of POOLS) {
    try {
      const ammKeypair = await initializeAMMPool(connection, payer, poolConfig);
      poolAccounts[poolConfig.symbol] = ammKeypair.publicKey.toBase58();

      // Save keypair to file
      fs.writeFileSync(
        `amm-pool-${poolConfig.symbol.toLowerCase()}-keypair.json`,
        JSON.stringify(Array.from(ammKeypair.secretKey))
      );
    } catch (error) {
      console.error(`\n❌ Skipping ${poolConfig.name} due to error`);
    }
  }

  console.log(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`\n✅ AMM Pool Initialization Complete!\n`);

  console.log(`📊 Pool Addresses:`);
  for (const [symbol, address] of Object.entries(poolAccounts)) {
    console.log(`   ${symbol}: ${address}`);
  }

  // Save pool info
  const poolInfo = {
    ammProgramId: AMM_PROGRAM_ID.toBase58(),
    routerProgramId: ROUTER_PROGRAM_ID.toBase58(),
    pools: poolAccounts,
    network: 'devnet',
    timestamp: new Date().toISOString(),
  };

  fs.writeFileSync(
    'amm-pools-info.json',
    JSON.stringify(poolInfo, null, 2)
  );

  console.log(`\n💾 Pool info saved to: scripts/amm-pools-info.json`);
  console.log(`\n🎉 AMM pools are ready for trading!`);
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

