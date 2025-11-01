// Portfolio and AMM testing on devnet
// Tests portfolio creation, AMM pool initialization, and collateral deposits

const {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  LAMPORTS_PER_SOL,
  sendAndConfirmTransaction
} = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Load configuration from keypairs directory
const configPath = path.join(__dirname, '../../keypairs/config.json');
const devWalletPath = path.join(__dirname, '../../keypairs/devwallet.json');

const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
const devWalletKeypair = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(fs.readFileSync(devWalletPath, 'utf8')))
);

// Program IDs from deployed configuration
const ROUTER_PROGRAM_ID = new PublicKey(config.programs.router.program_id);
const AMM_PROGRAM_ID = new PublicKey(config.programs.amm.program_id);

// Devnet connection
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

console.log('='.repeat(60));
console.log('PORTFOLIO AND AMM TESTING');
console.log('='.repeat(60));
console.log('');

// Test 1: Create user portfolio
async function test1_createPortfolio() {
  console.log('TEST 1: Create User Portfolio');
  console.log('-'.repeat(60));
  
  try {
    // Portfolio uses createAccountWithSeed, not PDA
    const portfolioSeed = "portfolio";
    const portfolioAddress = await PublicKey.createWithSeed(
      devWalletKeypair.publicKey,
      portfolioSeed,
      ROUTER_PROGRAM_ID
    );
    
    console.log('Creating portfolio for:', devWalletKeypair.publicKey.toBase58());
    console.log('Portfolio Address:', portfolioAddress.toBase58());
    console.log('Seed:', portfolioSeed);
    console.log('');
    
    // Check if portfolio already exists
    const existingPortfolio = await connection.getAccountInfo(portfolioAddress);
    if (existingPortfolio) {
      console.log('Portfolio already exists');
      console.log('Owner:', existingPortfolio.owner.toBase58());
      console.log('Data length:', existingPortfolio.data.length, 'bytes');
      console.log('');
      return portfolioAddress;
    }
    
    // Portfolio size: 500KB for BPF alignment and large arrays
    // Contains: exposures (256 slabs * 32 instruments), funding offsets, LP buckets
    const PORTFOLIO_SIZE = 500000;
    const rentExemption = await connection.getMinimumBalanceForRentExemption(PORTFOLIO_SIZE);
    
    console.log('Rent required:', (rentExemption / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
    console.log('');
    
    // Check balance
    const balance = await connection.getBalance(devWalletKeypair.publicKey);
    console.log('Current balance:', (balance / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
    
    if (balance < rentExemption) {
      console.log('');
      console.log('ERROR: Insufficient balance');
      console.log('Need:', (rentExemption / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
      console.log('Have:', (balance / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
      console.log('Short by:', ((rentExemption - balance) / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
      console.log('');
      console.log('Request more SOL from faucet:');
      console.log('solana airdrop 5', devWalletKeypair.publicKey.toBase58(), '--url devnet');
      console.log('');
      return null;
    }
    console.log('');
    
    // Create account with seed
    const createPortfolioAccountIx = SystemProgram.createAccountWithSeed({
      fromPubkey: devWalletKeypair.publicKey,
      basePubkey: devWalletKeypair.publicKey,
      seed: portfolioSeed,
      newAccountPubkey: portfolioAddress,
      lamports: rentExemption,
      space: PORTFOLIO_SIZE,
      programId: ROUTER_PROGRAM_ID,
    });
    
    // Initialize portfolio instruction
    // Discriminator: 1 (InitializePortfolio)
    // Data: user pubkey (32 bytes)
    const instructionData = Buffer.alloc(33);
    instructionData.writeUInt8(1, 0);
    devWalletKeypair.publicKey.toBuffer().copy(instructionData, 1);
    
    const initPortfolioIx = new TransactionInstruction({
      programId: ROUTER_PROGRAM_ID,
      keys: [
        { pubkey: portfolioAddress, isSigner: false, isWritable: true },
        { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true },
      ],
      data: instructionData
    });
    
    console.log('Sending transaction...');
    const transaction = new Transaction().add(createPortfolioAccountIx, initPortfolioIx);
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWalletKeypair],
      { commitment: 'confirmed' }
    );
    
    console.log('');
    console.log('SUCCESS: Portfolio created');
    console.log('Portfolio Address:', portfolioAddress.toBase58());
    console.log('Transaction:', signature);
    console.log('Explorer: https://explorer.solana.com/tx/' + signature + '?cluster=devnet');
    console.log('');
    
    return portfolioAddress;
  } catch (error) {
    console.error('ERROR:', error.message);
    if (error.logs) {
      console.error('Program logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    return null;
  }
}

// Test 2: Create AMM liquidity pool
async function test2_createAMMPool() {
  console.log('TEST 2: Create AMM Liquidity Pool');
  console.log('-'.repeat(60));
  
  try {
    const ammPoolKeypair = Keypair.generate();
    console.log('Creating AMM pool:', ammPoolKeypair.publicKey.toBase58());
    console.log('Pool type: Constant Product (x*y=k)');
    console.log('Trading pair: SOL/USDC');
    console.log('');
    
    // AmmState size in BPF: 448 bytes (includes alignment padding)
    const AMM_POOL_SIZE = 448;
    const rentExemption = await connection.getMinimumBalanceForRentExemption(AMM_POOL_SIZE);
    console.log('Rent required:', (rentExemption / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
    console.log('');
    
    // Create account
    const createAccountIx = SystemProgram.createAccount({
      fromPubkey: devWalletKeypair.publicKey,
      newAccountPubkey: ammPoolKeypair.publicKey,
      lamports: rentExemption,
      space: AMM_POOL_SIZE,
      programId: AMM_PROGRAM_ID
    });
    
    // Initialize AMM pool instruction
    // Data layout (138 bytes total):
    //   discriminator: u8 (1 byte)
    //   lp_owner: Pubkey (32 bytes)
    //   router_id: Pubkey (32 bytes)
    //   instrument: Pubkey (32 bytes)
    //   mark_px: i64 (8 bytes)
    //   taker_fee_bps: i64 (8 bytes)
    //   contract_size: i64 (8 bytes)
    //   bump: u8 (1 byte)
    //   x_reserve: i64 (8 bytes)
    //   y_reserve: i64 (8 bytes)
    const instructionData = Buffer.alloc(138);
    let offset = 0;
    
    instructionData.writeUInt8(0, offset); offset += 1; // Initialize discriminator
    devWalletKeypair.publicKey.toBuffer().copy(instructionData, offset); offset += 32; // lp_owner
    ROUTER_PROGRAM_ID.toBuffer().copy(instructionData, offset); offset += 32; // router_id
    SystemProgram.programId.toBuffer().copy(instructionData, offset); offset += 32; // instrument
    instructionData.writeBigInt64LE(BigInt(150_000_000), offset); offset += 8; // mark_px ($150 scaled)
    instructionData.writeBigInt64LE(BigInt(30), offset); offset += 8; // fee_bps (0.3%)
    instructionData.writeBigInt64LE(BigInt(1_000_000), offset); offset += 8; // contract_size
    instructionData.writeUInt8(0, offset); offset += 1; // bump
    instructionData.writeBigInt64LE(BigInt(1000_000_000), offset); offset += 8; // x_reserve
    instructionData.writeBigInt64LE(BigInt(150_000_000_000), offset); offset += 8; // y_reserve
    
    const initializeIx = new TransactionInstruction({
      programId: AMM_PROGRAM_ID,
      keys: [
        { pubkey: ammPoolKeypair.publicKey, isSigner: false, isWritable: true },
        { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true }
      ],
      data: instructionData
    });
    
    console.log('Sending transaction...');
    const transaction = new Transaction().add(createAccountIx, initializeIx);
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWalletKeypair, ammPoolKeypair],
      { commitment: 'confirmed' }
    );
    
    console.log('');
    console.log('SUCCESS: AMM Pool created');
    console.log('Pool Address:', ammPoolKeypair.publicKey.toBase58());
    console.log('Transaction:', signature);
    console.log('Explorer: https://explorer.solana.com/tx/' + signature + '?cluster=devnet');
    console.log('');
    console.log('Pool configuration:');
    console.log('  Type: Constant Product (x*y=k)');
    console.log('  Initial Price: $150.00');
    console.log('  Swap Fee: 0.3%');
    console.log('  x_reserve: 1,000 contracts');
    console.log('  y_reserve: 150,000 quote');
    console.log('');
    
    return ammPoolKeypair.publicKey;
  } catch (error) {
    console.error('ERROR:', error.message);
    if (error.logs) {
      console.error('Program logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    return null;
  }
}

// Test 3: Deposit collateral to portfolio
async function test3_depositCollateral() {
  console.log('TEST 3: Deposit Collateral');
  console.log('-'.repeat(60));
  
  try {
    const portfolioSeed = "portfolio";
    const portfolioAddress = await PublicKey.createWithSeed(
      devWalletKeypair.publicKey,
      portfolioSeed,
      ROUTER_PROGRAM_ID
    );
    
    // Maximum deposit is 100M lamports (0.1 SOL) per transaction
    const depositAmount = 0.05 * LAMPORTS_PER_SOL;
    
    console.log('Depositing collateral...');
    console.log('From:', devWalletKeypair.publicKey.toBase58());
    console.log('To Portfolio:', portfolioAddress.toBase58());
    console.log('Amount:', depositAmount / LAMPORTS_PER_SOL, 'SOL');
    console.log('');
    
    // Deposit instruction
    // Discriminator: 2 (Deposit)
    // Data: amount (u64, 8 bytes)
    const instructionData = Buffer.alloc(9);
    instructionData.writeUInt8(2, 0);
    instructionData.writeBigUInt64LE(BigInt(depositAmount), 1);
    
    const depositIx = new TransactionInstruction({
      programId: ROUTER_PROGRAM_ID,
      keys: [
        { pubkey: portfolioAddress, isSigner: false, isWritable: true },
        { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
      ],
      data: instructionData
    });
    
    const transaction = new Transaction().add(depositIx);
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWalletKeypair],
      { commitment: 'confirmed' }
    );
    
    console.log('SUCCESS: Collateral deposited');
    console.log('Transaction:', signature);
    console.log('Amount:', depositAmount / LAMPORTS_PER_SOL, 'SOL');
    console.log('');
    
    return true;
  } catch (error) {
    console.error('ERROR:', error.message);
    if (error.logs) {
      console.error('Program logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    return false;
  }
}

// Main test runner
async function runTests() {
  console.log('Starting tests...');
  console.log('Network: Devnet');
  console.log('Payer:', devWalletKeypair.publicKey.toBase58());
  console.log('');
  
  const balance = await connection.getBalance(devWalletKeypair.publicKey);
  console.log('Current Balance:', (balance / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
  console.log('');
  console.log('='.repeat(60));
  console.log('');
  
  try {
    // Test 1: Create portfolio
    const portfolio = await test1_createPortfolio();
    
    if (!portfolio) {
      console.log('ERROR: Cannot proceed without portfolio');
      console.log('Please add more SOL to continue');
      return;
    }
    
    console.log('='.repeat(60));
    console.log('');
    
    // Test 2: Create AMM pool
    const ammPool = await test2_createAMMPool();
    
    if (ammPool) {
      console.log('='.repeat(60));
      console.log('');
      
      // Test 3: Deposit collateral
      await test3_depositCollateral();
    }
    
    console.log('='.repeat(60));
    console.log('');
    console.log('TESTS COMPLETED');
    console.log('');
    
    const finalBalance = await connection.getBalance(devWalletKeypair.publicKey);
    console.log('Final Balance:', (finalBalance / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
    console.log('');
    
  } catch (error) {
    console.error('FATAL ERROR:', error);
  }
}

// Execute tests
runTests().catch(console.error);

