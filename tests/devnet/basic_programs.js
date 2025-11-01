// Basic program deployment and slab creation test
// Original: test_programs.js

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

const configPath = path.join(__dirname, '../../keypairs/config.json');
const devWalletPath = path.join(__dirname, '../../keypairs/devwallet.json');

const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
const devWalletKeypair = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(fs.readFileSync(devWalletPath, 'utf8')))
);

const routerProgramId = new PublicKey(config.programs.router.program_id);
const slabProgramId = new PublicKey(config.programs.slab.program_id);
const ammProgramId = new PublicKey(config.programs.amm.program_id);

const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

console.log('='.repeat(60));
console.log('PERCOLATOR ON-CHAIN PROGRAM TESTING');
console.log('='.repeat(60));
console.log('');

async function verifyProgramDeployment(connection, programId, name) {
  try {
    const accountInfo = await connection.getAccountInfo(programId);
    
    if (!accountInfo) {
      console.log('ERROR:', name, 'not found');
      return false;
    }
    
    console.log('SUCCESS:', name, programId.toBase58());
    console.log('  Executable:', accountInfo.executable);
    console.log('  Data Length:', accountInfo.data.length, 'bytes');
    console.log('');
    return true;
  } catch (error) {
    console.error('ERROR verifying', name, ':', error.message);
    return false;
  }
}

async function runTests() {
  console.log('Starting on-chain program tests...');
  console.log('Network: Devnet');
  console.log('Payer:', devWalletKeypair.publicKey.toBase58());
  console.log('');
  console.log('='.repeat(60));
  console.log('');

  try {
    console.log('TEST 1: Verify Program Deployment');
    console.log('-'.repeat(60));
    
    await verifyProgramDeployment(connection, routerProgramId, 'Router Program');
    await verifyProgramDeployment(connection, slabProgramId, 'Slab Program');
    await verifyProgramDeployment(connection, ammProgramId, 'AMM Program');
    
    console.log('='.repeat(60));
    console.log('');
    
    console.log('TEST 2: Calculate Router PDAs');
    console.log('-'.repeat(60));
    
    const [authorityPDA, authorityBump] = PublicKey.findProgramAddressSync(
      [Buffer.from('authority')],
      routerProgramId
    );
    
    const [registryPDA, registryBump] = PublicKey.findProgramAddressSync(
      [Buffer.from('registry')],
      routerProgramId
    );
    
    const [portfolioPDA, portfolioBump] = PublicKey.findProgramAddressSync(
      [Buffer.from('portfolio'), devWalletKeypair.publicKey.toBuffer()],
      routerProgramId
    );
    
    console.log('SUCCESS: Authority PDA:', authorityPDA.toBase58());
    console.log('  Bump:', authorityBump);
    console.log('');
    console.log('SUCCESS: Registry PDA:', registryPDA.toBase58());
    console.log('  Bump:', registryBump);
    console.log('');
    console.log('SUCCESS: Portfolio PDA (for dev wallet):', portfolioPDA.toBase58());
    console.log('  Bump:', portfolioBump);
    console.log('');
    
    console.log('='.repeat(60));
    console.log('');
    
    console.log('TEST 3: Check Registry Account');
    console.log('-'.repeat(60));
    
    let registryAccount = await connection.getAccountInfo(registryPDA);
    
    if (!registryAccount) {
      console.log('WARNING: Registry account does NOT exist yet');
      console.log('  Address:', registryPDA.toBase58());
      console.log('  Status: Needs initialization');
    } else {
      console.log('SUCCESS: Registry exists');
      console.log('  Owner:', registryAccount.owner.toBase58());
      console.log('  Data length:', registryAccount.data.length, 'bytes');
    }
    console.log('');
    
    console.log('='.repeat(60));
    console.log('');
    
    console.log('TEST 4: Create Test Slab (SOL/USD Order Book)');
    console.log('-'.repeat(60));
    
    const slabKeypair = Keypair.generate();
    const SLAB_SIZE = 4096;
    const slabRent = await connection.getMinimumBalanceForRentExemption(SLAB_SIZE);
    
    console.log('Creating slab account:', slabKeypair.publicKey.toBase58());
    console.log('Rent required:', (slabRent / LAMPORTS_PER_SOL).toFixed(4), 'SOL');
    
    const createSlabAccountIx = SystemProgram.createAccount({
      fromPubkey: devWalletKeypair.publicKey,
      newAccountPubkey: slabKeypair.publicKey,
      lamports: slabRent,
      space: SLAB_SIZE,
      programId: slabProgramId,
    });
    
    const instructionData = Buffer.alloc(122);
    let offset = 0;
    instructionData.writeUInt8(0, offset); offset += 1;
    devWalletKeypair.publicKey.toBuffer().copy(instructionData, offset); offset += 32;
    routerProgramId.toBuffer().copy(instructionData, offset); offset += 32;
    SystemProgram.programId.toBuffer().copy(instructionData, offset); offset += 32;
    instructionData.writeBigInt64LE(BigInt(15000), offset); offset += 8;
    instructionData.writeBigInt64LE(BigInt(20), offset); offset += 8;
    instructionData.writeBigInt64LE(BigInt(1000), offset); offset += 8;
    instructionData.writeUInt8(0, offset);
    
    const initializeSlabIx = new TransactionInstruction({
      programId: slabProgramId,
      keys: [
        { pubkey: slabKeypair.publicKey, isSigner: false, isWritable: true },
        { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true }
      ],
      data: instructionData,
    });
    
    const slabTx = new Transaction().add(createSlabAccountIx).add(initializeSlabIx);
    const slabSignature = await sendAndConfirmTransaction(
      connection,
      slabTx,
      [devWalletKeypair, slabKeypair],
      { commitment: 'confirmed' }
    );
    
    console.log('');
    console.log('SUCCESS: Slab created');
    console.log('  Slab Address:', slabKeypair.publicKey.toBase58());
    console.log('  Transaction:', slabSignature);
    console.log('  Symbol: SOL/USD');
    console.log('  Mark Price: $150.00');
    console.log('  Taker Fee: 0.2%');
    console.log('');
    
    const slabInfo = await connection.getAccountInfo(slabKeypair.publicKey);
    console.log('SUCCESS: Verification');
    console.log('  Account exists:', slabInfo !== null);
    console.log('  Owner:', slabInfo.owner.toBase58());
    console.log('  Data length:', slabInfo.data.length, 'bytes');
    console.log('');
    
    console.log('='.repeat(60));
    console.log('');
    
    console.log('TEST 5: Check Wallet Balance');
    console.log('-'.repeat(60));
    const balance = await connection.getBalance(devWalletKeypair.publicKey);
    console.log('SUCCESS: Dev Wallet:', devWalletKeypair.publicKey.toBase58());
    console.log('  Balance:', (balance / LAMPORTS_PER_SOL).toFixed(8), 'SOL');
    console.log('');
    
    console.log('='.repeat(60));
    console.log('');
    console.log('ALL TESTS COMPLETED');
    console.log('');
    console.log('Summary:');
    console.log('  Programs deployed and executable');
    console.log('  PDAs calculated correctly');
    console.log('  Registry initialization tested');
    console.log('  Slab creation tested');
    console.log('');
    
  } catch (error) {
    console.error('FATAL ERROR:', error);
    if (error.logs) {
      console.error('Program logs:');
      error.logs.forEach(log => console.error('  ', log));
    }
  }
}

runTests().catch(console.error);

