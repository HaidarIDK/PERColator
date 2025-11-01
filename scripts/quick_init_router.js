// Quick script to initialize router registry and vault on devnet
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, SystemProgram, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function initializeRouter() {
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  const routerProgramId = new PublicKey(config.programs.router.program_id);
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  console.log('Router Program:', routerProgramId.toString());
  console.log('Dev Wallet:', devWallet.publicKey.toString());
  console.log('');
  
  // Derive PDAs
  const REGISTRY_SEED = Buffer.from('registry');
  const VAULT_SEED = Buffer.from('vault');
  
  const [registryPDA, registryBump] = PublicKey.findProgramAddressSync(
    [REGISTRY_SEED],
    routerProgramId
  );
  
  const [vaultPDA, vaultBump] = PublicKey.findProgramAddressSync(
    [VAULT_SEED],
    routerProgramId
  );
  
  console.log('Registry PDA (authority):', registryPDA.toString());
  console.log('Vault PDA:', vaultPDA.toString());
  console.log('');
  
  // Derive the registry address using createWithSeed (actual account address)
  const registrySeed = 'registry';
  const derivedRegistryAddr = await PublicKey.createWithSeed(
    devWallet.publicKey,
    registrySeed,
    routerProgramId
  );
  
  console.log('Registry Address (with seed):', derivedRegistryAddr.toString());
  console.log('');
  
  // Check if already initialized
  const registryInfo = await connection.getAccountInfo(derivedRegistryAddr);
  
  if (registryInfo) {
    console.log('✅ Router already initialized!');
    console.log('   Registry:', registryInfo.lamports / 1e9, 'SOL');
    console.log('   Size:', registryInfo.data.length, 'bytes');
    return;
  }
  
  console.log('Initializing router...');
  console.log('');
  
  // Step 1: Create registry account using createAccountWithSeed
  const registrySize = 512 * 1024; // 512 KB for SlabRegistry
  const rentExemption = await connection.getMinimumBalanceForRentExemption(registrySize);
  
  console.log(`Creating registry account (${registrySize} bytes, ${(rentExemption / 1e9).toFixed(4)} SOL rent)...`);
  
  const createAccountIx = SystemProgram.createAccountWithSeed({
    fromPubkey: devWallet.publicKey,
    basePubkey: devWallet.publicKey,
    seed: registrySeed,
    newAccountPubkey: derivedRegistryAddr,
    lamports: rentExemption,
    space: registrySize,
    programId: routerProgramId,
  });
  
  // Step 2: Initialize registry with governance
  const instructionData = Buffer.alloc(33);
  instructionData.writeUInt8(0, 0); // Initialize discriminator
  devWallet.publicKey.toBuffer().copy(instructionData, 1);
  
  const initializeIx = new TransactionInstruction({
    keys: [
      { pubkey: derivedRegistryAddr, isSigner: false, isWritable: true },
      { pubkey: devWallet.publicKey, isSigner: true, isWritable: true },
    ],
    programId: routerProgramId,
    data: instructionData,
  });
  
  // Send both instructions in one transaction
  const transaction = new Transaction()
    .add(createAccountIx)
    .add(initializeIx);
  
  try {
    console.log('Sending initialize transaction...');
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet],
      { skipPreflight: false, commitment: 'confirmed' }
    );
    
    console.log('✅ Router initialized successfully!');
    console.log('   Signature:', signature);
    console.log('');
    
    // Verify
    const newRegistryInfo = await connection.getAccountInfo(derivedRegistryAddr);
    const newVaultInfo = await connection.getAccountInfo(vaultPDA);
    
    if (newRegistryInfo) {
      console.log('✅ Registry created:', derivedRegistryAddr.toString());
      console.log('   Balance:', newRegistryInfo.lamports / 1e9, 'SOL');
      console.log('   Owner:', newRegistryInfo.owner.toString());
      console.log('   Size:', newRegistryInfo.data.length, 'bytes');
    }
    
    if (newVaultInfo) {
      console.log('✅ Vault created:', vaultPDA.toString());
      console.log('   Balance:', newVaultInfo.lamports / 1e9, 'SOL');
      console.log('   Owner:', newVaultInfo.owner.toString());
    } else {
      console.log('ℹ️ Vault PDA not created yet (will be created on first use)');
    }
    
  } catch (error) {
    console.error('❌ Initialization failed:');
    console.error(error);
    if (error.logs) {
      console.error('Logs:', error.logs);
    }
    process.exit(1);
  }
}

initializeRouter().catch(console.error);

