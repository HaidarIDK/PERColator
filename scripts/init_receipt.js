#!/usr/bin/env node

/**
 * Initialize Receipt Account
 * 
 * Creates the receipt PDA account that the slab writes fill results to
 */

const {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Load config
const configPath = path.join(__dirname, '..', 'keypairs', 'config.json');
const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

// Constants
const ROUTER_PROGRAM_ID = new PublicKey(config.programs.router.program_id);
const SLAB_ACCOUNT = new PublicKey(config.trading_pairs['SOL-USD'].slab_address);
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

// Load wallet
const WALLET_PATH = path.join(__dirname, '..', 'keypairs', 'devwallet.json');
const walletData = JSON.parse(fs.readFileSync(WALLET_PATH, 'utf8'));
const wallet = Keypair.fromSecretKey(Uint8Array.from(walletData));

console.log(`\n🤖 Wallet: ${wallet.publicKey.toBase58()}`);

/**
 * Derive portfolio address (using createWithSeed, not PDA!)
 */
async function derivePortfolioAddress(userPubkey) {
  return await PublicKey.createWithSeed(
    userPubkey,
    'portfolio',
    ROUTER_PROGRAM_ID
  );
}

/**
 * Derive receipt PDA
 */
function deriveReceiptPDA(portfolioAddress, slabPubkey) {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from('receipt'),
      portfolioAddress.toBuffer(),
      slabPubkey.toBuffer(),
    ],
    ROUTER_PROGRAM_ID
  );
}

async function initializeReceipt() {
  // Derive portfolio and receipt addresses
  const portfolioAddress = await derivePortfolioAddress(wallet.publicKey);
  const [receiptPDA, receiptBump] = deriveReceiptPDA(portfolioAddress, SLAB_ACCOUNT);
  
  console.log(`\n📋 Portfolio: ${portfolioAddress.toBase58()}`);
  console.log(`📋 Slab: ${SLAB_ACCOUNT.toBase58()}`);
  console.log(`📋 Receipt PDA: ${receiptPDA.toBase58()}`);
  console.log(`📋 Bump: ${receiptBump}`);
  
  // Check if receipt already exists
  const receiptAccount = await connection.getAccountInfo(receiptPDA);
  if (receiptAccount) {
    console.log(`\n✅ Receipt already exists!`);
    console.log(`   Owner: ${receiptAccount.owner.toBase58()}`);
    console.log(`   Size: ${receiptAccount.data.length} bytes`);
    return;
  }
  
  console.log(`\n📝 Creating receipt account...`);
  
  // FillReceipt size = 48 bytes (7 fields × 8 bytes, but used:u32 is 4 bytes)
  // struct FillReceipt {
  //   used: u32 (4 bytes)
  //   seqno_committed: u32 (4 bytes)
  //   filled_qty: i64 (8 bytes)
  //   vwap_px: i64 (8 bytes)
  //   notional: i64 (8 bytes)
  //   fee: i64 (8 bytes)
  //   pnl_delta: i64 (8 bytes)
  // } = 48 bytes
  const RECEIPT_SIZE = 48;
  
  // Calculate rent
  const rentExemption = await connection.getMinimumBalanceForRentExemption(RECEIPT_SIZE);
  
  console.log(`   Space: ${RECEIPT_SIZE} bytes`);
  console.log(`   Rent: ${(rentExemption / 1e9).toFixed(6)} SOL`);
  
  // Create the receipt PDA account
  const transaction = new Transaction().add(
    SystemProgram.createAccountWithSeed({
      fromPubkey: wallet.publicKey,
      basePubkey: portfolioAddress,
      seed: `receipt_${SLAB_ACCOUNT.toBase58().slice(0, 8)}`,
      newAccountPubkey: receiptPDA,
      lamports: rentExemption,
      space: RECEIPT_SIZE,
      programId: ROUTER_PROGRAM_ID,
    })
  );
  
  const signature = await sendAndConfirmTransaction(
    connection,
    transaction,
    [wallet],
    {
      skipPreflight: false,
      commitment: 'confirmed',
    }
  );
  
  console.log(`\n✅ Receipt initialized!`);
  console.log(`   Signature: ${signature}`);
  console.log(`   Address: ${receiptPDA.toBase58()}`);
}

// Run
(async () => {
  try {
    await initializeReceipt();
  } catch (err) {
    console.error('\n❌ Error:', err.message);
    if (err.logs) {
      console.error('\n📋 Logs:');
      err.logs.forEach(log => console.error('  ', log));
    }
    process.exit(1);
  }
})();

