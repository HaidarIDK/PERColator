#!/usr/bin/env node

/**
 * Market Maker Bot
 * 
 * Continuously posts maker orders (bids and asks) to the SOL/USDC slab
 * and periodically updates them to create a live, moving orderbook.
 * 
 * Features:
 * - Posts 8-12 orders on each side (bid/ask)
 * - Refreshes orders every 30 seconds
 * - Adjusts prices based on mark price with some randomness
 * - Creates a realistic spread and depth
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
  console.log(`\n🤖 Market Maker Bot Starting...`);
  console.log(`   Wallet: ${wallet.publicKey.toBase58()}`);
} catch (err) {
  console.error(`❌ Failed to load wallet from ${WALLET_PATH}`);
  console.error('Run: solana-keygen new -o keypairs/devwallet.json');
  process.exit(1);
}

// Bot configuration
const CONFIG = {
  BASE_PRICE: 186.00,           // Base price for SOL
  PRICE_VOLATILITY: 0.02,       // 2% price movement
  SPREAD_PERCENTAGE: 0.005,     // 0.5% spread
  NUM_ORDERS_PER_SIDE: 8,       // 8 orders on each side
  ORDER_SIZE_MIN: 1.0,          // Min order size (SOL)
  ORDER_SIZE_MAX: 5.0,          // Max order size (SOL)
  REFRESH_INTERVAL_MS: 30000,   // Refresh orders every 30 seconds
  TICK_SIZE: 1.00,              // $1.00 tick size
};

let currentMarkPrice = CONFIG.BASE_PRICE;
let isRunning = true;
let orderCount = 0;

/**
 * Create PlaceOrder instruction
 */
