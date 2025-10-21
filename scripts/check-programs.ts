#!/usr/bin/env ts-node
/**
 * Check if programs are deployed on devnet
 */

import { Connection, PublicKey } from '@solana/web3.js';

const SLAB_PROGRAM_ID = new PublicKey('6EF2acRfPejnxXYd9apKc2wb3p2NLG8rKgWbCfp5G7Uz');
const ROUTER_PROGRAM_ID = new PublicKey('9CQWTSDobkHqWzvx4nufdke4C8GKuoaqiNBBLEYFoHoG');

async function main() {
  console.log('🔍 Checking programs on Solana devnet...\n');
  
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  try {
    // Check Slab program
    const slabInfo = await connection.getAccountInfo(SLAB_PROGRAM_ID);
    if (slabInfo) {
      console.log('✅ Slab Program Found!');
      console.log(`   ID: ${SLAB_PROGRAM_ID.toBase58()}`);
      console.log(`   Executable: ${slabInfo.executable}`);
      console.log(`   Owner: ${slabInfo.owner.toBase58()}`);
      console.log(`   Size: ${(slabInfo.data.length / 1024).toFixed(2)} KB`);
    } else {
      console.log('❌ Slab Program NOT found on devnet');
    }
    
    console.log('');
    
    // Check Router program
    const routerInfo = await connection.getAccountInfo(ROUTER_PROGRAM_ID);
    if (routerInfo) {
      console.log('✅ Router Program Found!');
      console.log(`   ID: ${ROUTER_PROGRAM_ID.toBase58()}`);
      console.log(`   Executable: ${routerInfo.executable}`);
      console.log(`   Owner: ${routerInfo.owner.toBase58()}`);
      console.log(`   Size: ${(routerInfo.data.length / 1024).toFixed(2)} KB`);
    } else {
      console.log('❌ Router Program NOT found on devnet');
    }
    
    if (slabInfo && routerInfo) {
      console.log('\n🎉 Both programs are deployed!');
      console.log('\n📝 Next step: Initialize slab account');
    }
    
  } catch (error: any) {
    console.error('❌ Error checking programs:', error.message);
  }
}

main();

