/**
 * SIMPLE DEMO - How Frontend Uses the SDK
 * 
 * This shows exactly what your frontend developer needs to do
 */

import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { 
  PercolatorClient, 
  Side, 
  priceToProtocol, 
  qtyToProtocol,
  priceFromProtocol,
  qtyFromProtocol
} from '../src';

async function demo() {
  console.log('🚀 PERColator SDK Demo\n');

  // ========================================
  // 1. SETUP (One-time initialization)
  // ========================================
  
  console.log('📡 Connecting to Solana devnet...');
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  // In real app, this comes from wallet adapter (Phantom, Solflare, etc.)
  const wallet = Keypair.generate(); // Demo wallet
  console.log(`   Wallet: ${wallet.publicKey.toBase58()}\n`);

  // Your deployed program IDs (get these from deployment)
  const ROUTER_ID = new PublicKey('RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr');
  const SLAB_ID = new PublicKey('SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk');

  // Create SDK client - THIS IS THE MAGIC
  const client = new PercolatorClient(connection, ROUTER_ID, SLAB_ID);
  console.log('✅ SDK Client initialized\n');

  // ========================================
  // 2. GET MARKET DATA
  // ========================================
  
  console.log('📊 Getting market data...');
  
  // Find BTC-PERP slab address (SDK calculates this automatically)
  const btcSlab = client.getSlabAddress('BTC-PERP');
  console.log(`   BTC-PERP Slab: ${btcSlab.toBase58()}`);

  // Get orderbook (in real app, this shows bid/ask levels)
  const orderbook = await client.getOrderbook(btcSlab, 0);
  console.log(`   Orderbook: ${orderbook?.bids.length || 0} bids, ${orderbook?.asks.length || 0} asks\n`);

  // ========================================
  // 3. CHECK PORTFOLIO
  // ========================================
  
  console.log('💼 Checking portfolio...');
  
  const portfolio = await client.getPortfolio(wallet.publicKey);
  
  if (portfolio) {
    console.log(`   Equity: ${portfolio.equity.toString()}`);
    console.log(`   Free Collateral: ${portfolio.freeCollateral.toString()}`);
    console.log(`   Open Positions: ${portfolio.exposures.length}\n`);
  } else {
    console.log('   No portfolio yet (user not initialized)\n');
  }

  // ========================================
  // 4. PLACE A TRADE (The Important Part!)
  // ========================================
  
  console.log('🎯 Placing trade: BUY 0.1 BTC at $65,000');
  
  try {
    // Step 1: Reserve liquidity
    console.log('   Step 1/2: Reserving...');
    
    const reservation = await client.reserve(
      btcSlab,
      wallet,
      0,  // account index
      0,  // instrument index (BTC)
      Side.Buy,
      qtyToProtocol(0.1),  // Convert 0.1 to protocol format
      priceToProtocol(65000),  // Convert $65,000 to protocol format
      60000  // 60 second timeout
    );

    console.log(`   ✅ Reserved!`);
    console.log(`      Hold ID: ${reservation.holdId.toString()}`);
    console.log(`      VWAP: $${priceFromProtocol(reservation.vwapPx)}`);
    console.log(`      Filled: ${qtyFromProtocol(reservation.filledQty)} BTC`);

    // Step 2: Commit (execute the trade)
    console.log('\n   Step 2/2: Committing...');
    
    const result = await client.commit(
      btcSlab,
      wallet,
      reservation.holdId
    );

    console.log(`   ✅ Trade Executed!`);
    console.log(`      Filled: ${qtyFromProtocol(result.filledQty)} BTC`);
    console.log(`      Avg Price: $${priceFromProtocol(result.avgPrice)}`);
    console.log(`      Fee: ${result.totalFee.toString()}`);
    console.log(`      Fills: ${result.fillsCount}\n`);

  } catch (error) {
    console.error('❌ Trade failed:', error);
  }

  // ========================================
  // 5. CANCEL EXAMPLE
  // ========================================
  
  console.log('🚫 Canceling a reservation (if you change your mind):');
  
  try {
    // First reserve
    const res = await client.reserve(
      btcSlab, wallet, 0, 0, Side.Sell,
      qtyToProtocol(0.05),
      priceToProtocol(66000),
      60000
    );

    console.log(`   Reserved: ${res.holdId.toString()}`);

    // Then cancel (before committing)
    await client.cancel(btcSlab, wallet, res.holdId);
    
    console.log(`   ✅ Canceled successfully\n`);
  } catch (error) {
    console.error('   ❌ Cancel failed:', error);
  }

  // ========================================
  // 6. UTILITY FUNCTIONS
  // ========================================
  
  console.log('🔧 SDK Utility Functions:');
  console.log(`   Convert price: $65,000 → ${priceToProtocol(65000).toString()} (protocol format)`);
  console.log(`   Convert qty: 1.5 BTC → ${qtyToProtocol(1.5).toString()} (protocol format)`);
  console.log(`   Back to human: ${priceFromProtocol(priceToProtocol(65000))} (readable)`);
  console.log(`   Basis points: 500 bps → 5%`);
  console.log(`   SOL conversion: 1,000,000,000 lamports → 1.0 SOL\n`);

  console.log('✅ Demo complete!');
  console.log('\n📖 For frontend integration, see: sdk/typescript/README.md');
}

// Run demo
demo().catch(console.error);


