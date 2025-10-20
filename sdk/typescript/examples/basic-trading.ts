/**
 * Basic Trading Example
 * 
 * Demonstrates the complete flow:
 * 1. Connect to devnet
 * 2. Deposit collateral
 * 3. Reserve liquidity
 * 4. Commit execution
 * 5. Check portfolio
 */

import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { PercolatorClient, Side, priceToProtocol, qtyToProtocol, priceFromProtocol } from '../src';
import BN from 'bn.js';

async function main() {
  // 1. Setup
  console.log('🔧 Setting up connection to devnet...');
  
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  // Load your wallet (replace with your secret key)
  const wallet = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(process.env.WALLET_SECRET_KEY || '[]'))
  );
  
  console.log(`Wallet: ${wallet.publicKey.toBase58()}`);

  // Program IDs (replace with your deployed programs)
  const ROUTER_PROGRAM_ID = new PublicKey(process.env.ROUTER_PROGRAM_ID || 'RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr');
  const SLAB_PROGRAM_ID = new PublicKey(process.env.SLAB_PROGRAM_ID || 'SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk');

  // 2. Initialize client
  const client = new PercolatorClient(
    connection,
    ROUTER_PROGRAM_ID,
    SLAB_PROGRAM_ID,
    wallet
  );

  console.log('✅ Client initialized');

  // 3. Get slab address for BTC-PERP
  const btcSlab = client.getSlabAddress('BTC-PERP');
  console.log(`BTC-PERP Slab: ${btcSlab.toBase58()}`);

  // 4. Check portfolio
  console.log('\n📊 Checking portfolio...');
  const portfolio = await client.getPortfolio(wallet.publicKey);
  
  if (portfolio) {
    console.log(`Equity: ${portfolio.equity.toString()}`);
    console.log(`Free Collateral: ${portfolio.freeCollateral.toString()}`);
    console.log(`Positions: ${portfolio.exposures.length}`);
  } else {
    console.log('No portfolio found (not initialized)');
  }

  // 5. Reserve to buy 0.1 BTC at $65,000
  console.log('\n🔒 Reserving liquidity...');
  
  const reserveResult = await client.reserve(
    btcSlab,
    wallet,
    0,  // account index
    0,  // instrument index (BTC)
    Side.Buy,
    qtyToProtocol(0.1),  // 0.1 BTC
    priceToProtocol(65000),  // $65,000 limit
    60000  // 60 second TTL
  );

  console.log(`✅ Reserved: holdId=${reserveResult.holdId.toString()}`);
  console.log(`   VWAP: $${priceFromProtocol(reserveResult.vwapPx)}`);
  console.log(`   Filled: ${reserveResult.filledQty.toString()}`);
  console.log(`   Max Charge: ${reserveResult.maxCharge.toString()}`);

  // 6. Commit the reservation
  console.log('\n✅ Committing...');
  
  const commitResult = await client.commit(
    btcSlab,
    wallet,
    reserveResult.holdId
  );

  console.log(`✅ Committed:`);
  console.log(`   Filled Qty: ${commitResult.filledQty.toString()}`);
  console.log(`   Avg Price: $${priceFromProtocol(commitResult.avgPrice)}`);
  console.log(`   Total Fee: ${commitResult.totalFee.toString()}`);
  console.log(`   Fills: ${commitResult.fillsCount}`);

  // 7. Check updated portfolio
  console.log('\n📊 Updated portfolio:');
  const updatedPortfolio = await client.getPortfolio(wallet.publicKey);
  
  if (updatedPortfolio) {
    console.log(`Equity: ${updatedPortfolio.equity.toString()}`);
    console.log(`Positions: ${updatedPortfolio.exposures.length}`);
  }

  console.log('\n✅ Done!');
}

main().catch(console.error);

