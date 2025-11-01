#!/usr/bin/env node

/**
 * Add Test Liquidity to Slab
 * 
 * Posts maker orders (bids and asks) to the SOL/USDC slab
 * so users can test trading against real liquidity.
 */

const {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
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

// Load wallet (market maker)
const WALLET_PATH = path.join(__dirname, '..', 'keypairs', 'devwallet.json');
let wallet;
try {
  const walletData = JSON.parse(fs.readFileSync(WALLET_PATH, 'utf8'));
  wallet = Keypair.fromSecretKey(Uint8Array.from(walletData));
  console.log(`\n🤖 Market Maker: ${wallet.publicKey.toBase58()}`);
} catch (err) {
  console.error(`❌ Failed to load wallet from ${WALLET_PATH}`);
  console.error('Run: solana-keygen new -o keypairs/devwallet.json');
  process.exit(1);
}

/**
 * Create PlaceOrder instruction
 */
function createPlaceOrderInstruction(side, price, qty) {
  // Instruction data layout:
  // [0] discriminator: 3 (PlaceOrder)
  // [1] side: 0=Buy, 1=Sell
  // [2-9] price: i64 (1e6 scale)
  // [10-17] qty: i64 (1e6 scale)
  // [18] post_only: u8 (1=true, 0=false)
  // [19] reduce_only: u8 (1=true, 0=false)
  
  const data = Buffer.alloc(20);
  data.writeUInt8(3, 0); // PlaceOrder discriminator
  data.writeUInt8(side, 1); // Side
  
  // Write i64 price (little-endian)
  const priceView = new DataView(data.buffer, data.byteOffset + 2, 8);
  priceView.setBigInt64(0, BigInt(price), true);
  
  // Write i64 qty (little-endian)
  const qtyView = new DataView(data.buffer, data.byteOffset + 10, 8);
  qtyView.setBigInt64(0, BigInt(qty), true);
  
  // post_only = false, reduce_only = false
  data.writeUInt8(0, 18);
  data.writeUInt8(0, 19);
  
  return new TransactionInstruction({
    keys: [
      { pubkey: SLAB_ACCOUNT, isSigner: false, isWritable: true },
      { pubkey: wallet.publicKey, isSigner: true, isWritable: false },
    ],
    programId: SLAB_PROGRAM_ID,
    data,
  });
}

/**
 * Round price to nearest tick size
 */
function roundToTick(price, tickSize) {
  return Math.round(price / tickSize) * tickSize;
}

/**
 * Add liquidity to the slab
 */
async function addLiquidity() {
  console.log(`\n📊 Slab: ${SLAB_ACCOUNT.toBase58()}`);
  
  // Read slab configuration
  const slabAccount = await connection.getAccountInfo(SLAB_ACCOUNT);
  if (!slabAccount) {
    console.error('❌ Slab account not found');
    process.exit(1);
  }
  
  const slabData = new DataView(slabAccount.data.buffer);
  const tickSize = Number(slabData.getBigInt64(152, true)); // $1.00
  const lotSize = Number(slabData.getBigInt64(160, true));  // 1.0 SOL
  const minOrder = Number(slabData.getBigInt64(168, true)); // 1.0 SOL
  const markPrice = Number(slabData.getBigInt64(176, true)); // Current price
  
  console.log(`\n⚙️  Slab Configuration:`);
  console.log(`   Tick Size: $${(tickSize / 1_000_000).toFixed(2)}`);
  console.log(`   Lot Size: ${(lotSize / 1_000_000).toFixed(3)} SOL`);
  console.log(`   Min Order: ${(minOrder / 1_000_000).toFixed(3)} SOL`);
  console.log(`   Mark Price: $${(markPrice / 1_000_000).toFixed(2)}`);
  
  // Create a spread around the mark price
  // Bids (buy orders) below market, Asks (sell orders) above market
  // All prices rounded to tick size, all quantities >= min order
  const baseQty = Math.max(minOrder, 1_000_000); // 1 SOL minimum
  
  const orders = [
    // Bids (buy orders) - Side 0
    { side: 0, price: roundToTick(markPrice * 0.98, tickSize), qty: baseQty * 1, label: 'BID @ -2%' },
    { side: 0, price: roundToTick(markPrice * 0.96, tickSize), qty: baseQty * 2, label: 'BID @ -4%' },
    { side: 0, price: roundToTick(markPrice * 0.94, tickSize), qty: baseQty * 3, label: 'BID @ -6%' },
    { side: 0, price: roundToTick(markPrice * 0.92, tickSize), qty: baseQty * 5, label: 'BID @ -8%' },
    
    // Asks (sell orders) - Side 1
    { side: 1, price: roundToTick(markPrice * 1.02, tickSize), qty: baseQty * 1, label: 'ASK @ +2%' },
    { side: 1, price: roundToTick(markPrice * 1.04, tickSize), qty: baseQty * 2, label: 'ASK @ +4%' },
    { side: 1, price: roundToTick(markPrice * 1.06, tickSize), qty: baseQty * 3, label: 'ASK @ +6%' },
    { side: 1, price: roundToTick(markPrice * 1.08, tickSize), qty: baseQty * 5, label: 'ASK @ +8%' },
  ];
  
  console.log('\n📝 Posting Maker Orders...\n');
  
  for (const order of orders) {
    try {
      const instruction = createPlaceOrderInstruction(order.side, order.price, order.qty);
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
      
      const priceUsd = (order.price / 1_000_000).toFixed(2);
      const qtyDisplay = (order.qty / 1_000_000).toFixed(2);
      console.log(`✅ ${order.label}: ${qtyDisplay} SOL @ $${priceUsd}`);
      console.log(`   Signature: ${signature.slice(0, 20)}...`);
    } catch (err) {
      console.error(`❌ Failed to place ${order.label}:`, err.message);
    }
  }
  
  console.log('\n✅ Liquidity added! Users can now trade against these orders.\n');
  console.log('💡 Note: These orders are owned by your wallet. You can cancel them later.');
}

// Run
(async () => {
  try {
    // Check wallet balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`   Balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`);
    
    if (balance < 0.01 * LAMPORTS_PER_SOL) {
      console.error('\n❌ Insufficient balance. Need at least 0.01 SOL for transaction fees.');
      console.log('Run: solana airdrop 1 ' + wallet.publicKey.toBase58() + ' --url devnet');
      process.exit(1);
    }
    
    await addLiquidity();
  } catch (err) {
    console.error('\n❌ Error:', err);
    process.exit(1);
  }
})();

