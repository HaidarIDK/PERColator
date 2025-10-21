#!/usr/bin/env ts-node
/**
 * Create a simple slab account for testing (no initialization needed)
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

const SLAB_PROGRAM_ID = new PublicKey('6EF2acRfPejnxXYd9apKc2wb3p2NLG8rKgWbCfp5G7Uz');
const SLAB_ACCOUNT_SIZE = 128 * 1024; // 128 KB

async function main() {
  console.log('🚀 Creating Slab Account for Testing\n');

  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  console.log('✅ Connected to devnet');

  // Load payer
  let payer: Keypair;
  if (fs.existsSync('slab-payer.json')) {
    const keypairData = JSON.parse(fs.readFileSync('slab-payer.json', 'utf-8'));
    payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
  } else {
    payer = Keypair.generate();
    fs.writeFileSync('slab-payer.json', JSON.stringify(Array.from(payer.secretKey)));
  }

  console.log(`👛 Payer: ${payer.publicKey.toBase58()}`);

  // Check balance
  let balance = await connection.getBalance(payer.publicKey);
  console.log(`💰 Balance: ${balance / LAMPORTS_PER_SOL} SOL`);

  const rentExemption = await connection.getMinimumBalanceForRentExemption(SLAB_ACCOUNT_SIZE);
  console.log(`💵 Rent needed: ${rentExemption / LAMPORTS_PER_SOL} SOL`);

  if (balance < rentExemption + 0.1 * LAMPORTS_PER_SOL) {
    console.log('\n🚰 Need more SOL! Getting from faucet...');
    try {
      const airdrop = await connection.requestAirdrop(payer.publicKey, 2 * LAMPORTS_PER_SOL);
      await connection.confirmTransaction(airdrop);
      balance = await connection.getBalance(payer.publicKey);
      console.log(`✅ New balance: ${balance / LAMPORTS_PER_SOL} SOL`);
    } catch (e: any) {
      console.log(`❌ Airdrop failed: ${e.message}`);
      console.log(`\n💡 Get SOL from: https://faucet.solana.com`);
      console.log(`   Address: ${payer.publicKey.toBase58()}`);
      process.exit(1);
    }
  }

  // Create slab account
  const slabAccount = Keypair.generate();
  console.log(`\n📦 Creating slab account: ${slabAccount.publicKey.toBase58()}`);

  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: slabAccount.publicKey,
    lamports: rentExemption,
    space: SLAB_ACCOUNT_SIZE,
    programId: SLAB_PROGRAM_ID,  // ← This makes it owned by the program!
  });

  const transaction = new Transaction().add(createAccountIx);

  try {
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [payer, slabAccount],
      { commitment: 'confirmed' }
    );

    console.log('\n🎉 SUCCESS! Slab account created!');
    console.log(`\n📊 Details:`);
    console.log(`   Address: ${slabAccount.publicKey.toBase58()}`);
    console.log(`   Owner: ${SLAB_PROGRAM_ID.toBase58()}`);
    console.log(`   Size: ${SLAB_ACCOUNT_SIZE / 1024} KB`);
    console.log(`   Rent: ${rentExemption / LAMPORTS_PER_SOL} SOL`);
    console.log(`   Transaction: ${signature}`);
    
    console.log(`\n🔍 View on Explorer:`);
    console.log(`   https://explorer.solana.com/address/${slabAccount.publicKey.toBase58()}?cluster=devnet`);
    
    // Save to file
    const config = {
      slabAccount: slabAccount.publicKey.toBase58(),
      programId: SLAB_PROGRAM_ID.toBase58(),
      signature,
      timestamp: new Date().toISOString(),
    };
    
    fs.writeFileSync('slab-account.json', JSON.stringify(config, null, 2));
    console.log(`\n💾 Saved to slab-account.json`);
    
    // Update backend env
    console.log(`\n✅ Add to api/.env:`);
    console.log(`SLAB_ACCOUNT=${slabAccount.publicKey.toBase58()}`);
    
    console.log(`\n🚀 Now run initialize-slab.ts to set it up!`);
    
  } catch (error: any) {
    console.error('\n❌ Failed:', error.message);
    process.exit(1);
  }
}

main();

