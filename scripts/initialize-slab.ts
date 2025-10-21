#!/usr/bin/env ts-node
/**
 * Initialize Percolator Slab Program on Devnet
 * 
 * This script creates and initializes a Slab account for the order book.
 */

import { 
  Connection, 
  Keypair, 
  PublicKey, 
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL
} from '@solana/web3.js';
import * as fs from 'fs';

// Program IDs (deployed on devnet)
const SLAB_PROGRAM_ID = new PublicKey('6EF2acRfPejnxXYd9apKc2wb3p2NLG8rKgWbCfp5G7Uz');
const ROUTER_PROGRAM_ID = new PublicKey('9CQWTSDobkHqWzvx4nufdke4C8GKuoaqiNBBLEYFoHoG');

// Slab state size: OPTIMIZED 128 KB (actual ~60KB)
// Ultra-cheap: ~0.5 SOL rent instead of 73 SOL!
// Supports: 50 users, 300 orders, 100 positions, 50 reservations
const SLAB_ACCOUNT_SIZE = 128 * 1024; // 128 KB - ULTRA-CHEAP!

async function main() {
  console.log('🚀 Percolator Slab Initialization Script\n');

  // Connect to Solana devnet
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  console.log('✅ Connected to Solana devnet');

  // Load or create payer keypair
  let payer: Keypair;
  const KEYPAIR_PATH = 'slab-payer.json';
  
  if (fs.existsSync(KEYPAIR_PATH)) {
    console.log('📂 Loading existing keypair...');
    const keypairData = JSON.parse(fs.readFileSync(KEYPAIR_PATH, 'utf-8'));
    payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
  } else {
    console.log('🔑 Generating new keypair...');
    payer = Keypair.generate();
    fs.writeFileSync(KEYPAIR_PATH, JSON.stringify(Array.from(payer.secretKey)));
    console.log(`💾 Keypair saved to ${KEYPAIR_PATH}`);
  }

  console.log(`👛 Payer: ${payer.publicKey.toBase58()}`);

  // Check balance and request airdrops if needed
  let balance = await connection.getBalance(payer.publicKey);
  console.log(`💰 Balance: ${balance / LAMPORTS_PER_SOL} SOL`);

  // Calculate rent needed (with some buffer for transaction fees)
  const rentExemptionEstimate = await connection.getMinimumBalanceForRentExemption(SLAB_ACCOUNT_SIZE);
  const totalNeeded = rentExemptionEstimate + (0.1 * LAMPORTS_PER_SOL); // Add 0.1 SOL for fees
  
  console.log(`💵 Estimated rent needed: ${rentExemptionEstimate / LAMPORTS_PER_SOL} SOL`);
  console.log(`📊 Total needed (with fees): ${totalNeeded / LAMPORTS_PER_SOL} SOL`);

  if (balance < totalNeeded) {
    const solNeeded = Math.ceil((totalNeeded - balance) / LAMPORTS_PER_SOL);
    console.log(`\n⚠️  Insufficient balance! Need ${solNeeded} more SOL.`);
    console.log(`\n🚰 Requesting airdrops from faucet...`);
    console.log(`   (This may take multiple attempts due to rate limits)\n`);
    
    // Request multiple small airdrops (max 2 SOL per request)
    const airdropAttempts = Math.ceil(solNeeded / 2);
    let successfulAirdrops = 0;
    
    for (let i = 0; i < airdropAttempts && balance < totalNeeded; i++) {
      try {
        console.log(`   Airdrop ${i + 1}/${airdropAttempts}...`);
        const airdropAmount = Math.min(2 * LAMPORTS_PER_SOL, totalNeeded - balance);
        const airdropSig = await connection.requestAirdrop(payer.publicKey, airdropAmount);
        await connection.confirmTransaction(airdropSig, 'confirmed');
        successfulAirdrops++;
        balance = await connection.getBalance(payer.publicKey);
        console.log(`   ✅ Success! New balance: ${balance / LAMPORTS_PER_SOL} SOL`);
        
        // Wait between airdrops to avoid rate limit
        if (i < airdropAttempts - 1 && balance < totalNeeded) {
          console.log(`   ⏳ Waiting 5 seconds before next airdrop...`);
          await new Promise(resolve => setTimeout(resolve, 5000));
        }
      } catch (error: any) {
        console.log(`   ❌ Airdrop ${i + 1} failed: ${error.message}`);
      }
    }
    
    // Check final balance
    balance = await connection.getBalance(payer.publicKey);
    if (balance < totalNeeded) {
      console.log(`\n❌ Still insufficient balance after ${successfulAirdrops} airdrops.`);
      console.log(`   Current: ${balance / LAMPORTS_PER_SOL} SOL`);
      console.log(`   Needed: ${totalNeeded / LAMPORTS_PER_SOL} SOL`);
      console.log(`\n💡 Solutions:`);
      console.log(`   1. Get more SOL from web faucet: https://faucet.solana.com/`);
      console.log(`      Address: ${payer.publicKey.toBase58()}`);
      console.log(`   2. Wait a few minutes and run script again (rate limit resets)`);
      console.log(`   3. Use a different wallet with more SOL`);
      process.exit(1);
    }
    
    console.log(`\n✅ Sufficient balance acquired! (${balance / LAMPORTS_PER_SOL} SOL)\n`);
  }

  // Create Slab account
  console.log('\n📦 Creating Slab account...');
  const slabAccount = Keypair.generate();
  console.log(`🆔 Slab Account: ${slabAccount.publicKey.toBase58()}`);

  const rentExemption = await connection.getMinimumBalanceForRentExemption(SLAB_ACCOUNT_SIZE);
  console.log(`💵 Rent required: ${rentExemption / LAMPORTS_PER_SOL} SOL`);

  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: slabAccount.publicKey,
    lamports: rentExemption,
    space: SLAB_ACCOUNT_SIZE,
    programId: SLAB_PROGRAM_ID,
  });

  // Build Initialize instruction
  console.log('\n🔧 Building Initialize instruction...');
  
  // Instruction data layout:
  // - 1 byte: discriminator (4 = Initialize)
  // - 32 bytes: authority pubkey
  // - 32 bytes: oracle pubkey (can be dummy for now)
  // - 32 bytes: router pubkey
  // - 2 bytes: imr (initial margin ratio, basis points)
  // - 2 bytes: mmr (maintenance margin ratio, basis points)
  // - 8 bytes: maker_fee (can be negative for rebate)
  // - 8 bytes: taker_fee
  // - 8 bytes: batch_ms (batch window duration)
  // - 2 bytes: freeze_levels
  
  const instructionData = Buffer.alloc(1 + 32 + 32 + 32 + 2 + 2 + 8 + 8 + 8 + 2);
  let offset = 0;
  
  // Discriminator: 4 = Initialize
  instructionData.writeUInt8(4, offset);
  offset += 1;
  
  // Authority (LP owner) - use payer
  payer.publicKey.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // Oracle pubkey (dummy for now - use system program)
  SystemProgram.programId.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // Router pubkey
  ROUTER_PROGRAM_ID.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // IMR: 500 basis points = 5%
  instructionData.writeUInt16LE(500, offset);
  offset += 2;
  
  // MMR: 300 basis points = 3%
  instructionData.writeUInt16LE(300, offset);
  offset += 2;
  
  // Maker fee: -2 basis points = -0.02% (rebate)
  instructionData.writeBigInt64LE(BigInt(-2), offset);
  offset += 8;
  
  // Taker fee: 5 basis points = 0.05%
  instructionData.writeBigInt64LE(BigInt(5), offset);
  offset += 8;
  
  // Batch window: 100ms
  instructionData.writeBigUInt64LE(BigInt(100), offset);
  offset += 8;
  
  // Freeze levels: 3
  instructionData.writeUInt16LE(3, offset);
  
  const initializeIx = {
    programId: SLAB_PROGRAM_ID,
    keys: [
      { pubkey: slabAccount.publicKey, isSigner: false, isWritable: true },
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
    ],
    data: instructionData,
  };

  // Build and send transaction
  console.log('\n📤 Submitting transaction...');
  const transaction = new Transaction()
    .add(createAccountIx)
    .add(initializeIx);

  try {
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [payer, slabAccount],
      { commitment: 'confirmed' }
    );

    console.log('\n🎉 SUCCESS! Slab initialized!');
    console.log(`\n📊 Details:`);
    console.log(`   Slab Account: ${slabAccount.publicKey.toBase58()}`);
    console.log(`   Transaction: ${signature}`);
    console.log(`\n🔍 View on Explorer:`);
    console.log(`   https://explorer.solana.com/tx/${signature}?cluster=devnet`);
    console.log(`   https://explorer.solana.com/address/${slabAccount.publicKey.toBase58()}?cluster=devnet`);
    
    // Save the Slab account address
    fs.writeFileSync(
      'slab-account.json',
      JSON.stringify({
        slabAccount: slabAccount.publicKey.toBase58(),
        programId: SLAB_PROGRAM_ID.toBase58(),
        transaction: signature,
        timestamp: new Date().toISOString(),
      }, null, 2)
    );
    console.log(`\n💾 Slab account info saved to slab-account.json`);
    
    console.log(`\n✅ Next steps:`);
    console.log(`   1. Update backend to use this Slab account:`);
    console.log(`      SLAB_ACCOUNT=${slabAccount.publicKey.toBase58()}`);
    console.log(`   2. Test Reserve/Commit transactions!`);
    
  } catch (error: any) {
    console.error('\n❌ Initialization failed:', error);
    if (error.logs) {
      console.error('\n📝 Transaction logs:');
      error.logs.forEach((log: string) => console.error(`   ${log}`));
    }
    process.exit(1);
  }
}

main().catch(console.error);

