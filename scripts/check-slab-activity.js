#!/usr/bin/env node

const { Connection, PublicKey } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Load config
const configPath = path.join(__dirname, '..', 'keypairs', 'config.json');
const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));

const SLAB_ACCOUNT = new PublicKey(config.trading_pairs['SOL-USD'].slab_address);
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

async function checkActivity() {
  console.log(`\n📊 Checking Slab Activity`);
  console.log(`   Slab: ${SLAB_ACCOUNT.toBase58()}\n`);
  
  try {
    // Get recent signatures
    const signatures = await connection.getSignaturesForAddress(SLAB_ACCOUNT, { limit: 10 });
    
    if (signatures.length === 0) {
      console.log('❌ No transactions found for this slab');
      return;
    }
    
    console.log(`✅ Found ${signatures.length} recent transactions:\n`);
    
    for (let i = 0; i < Math.min(5, signatures.length); i++) {
      const sig = signatures[i];
      const time = new Date(sig.blockTime * 1000);
      const status = sig.err ? '❌ Failed' : '✅ Success';
      
      console.log(`${i + 1}. ${status}`);
      console.log(`   Sig: ${sig.signature.slice(0, 20)}...`);
      console.log(`   Time: ${time.toLocaleString()}`);
      if (sig.err) {
        console.log(`   Error: ${JSON.stringify(sig.err)}`);
      }
      console.log('');
    }
    
    // Check for recent successful transactions
    const successCount = signatures.filter(s => !s.err).length;
    const recentSuccess = signatures.filter(s => !s.err && (Date.now() - s.blockTime * 1000) < 60000).length;
    
    console.log(`📈 Summary:`);
    console.log(`   Total success: ${successCount}/${signatures.length}`);
    console.log(`   Recent (last 60s): ${recentSuccess}`);
    
    if (recentSuccess > 0) {
      console.log(`\n✅ Slab has REAL on-chain liquidity!`);
    } else {
      console.log(`\n⚠️  No recent successful transactions`);
    }
    
  } catch (error) {
    console.error('❌ Error:', error.message);
  }
}

checkActivity();


