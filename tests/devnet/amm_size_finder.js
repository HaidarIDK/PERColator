// Find correct AMM pool size through testing
// Original: test_amm_sizes.js

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

const ROUTER_PROGRAM_ID = new PublicKey(config.programs.router.program_id);
const AMM_PROGRAM_ID = new PublicKey(config.programs.amm.program_id);
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

async function testSize(size) {
  const ammPoolKeypair = Keypair.generate();
  const rentExemption = await connection.getMinimumBalanceForRentExemption(size);
  
  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: devWalletKeypair.publicKey,
    newAccountPubkey: ammPoolKeypair.publicKey,
    lamports: rentExemption,
    space: size,
    programId: AMM_PROGRAM_ID
  });
  
  const instructionData = Buffer.alloc(138);
  let offset = 0;
  instructionData.writeUInt8(0, offset); offset += 1;
  devWalletKeypair.publicKey.toBuffer().copy(instructionData, offset); offset += 32;
  ROUTER_PROGRAM_ID.toBuffer().copy(instructionData, offset); offset += 32;
  SystemProgram.programId.toBuffer().copy(instructionData, offset); offset += 32;
  instructionData.writeBigInt64LE(BigInt(150_000_000), offset); offset += 8;
  instructionData.writeBigInt64LE(BigInt(30), offset); offset += 8;
  instructionData.writeBigInt64LE(BigInt(1_000_000), offset); offset += 8;
  instructionData.writeUInt8(0, offset); offset += 1;
  instructionData.writeBigInt64LE(BigInt(1000_000_000), offset); offset += 8;
  instructionData.writeBigInt64LE(BigInt(150_000_000_000), offset); offset += 8;
  
  const initializeIx = new TransactionInstruction({
    programId: AMM_PROGRAM_ID,
    keys: [
      { pubkey: ammPoolKeypair.publicKey, isSigner: false, isWritable: true },
      { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true }
    ],
    data: instructionData
  });
  
  try {
    const transaction = new Transaction().add(createAccountIx, initializeIx);
    const { value: { err } } = await connection.simulateTransaction(transaction, [devWalletKeypair, ammPoolKeypair]);
    
    if (!err) {
      console.log('SUCCESS: SIZE', size, 'WORKS');
      return true;
    } else {
      const errStr = JSON.stringify(err);
      if (errStr.includes('incorrect size')) {
        console.log('ERROR: Size', size, '- incorrect size');
      } else {
        console.log('WARNING: Size', size, '-', errStr.substring(0, 80));
      }
    }
  } catch (error) {
    console.log('ERROR: Size', size, '-', error.message.substring(0, 60));
  }
  return false;
}

async function findCorrectSize() {
  console.log('Testing AMM pool sizes...');
  console.log('');
  
  // Test common BPF-aligned sizes around 400
  const sizesToTest = [
    400, 408, 416, 424, 432, 440, 448, 456, 464, 472, 480,
    488, 496, 504, 512, 520, 528, 536, 544
  ];
  
  for (const size of sizesToTest) {
    const works = await testSize(size);
    if (works) {
      console.log('');
      console.log('CORRECT SIZE FOUND:', size, 'bytes');
      break;
    }
  }
}

findCorrectSize().catch(console.error);

