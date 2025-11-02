#!/usr/bin/env node

/**
 * Cancel All Orders
 * 
 * Cancels all orders from this wallet on the slab
 * to clear the orderbook for fresh orders
 */

const {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Load config
const configPath = path.join(__dirname, '..', 'keypairs', 'config.json');
const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

// Program IDs
const SLAB_PROGRAM_ID = new PublicKey(config.programs.slab.program_id);
const SLAB_ACCOUNT = new PublicKey(config.trading_pairs['SOL-USD'].slab_address);

// RPC connection
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

// Load wallet
const WALLET_PATH = path.join(__dirname, '..', 'keypairs', 'devwallet.json');
let wallet;
try {
  const walletData = JSON.parse(fs.readFileSync(WALLET_PATH, 'utf8'));
  wallet = Keypair.fromSecretKey(Uint8Array.from(walletData));
  console.log(`\n🧹 Cancel All Orders`);
  console.log(`   Wallet: ${wallet.publicKey.toBase58()}`);
  console.log(`   Slab: ${SLAB_ACCOUNT.toBase58()}\n`);
} catch (err) {
  console.error(`❌ Failed to load wallet from ${WALLET_PATH}`);
  process.exit(1);
}

/**
 * Create CancelAllOrders instruction
 */
function createCancelAllOrdersInstruction() {
  // Instruction data: [discriminator: 5 (CancelAll)]
  const data = Buffer.alloc(1);
  data.writeUInt8(5, 0); // CancelAll discriminator
  
  return new TransactionInstruction({
    keys: [
      { pubkey: SLAB_ACCOUNT, isSigner: false, isWritable: true },
      { pubkey: wallet.publicKey, isSigner: true, isWritable: false },
    ],
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

async function cancelAll() {
  try {
    console.log('Sending CancelAll instruction...');
    
    const instruction = createCancelAllOrdersInstruction();
    const transaction = new Transaction().add(instruction);
    
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [wallet],
      {
        skipPreflight: false,
        commitment: 'confirmed',
      }
    );
    
    console.log(`✅ All orders cancelled!`);
    console.log(`   Signature: ${signature.slice(0, 20)}...`);
    console.log(`\n💡 Now the market maker bot can post fresh orders!`);
    
  } catch (error) {
    console.error('❌ Failed to cancel orders:', error.message);
    
    if (error.message.includes('0x')) {
      const match = error.message.match(/0x[0-9a-f]+/);
      if (match) {
        console.log(`   Error code: ${match[0]}`);
      }
    }
    
    process.exit(1);
  }
}

cancelAll();


