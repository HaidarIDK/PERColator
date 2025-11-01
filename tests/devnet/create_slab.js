// Create a slab account for SOL/USDC trading
const { Connection, Keypair, Transaction, SystemProgram, PublicKey } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

async function createSlab() {
  console.log('\n=== Creating SOL/USDC Slab ===\n');
  
  // Load config
  const configPath = path.join(__dirname, '../../keypairs/config.json');
  const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
  
  // Load devwallet keypair
  const devWalletPath = path.join(__dirname, '../../keypairs/devwallet.json');
  const devWalletKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(devWalletPath, 'utf8')))
  );
  
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  const slabProgramId = new PublicKey(config.programs.slab.program_id);
  const routerProgramId = new PublicKey(config.programs.router.program_id);
  
  console.log('Devnet Wallet:', devWalletKeypair.publicKey.toString());
  console.log('Slab Program:', slabProgramId.toString());
  console.log('Router Program:', routerProgramId.toString());
  
  // Generate new keypair for the slab account
  const slabKeypair = Keypair.generate();
  console.log('\nNew Slab Account:', slabKeypair.publicKey.toString());
  
  // Calculate rent for 4KB account
  const SLAB_SIZE = 4096;
  const rent = await connection.getMinimumBalanceForRentExemption(SLAB_SIZE);
  console.log('Rent Required:', rent, 'lamports');
  
  // Build instruction data for Initialize
  // Format: [discriminator(1), lp_owner(32), router_id(32), instrument(32),
  //          mark_px(8), taker_fee_bps(8), contract_size(8), bump(1)]
  const instructionData = Buffer.alloc(122);
  let offset = 0;
  
  // Discriminator: 0 (Initialize)
  instructionData.writeUInt8(0, offset);
  offset += 1;
  
  // LP Owner: devwallet
  devWalletKeypair.publicKey.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // Router ID: router program
  routerProgramId.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // Instrument: system program (dummy)
  SystemProgram.programId.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // Mark price: $150 (for SOL) = 150_000_000 in 1e6 scale
  const markPrice = BigInt(150_000_000);
  instructionData.writeBigInt64LE(markPrice, offset);
  offset += 8;
  
  // Taker fee: 20 bps (0.2%)
  const takerFeeBps = BigInt(20);
  instructionData.writeBigInt64LE(takerFeeBps, offset);
  offset += 8;
  
  // Contract size: 1,000,000 (1 SOL in 1e6 scale)
  const contractSize = BigInt(1_000_000);
  instructionData.writeBigInt64LE(contractSize, offset);
  offset += 8;
  
  // Bump: 0 (not using PDA)
  instructionData.writeUInt8(0, offset);
  
  console.log('\nCreating slab account...');
  
  // Create transaction with both instructions
  const transaction = new Transaction();
  
  // 1. Create account instruction
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: devWalletKeypair.publicKey,
      newAccountPubkey: slabKeypair.publicKey,
      lamports: rent,
      space: SLAB_SIZE,
      programId: slabProgramId,
    })
  );
  
  // 2. Initialize instruction
  transaction.add({
    keys: [
      { pubkey: slabKeypair.publicKey, isSigner: false, isWritable: true },
      { pubkey: devWalletKeypair.publicKey, isSigner: true, isWritable: true },
    ],
    programId: slabProgramId,
    data: instructionData,
  });
  
  // Send transaction
  const signature = await connection.sendTransaction(
    transaction,
    [devWalletKeypair, slabKeypair],
    { skipPreflight: false }
  );
  
  console.log('\nWaiting for confirmation...');
  await connection.confirmTransaction(signature, 'confirmed');
  
  console.log('\n✓ Slab created successfully!');
  console.log('Transaction:', signature);
  console.log('Slab Address:', slabKeypair.publicKey.toString());
  
  // Save slab info to config
  config.trading_pairs['SOL-USD'].slab_address = slabKeypair.publicKey.toString();
  config.trading_pairs['SOL-USD'].status = 'active';
  config.trading_pairs['SOL-USD'].created_at = new Date().toISOString();
  
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
  console.log('\n✓ Config updated');
  
  // Save slab keypair
  const slabKeypairPath = path.join(__dirname, '../../keypairs/slab-sol-usd.json');
  fs.writeFileSync(slabKeypairPath, JSON.stringify(Array.from(slabKeypair.secretKey)));
  console.log('✓ Slab keypair saved to', slabKeypairPath);
  
  console.log('\n=== SOL/USDC Order Book Ready ===\n');
  console.log('Symbol: SOL/USDC');
  console.log('Mark Price: $150');
  console.log('Contract Size: 1 SOL');
  console.log('Taker Fee: 0.2%');
  console.log('\nYou can now place orders on this slab!');
}

createSlab().catch(console.error);

