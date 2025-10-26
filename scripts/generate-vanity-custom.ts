#!/usr/bin/env ts-node
/**
 * Generate vanity Solana addresses with prefix and suffix matching
 * Supports leetspeak alternatives
 * 
 * Usage:
 *   npx ts-node scripts/generate-vanity-custom.ts --prefix hai --suffix der
 */

import { Keypair } from '@solana/web3.js';
import * as fs from 'fs';

// Parse command line arguments
const args = process.argv.slice(2);
let prefix = '';
let suffix = '';
let outputFile = '';

for (let i = 0; i < args.length; i++) {
  if (args[i] === '--prefix' && i + 1 < args.length) {
    prefix = args[i + 1];
    i++;
  } else if (args[i] === '--suffix' && i + 1 < args.length) {
    suffix = args[i + 1];
    i++;
  } else if (args[i] === '--output' && i + 1 < args.length) {
    outputFile = args[i + 1];
    i++;
  }
}

if (!prefix && !suffix) {
  console.error('Usage: npx ts-node scripts/generate-vanity-custom.ts --prefix <PREFIX> --suffix <SUFFIX> [--output <FILE>]');
  console.error('Example: npx ts-node scripts/generate-vanity-custom.ts --prefix hai --suffix der --output wallet.json');
  console.error('\nSupports leetspeak: h=h, a=a/4, i=i/1, d=d, e=e/3, r=r');
  process.exit(1);
}

// Create regex patterns for flexible matching
const createFlexiblePattern = (text: string, isEnd: boolean = false): RegExp => {
  // Leetspeak mappings (case-insensitive for Base58 compatibility)
  const leet: { [key: string]: string } = {
    'a': '[aA4]',
    'e': '[eE3]',
    'i': '[iI1]',
    'o': '[oO0]',
    's': '[sS5]',
    't': '[tT7]',
    'h': '[hH]',
    'd': '[dD]',
    'r': '[rR]',
    'l': '[lL1]',
  };

  let pattern = '';
  for (const char of text.toLowerCase()) {
    pattern += leet[char] || char;
  }

  if (isEnd) {
    return new RegExp(pattern + '$', 'i');
  } else {
    return new RegExp('^' + pattern, 'i');
  }
};

const prefixPattern = prefix ? createFlexiblePattern(prefix, false) : null;
const suffixPattern = suffix ? createFlexiblePattern(suffix, true) : null;

console.log('🎯 Generating vanity address:');
if (prefix) console.log(`   Starting with: ${prefix} (flexible matching)`);
if (suffix) console.log(`   Ending with: ${suffix} (flexible matching)`);
console.log(`\n⏱️  Estimated time: ${estimateTime(prefix.length, suffix.length)}`);
console.log(`🔑 Searching... (Press Ctrl+C to stop)\n`);

let attempts = 0;
const startTime = Date.now();

// Progress update every 10k attempts
const progressInterval = setInterval(() => {
  const elapsed = (Date.now() - startTime) / 1000;
  const rate = attempts / elapsed;
  console.log(`   Checked ${attempts.toLocaleString()} addresses (${rate.toFixed(0)}/sec)`);
}, 10000);

while (true) {
  attempts++;
  
  const keypair = Keypair.generate();
  const pubkey = keypair.publicKey.toBase58();
  
  // Check if matches prefix and suffix
  const matchesPrefix = !prefixPattern || prefixPattern.test(pubkey);
  const matchesSuffix = !suffixPattern || suffixPattern.test(pubkey);
  
  if (matchesPrefix && matchesSuffix) {
    clearInterval(progressInterval);
    
    const elapsed = (Date.now() - startTime) / 1000;
    
    console.log(`\n✅ FOUND after ${attempts.toLocaleString()} attempts in ${elapsed.toFixed(1)}s!`);
    console.log(`\n🔑 Public Key:  ${pubkey}`);
    console.log(`🔐 Secret Key:  [${keypair.secretKey.slice(0, 4).join(', ')}...] (${keypair.secretKey.length} bytes)`);
    
    // Save to file
    const fileName = outputFile || `${prefix}${suffix ? '-' + suffix : ''}-keypair.json`;
    fs.writeFileSync(fileName, JSON.stringify(Array.from(keypair.secretKey)));
    console.log(`\n💾 Saved to: ${fileName} (Solana CLI format)`);
    
    // Also save detailed version
    const detailedFile = fileName.replace('.json', '-detailed.json');
    const keypairData = {
      publicKey: pubkey,
      secretKey: Array.from(keypair.secretKey)
    };
    fs.writeFileSync(detailedFile, JSON.stringify(keypairData, null, 2));
    console.log(`💾 Detailed info saved to: ${detailedFile}`);
    
    console.log(`\n📋 To use with Solana CLI:`);
    console.log(`   solana-keygen pubkey ${fileName}`);
    console.log(`   solana balance ${pubkey}`);
    
    process.exit(0);
  }
  
  // Show progress every 50k attempts
  if (attempts % 50000 === 0) {
    const elapsed = (Date.now() - startTime) / 1000;
    const rate = attempts / elapsed;
    process.stdout.write(`\r   ${attempts.toLocaleString()} checked (${rate.toFixed(0)}/sec)...`);
  }
}

function estimateTime(prefixLength: number, suffixLength: number): string {
  // Base58 has 58 characters
  // For combined prefix + suffix, multiply the probabilities
  const prefixPossibilities = prefixLength > 0 ? Math.pow(58, prefixLength) : 1;
  const suffixPossibilities = suffixLength > 0 ? Math.pow(58, suffixLength) : 1;
  const totalPossibilities = prefixPossibilities * suffixPossibilities;
  
  const attemptsPerSecond = 50000; // Conservative estimate
  const seconds = totalPossibilities / attemptsPerSecond / 2; // On average, find in half the space
  
  if (seconds < 60) return `${Math.round(seconds)} seconds`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} minutes`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)} hours`;
  if (seconds < 604800) return `${Math.round(seconds / 86400)} days`;
  if (seconds < 2592000) return `${Math.round(seconds / 604800)} weeks`;
  return `${Math.round(seconds / 2592000)} months (may take a while!)`;
}

