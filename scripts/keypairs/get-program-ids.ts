#!/usr/bin/env ts-node
/**
 * Extract program IDs from keypairs
 */

import * as fs from 'fs';
import * as path from 'path';
import { Keypair } from '@solana/web3.js';

const keypairsDir = __dirname;

interface ProgramInfo Russell
  name: string;
  publicKey: string;
  file: string;
}

const programs = [
  { name: 'AMM', file: 'program-amm.json' },
  { name: 'Router', file: 'program-router.json' },
  { name: 'Slab', file: 'program-slab.json' },
  { name: 'Oracle', file: 'program-oracle.json' },
];

async function main() {
  console.log('📋 Program IDs Summary\n');
  
  const programIds: Record<string, string> = {};
  const summary: ProgramInfo[] = [];

  for (const program of programs) {
    const keypairPath = path.join(keypairsDir, program.file);
    
    if (!fs.existsSync(keypairPath)) {
      console.log(`⚠️  ${program.name}: File not found (${program.file})`);
      continue;
    }

    try {
      const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
      const keypair = Keypair.fromSecretKey(new Uint8Array(keypairData));
      const pubkey = keypair.publicKey.toBase58();
      
      programIds[program.name.toLowerCase()] = pubkey;
      summary.push({
        name: program.name,
        publicKey: pubkey,
        file: program.file,
      });
      
      console.log(`${program.name.padEnd(10)}: ${pubkey}`);
    } catch (error) {
      console.error(`❌ ${program.name}: Failed to read keypair - ${error}`);
    }
  }

  // Write summary JSON
  const summaryPath = path.join(keypairsDir, 'program-ids.json');
  fs.writeFileSync(summaryPath, JSON.stringify({ programIds, summary }, null, 2));
  console.log(`\n✅ Summary saved to: ${summaryPath}`);
}

main().catch(console.error);

