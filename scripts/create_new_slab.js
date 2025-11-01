// Create a fresh SOL-USD slab account
const { Connection, PublicKey, Keypair, Transaction, SystemProgram, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function createSlab() {
  console.log('Creating fresh SOL-USD slab account...\n');
  
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  const slabProgramId = new PublicKey(config.programs.slab.program_id);
  const routerProgramId = new PublicKey(config.programs.router.program_id);
  
  console.log('Slab Program:', slabProgramId.toString());
  console.log('Router Program:', routerProgramId.toString());
  console.log('Authority:', devWallet.publicKey.toString());
  console.log('');
  
  // Generate new keypair for the slab account
  const slabKeypair = Keypair.generate();
  console.log('New Slab Address:', slabKeypair.publicKey.toString());
  
  // Slab state size: ~65KB for the orderbook
  const SLAB_STATE_SIZE = 65536;
  
  // Get rent exemption
  const rentExemption = await connection.getMinimumBalanceForRentExemption(SLAB_STATE_SIZE);
  console.log('Rent exemption:', rentExemption / 1e9, 'SOL');
  
  // Instrument pubkey (dummy for v0)
  const instrumentKeypair = Keypair.generate();
  
  // Slab parameters
  const lpOwner = devWallet.publicKey; // LP owner
  const routerId = routerProgramId;
  const instrument = instrumentKeypair.publicKey;
  const markPx = 186_000_000; // $186 * 1e6
  const takerFeeBps = 5; // 0.05% = 5 bps
  const contractSize = 1_000_000; // 1.0 SOL = 1e6
  const bump = 0; // Not a PDA
  
  console.log('');
  console.log('Slab Parameters:');
  console.log('  LP Owner:', lpOwner.toString());
  console.log('  Router:', routerId.toString());
  console.log('  Instrument:', instrument.toString());
  console.log('  Mark Price:', markPx / 1e6, 'USD');
  console.log('  Taker Fee:', takerFeeBps, 'bps');
  console.log('  Contract Size:', contractSize / 1e6, 'SOL');
  console.log('');
  
  // Build Initialize instruction data
  const instructionData = Buffer.alloc(122);
  let offset = 0;
  
  // Discriminator: 0 = Initialize
  instructionData.writeUInt8(0, offset);
  offset += 1;
  
  // lp_owner: Pubkey (32 bytes)
  lpOwner.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // router_id: Pubkey (32 bytes)
  routerId.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // instrument: Pubkey (32 bytes)
  instrument.toBuffer().copy(instructionData, offset);
  offset += 32;
  
  // mark_px: i64 (8 bytes)
  instructionData.writeBigInt64LE(BigInt(markPx), offset);
  offset += 8;
  
  // taker_fee_bps: i64 (8 bytes)
  instructionData.writeBigInt64LE(BigInt(takerFeeBps), offset);
  offset += 8;
  
  // contract_size: i64 (8 bytes)
  instructionData.writeBigInt64LE(BigInt(contractSize), offset);
  offset += 8;
  
  // bump: u8 (1 byte)
  instructionData.writeUInt8(bump, offset);
  offset += 1;
  
  console.log('Instruction data size:', instructionData.length, 'bytes');
  
  // Create the slab account
  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: devWallet.publicKey,
    newAccountPubkey: slabKeypair.publicKey,
    lamports: rentExemption,
    space: SLAB_STATE_SIZE,
    programId: slabProgramId,
  });
  
  // Initialize the slab
  const initializeSlabIx = {
    keys: [
      { pubkey: slabKeypair.publicKey, isSigner: false, isWritable: true },
      { pubkey: devWallet.publicKey, isSigner: true, isWritable: false },
    ],
    programId: slabProgramId,
    data: instructionData,
  };
  
  // Send transaction
  const transaction = new Transaction();
  transaction.add(createAccountIx);
  transaction.add(initializeSlabIx);
  
  console.log('Sending transaction...');
  try {
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet, slabKeypair],
      { commitment: 'confirmed' }
    );
    
    console.log('✅ Slab created successfully!');
    console.log('   Signature:', signature);
    console.log('   Address:', slabKeypair.publicKey.toString());
    console.log('');
    
    // Update config
    if (!config.trading_pairs) config.trading_pairs = {};
    config.trading_pairs['SOL-USD'] = {
      symbol: 'SOL-USD',
      description: 'SOL/USDC perpetual order book',
      status: 'active',
      slab_address: slabKeypair.publicKey.toString(),
      created_at: new Date().toISOString(),
    };
    
    // Save slab keypair
    fs.writeFileSync(
      'keypairs/slab-sol-usd.json',
      JSON.stringify(Array.from(slabKeypair.secretKey))
    );
    
    fs.writeFileSync('keypairs/config.json', JSON.stringify(config, null, 2));
    console.log('✅ Slab address saved to config.json');
    console.log('✅ Slab keypair saved to keypairs/slab-sol-usd.json');
    
  } catch (error) {
    console.error('❌ Failed to create slab:', error.message);
    if (error.logs) {
      console.error('Logs:', error.logs);
    }
    throw error;
  }
}

createSlab().catch(console.error);

