// Initialize a price oracle for SOL/USD
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, SystemProgram, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function initializeOracle() {
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  // Load or create oracle program keypair
  let oracleProgramKeypair;
  try {
    const oracleProgramData = JSON.parse(fs.readFileSync('keypairs/oracle.json', 'utf8'));
    oracleProgramKeypair = Keypair.fromSecretKey(Uint8Array.from(oracleProgramData));
  } catch (e) {
    console.log('Oracle program keypair not found, generating new one...');
    oracleProgramKeypair = Keypair.generate();
    fs.writeFileSync('keypairs/oracle.json', JSON.stringify(Array.from(oracleProgramKeypair.secretKey)));
  }
  
  const oracleProgramId = oracleProgramKeypair.publicKey;
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  console.log('Oracle Program ID:', oracleProgramId.toString());
  console.log('Authority (DevWallet):', devWallet.publicKey.toString());
  console.log('');
  
  // Check if oracle program is deployed
  const programInfo = await connection.getAccountInfo(oracleProgramId);
  if (!programInfo || !programInfo.executable) {
    console.log('❌ Oracle program not deployed!');
    console.log('');
    console.log('To deploy the oracle program:');
    console.log('  cd programs/oracle');
    console.log('  cargo build-sbf');
    console.log('  solana program deploy target/deploy/percolator_oracle.so --keypair ../../keypairs/oracle.json --url devnet');
    console.log('');
    process.exit(1);
  }
  
  console.log('✅ Oracle program is deployed');
  console.log('');
  
  // Create a regular keypair for the oracle account (simpler than PDA for v0)
  // In production, you'd use a PDA, but for testing a keypair is fine
  let oracleKeypair;
  try {
    const oracleData = JSON.parse(fs.readFileSync('keypairs/oracle-account-sol-usd.json', 'utf8'));
    oracleKeypair = Keypair.fromSecretKey(Uint8Array.from(oracleData));
    console.log('Using existing oracle account keypair');
  } catch (e) {
    console.log('Generating new oracle account keypair...');
    oracleKeypair = Keypair.generate();
    fs.writeFileSync('keypairs/oracle-account-sol-usd.json', JSON.stringify(Array.from(oracleKeypair.secretKey)));
  }
  
  const instrumentKeypair = Keypair.generate(); // Dummy instrument for v0
  
  console.log('Instrument:', instrumentKeypair.publicKey.toString());
  console.log('Oracle Account:', oracleKeypair.publicKey.toString());
  console.log('');
  
  // Check if oracle already exists
  const oracleInfo = await connection.getAccountInfo(oracleKeypair.publicKey);
  if (oracleInfo) {
    console.log('✅ Oracle already initialized!');
    console.log('   Address:', oracleKeypair.publicKey.toString());
    
    // Read current price
    if (oracleInfo.data.length >= 16) {
      const priceBytes = oracleInfo.data.slice(8, 16); // price is at offset 8
      const price = Buffer.from(priceBytes).readBigInt64LE();
      console.log('   Current Price:', Number(price) / 1_000_000, 'USD');
    }
    
    // Save to config
    if (!config.oracles) config.oracles = {};
    config.oracles['SOL-USD'] = {
      address: oracleKeypair.publicKey.toString(),
      instrument: instrumentKeypair.publicKey.toString(),
      created_at: new Date().toISOString()
    };
    fs.writeFileSync('keypairs/config.json', JSON.stringify(config, null, 2));
    console.log('');
    console.log('✅ Oracle address saved to config.json');
    return;
  }
  
  console.log('Initializing oracle...');
  console.log('');
  
  // Get current SOL price from a price feed (use a reasonable default)
  const currentPrice = 195; // SOL price in USD (update this manually or fetch from API)
  const priceFixed = Math.floor(currentPrice * 1_000_000); // Convert to 1e6 scale
  
  console.log('Initial Price:', currentPrice, 'USD (', priceFixed, 'fixed-point)');
  console.log('');
  
  // Calculate rent for oracle account
  const oracleSize = 128; // PriceOracle size
  const rent = await connection.getMinimumBalanceForRentExemption(oracleSize);
  
  // Create oracle account
  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: devWallet.publicKey,
    newAccountPubkey: oracleKeypair.publicKey,
    lamports: rent,
    space: oracleSize,
    programId: oracleProgramId,
  });
  
  // Build Initialize instruction
  // Discriminator: 0
  // Data: initial_price (8 bytes) + bump (1 byte)
  // Since we're using a keypair instead of PDA, bump is not used but required by instruction
  const instructionData = Buffer.alloc(9);
  instructionData.writeBigInt64LE(BigInt(priceFixed), 0);
  instructionData.writeUInt8(0, 8); // Bump = 0 (not used for keypair accounts)
  
  const initializeIx = new TransactionInstruction({
    keys: [
      { pubkey: oracleKeypair.publicKey, isSigner: false, isWritable: true },
      { pubkey: devWallet.publicKey, isSigner: true, isWritable: false },
      { pubkey: instrumentKeypair.publicKey, isSigner: false, isWritable: false },
    ],
    programId: oracleProgramId,
    data: Buffer.concat([Buffer.from([0]), instructionData]), // 0 = Initialize discriminator
  });
  
  // Send transaction
  try {
    const transaction = new Transaction().add(createAccountIx, initializeIx);
    
    console.log('Sending Initialize transaction...');
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet, oracleKeypair], // Both must sign
      { skipPreflight: false, commitment: 'confirmed' }
    );
    
    console.log('✅ Oracle initialized successfully!');
    console.log('   Signature:', signature);
    console.log('   Address:', oracleKeypair.publicKey.toString());
    console.log('');
    
    // Save to config
    if (!config.oracles) config.oracles = {};
    config.oracles['SOL-USD'] = {
      address: oracleKeypair.publicKey.toString(),
      instrument: instrumentKeypair.publicKey.toString(),
      program_id: oracleProgramId.toString(),
      initial_price: currentPrice,
      created_at: new Date().toISOString()
    };
    fs.writeFileSync('keypairs/config.json', JSON.stringify(config, null, 2));
    
    console.log('✅ Oracle address saved to config.json');
    
  } catch (error) {
    console.error('❌ Oracle initialization failed:');
    console.error(error);
    if (error.logs) {
      console.error('Logs:', error.logs);
    }
    process.exit(1);
  }
}

initializeOracle().catch(console.error);

