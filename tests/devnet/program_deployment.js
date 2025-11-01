// Test deployed Percolator programs on devnet
// Verifies program deployment, PDA calculation, registry, and slab creation

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

// Load configuration
const configPath = path.join(__dirname, '../../keypairs/config.json');
const devWalletPath = path.join(__dirname, '../../keypairs/devwallet.json');

const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
const devWalletKeypair = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(fs.readFileSync(devWalletPath, 'utf8')))
);

// Program IDs
const ROUTER_PROGRAM_ID = new PublicKey(config.programs.router.program_id);
const SLAB_PROGRAM_ID = new PublicKey(config.programs.slab.program_id);
const AMM_PROGRAM_ID = new PublicKey(config.programs.amm.program_id);

// Connection
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

console.log('='.repeat(60));
console.log('PERCOLATOR PROGRAM DEPLOYMENT VERIFICATION');
console.log('='.repeat(60));
console.log('');

// Test 1: Verify program deployment
async function verifyProgramDeployment(pubkey, name) {
  try {
    const accountInfo = await connection.getAccountInfo(pubkey);
    
    if (!accountInfo) {
      console.log('ERROR:', name, 'not found at', pubkey.toBase58());
      return false;
    }
    
    console.log('SUCCESS:', name);
    console.log('  Address:', pubkey.toBase58());
    console.log('  Executable:', accountInfo.executable);
    console.log('  Owner:', accountInfo.owner.toBase58());
    console.log('  Data Length:', accountInfo.data.length, 'bytes');
    console.log('');
    
    return true;
  } catch (error) {
    console.error('ERROR verifying', name, ':', error.message);
    return false;
  }
}

// Test 2: Calculate and display PDAs
function calculatePDAs() {
  console.log('TEST: Calculate Router PDAs');
  console.log('-'.repeat(60));
  
  // Authority PDA: manages program permissions
  const [authorityPDA, authorityBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('authority')],
    ROUTER_PROGRAM_ID
  );
  
  console.log('Authority PDA:', authorityPDA.toBase58());
  console.log('  Bump:', authorityBump);
  console.log('');
  
  // Registry PDA: stores global exchange configuration
  const [registryPDA, registryBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('registry')],
    ROUTER_PROGRAM_ID
  );
  
  console.log('Registry PDA:', registryPDA.toBase58());
  console.log('  Bump:', registryBump);
  console.log('');
  
  // Portfolio PDA: user-specific cross-margin account
  const [portfolioPDA, portfolioBump] = PublicKey.findProgramAddressSync(
    [Buffer.from('portfolio'), devWalletKeypair.publicKey.toBuffer()],
    ROUTER_PROGRAM_ID
  );
  
  console.log('Portfolio PDA (for dev wallet):', portfolioPDA.toBase58());
  console.log('  Bump:', portfolioBump);
  console.log('  User:', devWalletKeypair.publicKey.toBase58());
  console.log('');
  
  return { authorityPDA, registryPDA, portfolioPDA };
}

// Test 3: Check registry account status
async function checkRegistry(registryPDA) {
  console.log('TEST: Check Registry Status');
  console.log('-'.repeat(60));
  
  const registryAccount = await connection.getAccountInfo(registryPDA);
  
  if (!registryAccount) {
    console.log('Registry does not exist yet');
    console.log('  Address:', registryPDA.toBase58());
    console.log('  Status: Needs initialization');
    console.log('');
    return false;
  }
  
  console.log('Registry exists');
  console.log('  Address:', registryPDA.toBase58());
  console.log('  Owner:', registryAccount.owner.toBase58());
  console.log('  Data length:', registryAccount.data.length, 'bytes');
  console.log('');
  
  return true;
}