function createPlaceOrderInstruction(side, price, qty) {
  const data = Buffer.alloc(20);
  data.writeUInt8(3, 0); // PlaceOrder discriminator
  data.writeUInt8(side, 1); // Side (0=Buy, 1=Sell)
  
  // Write i64 price
  const priceView = new DataView(data.buffer, data.byteOffset + 2, 8);
  priceView.setBigInt64(0, BigInt(price), true);
  
  // Write i64 qty
  const qtyView = new DataView(data.buffer, data.byteOffset + 10, 8);
  qtyView.setBigInt64(0, BigInt(qty), true);
  
  // post_only = true (maker orders only)
  data.writeUInt8(1, 18);
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
 * Round price to nearest tick
 */
function roundToTick(price) {
  return Math.round(price / CONFIG.TICK_SIZE) * CONFIG.TICK_SIZE;
}

/**
 * Generate random order size (aligned to lot size)
 */
function randomSize() {
  // Generate random size between min and max
  const raw = Math.random() * (CONFIG.ORDER_SIZE_MAX - CONFIG.ORDER_SIZE_MIN) + CONFIG.ORDER_SIZE_MIN;
  
  // Round to nearest whole number (lot size = 1.0 SOL)
  return Math.round(raw);
}

/**
 * Update mark price with some random movement
 */
function updateMarkPrice() {
  const change = (Math.random() - 0.5) * CONFIG.PRICE_VOLATILITY * currentMarkPrice;
  currentMarkPrice = Math.max(
    CONFIG.BASE_PRICE * 0.95,
    Math.min(CONFIG.BASE_PRICE * 1.05, currentMarkPrice + change)
  );
}

/**
 * Generate bid prices (below mark price)
 */
function generateBidPrices() {
  const bids = [];
  const spread = currentMarkPrice * CONFIG.SPREAD_PERCENTAGE;
  const topBid = currentMarkPrice - spread;
  
  for (let i = 0; i < CONFIG.NUM_ORDERS_PER_SIDE; i++) {
    // Spread bids from -0.5% to -4% below mark price
    const offset = (0.005 + (i * 0.004)) * currentMarkPrice;
    const price = roundToTick(currentMarkPrice - offset);
    const size = randomSize();
    
    if (price > 0) {
      bids.push({ price, size });
    }
  }
  
  return bids;
}

/**
 * Generate ask prices (above mark price)
 */
function generateAskPrices() {
  const asks = [];
  const spread = currentMarkPrice * CONFIG.SPREAD_PERCENTAGE;
  const topAsk = currentMarkPrice + spread;
  
  for (let i = 0; i < CONFIG.NUM_ORDERS_PER_SIDE; i++) {
    // Spread asks from +0.5% to +4% above mark price
    const offset = (0.005 + (i * 0.004)) * currentMarkPrice;
    const price = roundToTick(currentMarkPrice + offset);
    const size = randomSize();
    
    if (price > 0) {
      asks.push({ price, size });
    }
  }
  
  return asks;
}

/**
 * Post all orders for this cycle
 */
async function postOrders() {
  console.log(`\n📊 Posting Order Cycle #${++orderCount}`);
  console.log(`   Mark Price: $${currentMarkPrice.toFixed(2)}`);
  
  const bids = generateBidPrices();
  const asks = generateAskPrices();
  
  const allOrders = [
    ...bids.map(o => ({ side: 0, ...o, label: 'BID' })),
    ...asks.map(o => ({ side: 1, ...o, label: 'ASK' })),
  ];
  
  let successCount = 0;
  let failCount = 0;
  let errors = {};
  
  for (const order of allOrders) {
    try {
      const priceScaled = Math.floor(order.price * 1_000_000);
      const sizeScaled = Math.floor(order.size * 1_000_000);
      
      const instruction = createPlaceOrderInstruction(order.side, priceScaled, sizeScaled);
      const transaction = new Transaction().add(instruction);
      
      await sendAndConfirmTransaction(connection, transaction, [wallet], {
        skipPreflight: false,
        commitment: 'confirmed',
      });
      
      successCount++;
    } catch (error) {
      failCount++;
      const errorMsg = error.message || String(error);
      
      // Extract the key part of the error
      let shortMsg = 'Other';
      if (errorMsg.includes('not aligned')) shortMsg = 'Lot size alignment';
      else if (errorMsg.includes('duplicate')) shortMsg = 'Duplicate order';
      else if (errorMsg.includes('insufficient')) shortMsg = 'Insufficient funds';
      else if (errorMsg.includes('custom program error')) {
        const match = errorMsg.match(/0x[0-9a-f]+/);
        shortMsg = match ? `Program error ${match[0]}` : 'Program error';
      }
      
      errors[shortMsg] = (errors[shortMsg] || 0) + 1;
      
      // Log first error in detail
      if (failCount === 1) {
        console.log(`   First error details: ${errorMsg.slice(0, 150)}`);
      }
    }
  }
  
  // Show error summary if any
  if (failCount > 0 && Object.keys(errors).length > 0) {
    console.log('   Error breakdown:');
    for (const [msg, count] of Object.entries(errors)) {
      console.log(`     - ${msg}: ${count}x`);
    }
  }
  
  console.log(`   ✅ Posted ${successCount} orders`);
  if (failCount > 0) {
    console.log(`   ⚠️  ${failCount} orders failed (likely duplicates or low balance)`);
  }
  console.log(`   📈 ${bids.length} bids from $${bids[0].price.toFixed(2)} to $${bids[bids.length-1].price.toFixed(2)}`);
  console.log(`   📉 ${asks.length} asks from $${asks[0].price.toFixed(2)} to $${asks[asks.length-1].price.toFixed(2)}`);
}

/**
 * Main bot loop
 */
async function runBot() {
  console.log(`\n🚀 Market Maker Bot Active!`);
  console.log(`   Slab: ${SLAB_ACCOUNT.toBase58()}`);
  console.log(`   Refresh: Every ${CONFIG.REFRESH_INTERVAL_MS / 1000}s`);
  console.log(`   Orders per side: ${CONFIG.NUM_ORDERS_PER_SIDE}`);
  console.log(`\n⏸️  Press Ctrl+C to stop\n`);
  
  while (isRunning) {
    try {
      // Update mark price with some volatility
      updateMarkPrice();
      
      // Post new orders
      await postOrders();
      
      // Wait before next cycle
      await new Promise(resolve => setTimeout(resolve, CONFIG.REFRESH_INTERVAL_MS));
    } catch (error) {
      console.error('❌ Error in bot loop:', error.message);
      await new Promise(resolve => setTimeout(resolve, 5000));
    }
  }
}

/**
 * Graceful shutdown
 */
process.on('SIGINT', () => {
  console.log('\n\n🛑 Stopping Market Maker Bot...');
  isRunning = false;
  setTimeout(() => {
    console.log('✅ Bot stopped. Orders remain on-chain.');
    process.exit(0);
  }, 1000);
});

// Start the bot
(async () => {
  try {
    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`   Balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`);
    
    if (balance < 0.1 * LAMPORTS_PER_SOL) {
      console.warn('⚠️  Low balance! Request devnet SOL from faucet.');
    }
    
    await runBot();
  } catch (error) {
    console.error('❌ Fatal error:', error);
    process.exit(1);
  }
})();

