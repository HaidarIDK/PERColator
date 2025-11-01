// Create the vault PDA for native SOL trading
const { Connection, PublicKey, Transaction, SystemProgram, Keypair, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function createVault() {
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  const routerProgramId = new PublicKey(config.programs.router.program_id);
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  console.log('Router Program:', routerProgramId.toString());
  console.log('Dev Wallet:', devWallet.publicKey.toString());
  console.log('');
  
  // Derive vault PDA (without mint for native SOL)
  const VAULT_SEED = Buffer.from('vault');
  const [vaultPDA, vaultBump] = PublicKey.findProgramAddressSync(
    [VAULT_SEED],
    routerProgramId
  );
  
  console.log('Vault PDA:', vaultPDA.toString());
  console.log('Vault Bump:', vaultBump);
  console.log('');
  
  // Check if already exists
  const vaultInfo = await connection.getAccountInfo(vaultPDA);
  if (vaultInfo) {
    console.log('✅ Vault already exists!');
    console.log('   Balance:', vaultInfo.lamports / 1e9, 'SOL');
    console.log('   Size:', vaultInfo.data.length, 'bytes');
    console.log('   Owner:', vaultInfo.owner.toString());
    return;
  }
  
  console.log('Creating vault PDA...');
  console.log('');
  
  // Vault struct size: router_id(32) + mint(32) + token_account(32) + balance(16) + total_pledged(16) + bump(1) + padding(7) = 136 bytes
  const vaultSize = 136;
  const rentExemption = await connection.getMinimumBalanceForRentExemption(vaultSize);
  
  console.log(`Vault size: ${vaultSize} bytes`);
  console.log(`Rent: ${(rentExemption / 1e9).toFixed(4)} SOL`);
  console.log('');
  
  // Create vault account
  const createAccountIx = SystemProgram.createAccount({
    fromPubkey: devWallet.publicKey,
    newAccountPubkey: vaultPDA,
    lamports: rentExemption,
    space: vaultSize,
    programId: routerProgramId,
  });
  
  console.log('⚠️ ERROR: Cannot create PDA directly with SystemProgram.createAccount!');
  console.log('PDAs must be created by the program itself via CPI.');
  console.log('');
  console.log('Solution: The vault PDA should be auto-created on first use by the router program.');
  console.log('The program code might be missing the vault auto-creation logic.');
}

createVault().catch(console.error);