// Test 4: Create test slab (order book)
async function createTestSlab() {
  console.log('TEST: Create Test Slab (Order Book)');
  console.log('-'.repeat(60));
  
  try {
    const slabKeypair = Keypair.generate();
    console.log('Creating slab:', slabKeypair.publicKey.toBase58());
    console.log('Symbol: SOL/USD');
    console.log('Type: Perpetual futures order book');
    console.log('');
    
    // Standard slab size for testing
    const SLAB_SIZE = 4096;
    const rentExemption = await connection.getMinimumBalanceForRentExemption(SLAB_SIZE);
    console.log('Rent required:', (rentExemption / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
    console.log('');
    
    // Create account
    const createAccountIx = SystemProgram.createAccount({
      fromPubkey: devWalletKeypair.publicKey,
      newAccountPubkey: slabKeypair.publicKey,
      lamports: rentExemption,
      space: SLAB_SIZE,
      programId: SLAB_PROGRAM_ID
    });
    
    // Initialize slab instruction
    // Data layout: discriminator + lp_owner + router_id + instrument + mark_px + taker_fee_bps + contract_size + bump
    const instructionData = Buffer.alloc(122);
    let offset = 0;
    
    instructionData.writeUInt8(0, offset); offset += 1; // discriminator
    devWalletKeypair.publicKey.toBuffer().copy(instructionData, offset); offset += 32; // lp_owner
    ROUTER_PROGRAM_ID.toBuffer().copy(instructionData, offset); offset += 32; // router_id
    SystemProgram.programId.toBuffer().copy(instructionData, offset); offset += 32; // instrument
    instructionData.writeBigInt64LE(BigInt(15000), offset); offset += 8; // mark_px ($150)
    instructionData.writeBigInt64LE(BigInt(20), offset); offset += 8; // taker_fee_bps (0.2%)
    instructionData.writeBigInt64LE(BigInt(1000), offset); offset += 8; // contract_size
    instructionData.writeUInt8(0, offset); // bump
    
    const initializeIx = new TransactionInstruction({
      programId: SLAB_PROGRAM_ID,
      keys: [
        { pubkey: slabKeypair.publicKey, isSigner: false, isWritable: true },
        { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true }
      ],
      data: instructionData
    });
    
    console.log('Sending transaction...');
    const transaction = new Transaction().add(createAccountIx, initializeIx);
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWalletKeypair, slabKeypair],
      { commitment: 'confirmed' }
    );
    
    console.log('');
    console.log('SUCCESS: Slab created');
    console.log('  Slab Address:', slabKeypair.publicKey.toBase58());
    console.log('  Transaction:', signature);
    console.log('  Symbol: SOL/USD');
    console.log('  Mark Price: $150.00');
    console.log('  Taker Fee: 0.2%');
    console.log('');
    
    // Verify
    const slabInfo = await connection.getAccountInfo(slabKeypair.publicKey);
    console.log('Verification:');
    console.log('  Account exists:', slabInfo !== null);
    console.log('  Owner:', slabInfo.owner.toBase58());
    console.log('  Data length:', slabInfo.data.length, 'bytes');
    console.log('');
    
    return slabKeypair.publicKey;
  } catch (error) {
    console.error('ERROR:', error.message);
    if (error.logs) {
      console.error('Program logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
    return null;
  }
}

// Main test runner
async function runTests() {
  console.log('Starting deployment verification...');
  console.log('Network: Devnet');
  console.log('Payer:', devWalletKeypair.publicKey.toBase58());
  console.log('');
  
  const balance = await connection.getBalance(devWalletKeypair.publicKey);
  console.log('Balance:', (balance / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
  console.log('');
  console.log('='.repeat(60));
  console.log('');
  
  try {
    // Test 1: Verify program deployment
    console.log('TEST 1: Verify Program Deployment');
    console.log('-'.repeat(60));
    
    await verifyProgramDeployment(ROUTER_PROGRAM_ID, 'Router Program');
    await verifyProgramDeployment(SLAB_PROGRAM_ID, 'Slab Program');
    await verifyProgramDeployment(AMM_PROGRAM_ID, 'AMM Program');
    
    console.log('='.repeat(60));
    console.log('');
    
    // Test 2: Calculate PDAs
    const pdas = calculatePDAs();
    
    console.log('='.repeat(60));
    console.log('');
    
    // Test 3: Check registry
    await checkRegistry(pdas.registryPDA);
    
    console.log('='.repeat(60));
    console.log('');
    
    // Test 4: Create test slab
    await createTestSlab();
    
    console.log('='.repeat(60));
    console.log('');
    console.log('ALL TESTS COMPLETED');
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

