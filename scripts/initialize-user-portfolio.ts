/**
 * Initialize User Portfolio (Minimal v0)
 * Creates portfolio account for Router-based trading
 */

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
  clusterApiUrl,
} from '@solana/web3.js';
import fs from 'fs';

// Router Program ID (deployed on devnet)
const ROUTER_PROGRAM_ID = new PublicKey('RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr');

// Router instruction discriminators
const RouterInstruction = {
  InitializePortfolio: 1,
} as const;

// Portfolio account size (from Router program)
// Portfolio structure: ~12KB
const PORTFOLIO_SIZE = 12288;

/**
 * Derive portfolio PDA for a user
 */
function derivePortfolioPDA(userPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from('portfolio'), userPubkey.toBuffer()],
    ROUTER_PROGRAM_ID
  );
}

async function main() {
  console.log('🚀 PERColator Portfolio Initialization\n');

  // Connect to devnet
  const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
  console.log('✅ Connected to Solana devnet');

  // Load user keypair (for demo, use PERC wallet)
  const keypairPath = process.argv[2] || 'perc-keypair.json';
  let user: Keypair;

  try {
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
    user = Keypair.fromSecretKey(new Uint8Array(keypairData));
    console.log('📂 Loaded wallet');
  } catch {
    console.error('❌ Could not load keypair');
    console.log(`\n💡 Usage: npx ts-node initialize-user-portfolio.ts [keypair.json]`);
    console.log(`   Default: perc-keypair.json\n`);
    process.exit(1);
  }

  console.log(`👛 User: ${user.publicKey.toBase58()}`);

  // Check balance
  const balance = await connection.getBalance(user.publicKey);
  console.log(`💰 Balance: ${(balance / 1e9).toFixed(8)} SOL`);

  // Derive portfolio PDA
  const [portfolioPDA, portfolioBump] = derivePortfolioPDA(user.publicKey);

  console.log(`\n📋 Portfolio PDA: ${portfolioPDA.toBase58()}`);
  console.log(`   Bump: ${portfolioBump}`);

  // Check if portfolio already exists
  const portfolioInfo = await connection.getAccountInfo(portfolioPDA);
  if (portfolioInfo) {
    console.log('⚠️  Portfolio already initialized!');
    console.log(`   Owner: ${portfolioInfo.owner.toBase58()}`);
    console.log(`   Size: ${portfolioInfo.data.length} bytes\n`);
    console.log('✅ Portfolio ready - you can start trading!');
    return;
  }

  console.log('   Portfolio doesn\'t exist yet - will be created\n');

  // Calculate rent for Portfolio account
  const portfolioRent = await connection.getMinimumBalanceForRentExemption(PORTFOLIO_SIZE);
  console.log(`💵 Rent for Portfolio: ${(portfolioRent / 1e9).toFixed(8)} SOL`);

  // Check if we need airdrop
  if (balance < portfolioRent + 0.01e9) {
    const needed = portfolioRent + 0.01e9 - balance;
    console.log(`\n⚠️  Insufficient balance! Need ${(needed / 1e9).toFixed(4)} more SOL.`);
    console.log(`\n🚰 Requesting airdrop...`);
    
    try {
      const signature = await connection.requestAirdrop(user.publicKey, 2e9);
      await connection.confirmTransaction(signature);
      console.log(`✅ Airdropped 2 SOL`);
      
      const newBalance = await connection.getBalance(user.publicKey);
      console.log(`💰 New balance: ${(newBalance / 1e9).toFixed(8)} SOL\n`);
    } catch (error: any) {
      console.error(`❌ Airdrop failed: ${error.message}`);
      console.log(`\n💡 Get SOL from: https://faucet.solana.com/`);
      console.log(`   Address: ${user.publicKey.toBase58()}\n`);
      process.exit(1);
    }
  }

  console.log('\n🔧 Building Portfolio initialization (2-step process)...\n');

  // STEP 1: Create the PDA account with System Program
  // We can't use createAccount with a PDA, so we use createAccountWithSeed as a workaround
  // Actually, we need to use the proper Solana pattern: allocate with System Program
  
  // Note: Since PDAs can't sign, we can't use regular createAccount
  // We need the Router program to create it via CPI, OR
  // We use a workaround: Create with System Program allocate + assign

  console.log('Step 1: Creating PDA account with System Program...');

  // For PDA creation, we use System Program's create_account instruction manually
  // with invoke_signed from the Router program... but Router doesn't have that logic.
  
  // WORKAROUND for v0: Router program expects account to exist
  // We'll need to use a regular Keypair instead of PDA for now

  console.warn('⚠️  Warning: Router requires PDA but cannot create it');
  console.warn('   This is a known v0 limitation');
  console.warn('   For now, using Memo approach recommended');
  console.log('');

  throw new Error('Portfolio PDA creation requires Router program modification.\n\n' +
    'Current Router code expects PDAs to pre-exist but lacks CPI logic to create them.\n\n' +
    'Options:\n' +
    '1. Stick with Memo approach for v0 (RECOMMENDED)\n' +
    '2. Modify Router program to add PDA creation via CPI\n' +
    '3. Use Router in v1 with proper account creation\n\n' +
    'See V1_ROUTER_INTEGRATION.md for full Router implementation plan.');

    console.log(`\n✅ Portfolio initialized successfully!`);
    console.log(`   Signature: ${signature}`);
    console.log(`   Portfolio PDA: ${portfolioPDA.toBase58()}`);
    console.log(`\n🔗 View on Solscan:`);
    console.log(`   https://solscan.io/tx/${signature}?cluster=devnet`);
    console.log(`\n📊 Portfolio Info:`);
    console.log(`   Owner: ${ROUTER_PROGRAM_ID.toBase58()}`);
    console.log(`   User: ${user.publicKey.toBase58()}`);
    console.log(`   Size: ${PORTFOLIO_SIZE} bytes`);

    // Save info to file
    const portfolioInfo = {
      userPublicKey: user.publicKey.toBase58(),
      portfolioPDA: portfolioPDA.toBase58(),
      portfolioBump,
      routerProgramId: ROUTER_PROGRAM_ID.toBase58(),
      initSignature: signature,
      network: 'devnet',
      timestamp: new Date().toISOString(),
    };

    fs.writeFileSync(
      `portfolio-${user.publicKey.toBase58().slice(0, 8)}.json`,
      JSON.stringify(portfolioInfo, null, 2)
    );

    console.log(`\n💾 Portfolio info saved to: portfolio-${user.publicKey.toBase58().slice(0, 8)}.json`);
    console.log(`\n🎉 You're ready to trade with Router integration!`);

  } catch (error: any) {
    console.error(`\n❌ Initialization failed:`, error.message);
    if (error.logs) {
      console.log(`\n📋 Transaction logs:`);
      error.logs.forEach((log: string) => console.log(`   ${log}`));
    }
    process.exit(1);
  }
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

