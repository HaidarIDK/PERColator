// Initialize the vault PDA for native SOL using the new InitializeVault instruction
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, SystemProgram, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function initializeVault() {
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  const routerProgramId = new PublicKey(config.programs.router.program_id);
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  console.log('Router Program:', routerProgramId.toString());
  console.log('Dev Wallet:', devWallet.publicKey.toString());
  console.log('');
  
  // Native SOL mint
  const nativeSolMint = new PublicKey('So11111111111111111111111111111111111111112');
  
  // Derive vault PDA
  const VAULT_SEED = Buffer.from('vault');
  const [vaultPDA, vaultBump] = PublicKey.findProgramAddressSync(
    [VAULT_SEED, nativeSolMint.toBuffer()],
    routerProgramId
  );
  
  console.log('Native SOL Mint:', nativeSolMint.toString());
  console.log('Vault PDA:', vaultPDA.toString());
  console.log('Bump:', vaultBump);
  console.log('');
  
  // Check if vault already exists
  const vaultInfo = await connection.getAccountInfo(vaultPDA);
  if (vaultInfo) {
    console.log('✅ Vault already exists!');
    console.log('   Balance:', vaultInfo.lamports / 1e9, 'SOL');
    console.log('   Size:', vaultInfo.data.length, 'bytes');
    console.log('   Owner:', vaultInfo.owner.toString());
    return;
  }
  
  console.log('Initializing vault...');
  console.log('');
  
  // Build InitializeVault instruction
  // Discriminator: 13
  // No additional data (vault PDA derived from mint)
  const instructionData = Buffer.alloc(1);
  instructionData.writeUInt8(13, 0); // InitializeVault discriminator
  
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: vaultPDA, isSigner: false, isWritable: true },
      { pubkey: nativeSolMint, isSigner: false, isWritable: false },
      { pubkey: devWallet.publicKey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: routerProgramId,
    data: instructionData,
  });
  
  const transaction = new Transaction().add(instruction);
  
  try {
    console.log('Sending InitializeVault transaction...');
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet],
      { skipPreflight: false, commitment: 'confirmed' }
    );
    
    console.log('✅ Vault initialized successfully!');
    console.log('   Signature:', signature);
    console.log('');
    
    // Verify
    const newVaultInfo = await connection.getAccountInfo(vaultPDA);
    if (newVaultInfo) {
      console.log('✅ Vault created:', vaultPDA.toString());
      console.log('   Balance:', newVaultInfo.lamports / 1e9, 'SOL');
      console.log('   Size:', newVaultInfo.data.length, 'bytes');
      console.log('   Owner:', newVaultInfo.owner.toString());
    }
    
  } catch (error) {
    console.error('❌ Vault initialization failed:');
    console.error(error);
    if (error.logs) {
      console.error('Logs:', error.logs);
    }
    process.exit(1);
  }
}

initializeVault().catch(console.error);

