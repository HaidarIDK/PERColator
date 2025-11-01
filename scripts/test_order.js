// Test order placement with full logging
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, SystemProgram, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function testOrder() {
  console.log('🧪 Testing Order Placement\n');
  
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  const routerProgramId = new PublicKey(config.programs.router.program_id);
  const slabAddress = new PublicKey(config.trading_pairs['SOL-USD'].slab_address);
  const oracleAddress = new PublicKey(config.oracles['SOL-USD'].address);
  
  console.log('Router Program:', routerProgramId.toString());
  console.log('Slab Address:', slabAddress.toString());
  console.log('Oracle Address:', oracleAddress.toString());
  console.log('User:', devWallet.publicKey.toString());
  console.log('');
  
  // Derive portfolio address (using createWithSeed, not PDA!)
  const portfolioAddress = await PublicKey.createWithSeed(
    devWallet.publicKey,
    'portfolio',
    routerProgramId
  );
  console.log('Portfolio Address:', portfolioAddress.toString());
  
  // Derive vault PDA
  const NATIVE_SOL_MINT = new PublicKey('So11111111111111111111111111111111111111112');
  const [vaultPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from('vault'), NATIVE_SOL_MINT.toBuffer()],
    routerProgramId
  );
  console.log('Vault PDA:', vaultPDA.toString());
  
  // Derive router authority PDA (seed is just "authority")
  const [routerAuthorityPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from('authority')],
    routerProgramId
  );
  console.log('Router Authority PDA:', routerAuthorityPDA.toString());
  
  // Derive registry address (using createWithSeed)
  const devWalletPubkey = new PublicKey(config.keypairs.devwallet.pubkey);
  const registryAddress = await PublicKey.createWithSeed(
    devWalletPubkey,
    'registry',
    routerProgramId
  );
  console.log('Registry Address:', registryAddress.toString());
  console.log('');
  
  // Create receipt account (temporary)
  const receiptKeypair = Keypair.generate();
  const RECEIPT_SIZE = 48;
  const rentExemption = await connection.getMinimumBalanceForRentExemption(RECEIPT_SIZE);
  
  console.log('Receipt Account:', receiptKeypair.publicKey.toString());
  console.log('Receipt Rent:', rentExemption / 1e9, 'SOL');
  console.log('');
  
  const createReceiptIx = SystemProgram.createAccount({
    fromPubkey: devWallet.publicKey,
    newAccountPubkey: receiptKeypair.publicKey,
    lamports: rentExemption,
    space: RECEIPT_SIZE,
    programId: routerProgramId,
  });
  
  // Build ExecuteCrossSlab instruction
  const side = 0; // Buy
  const size = 1.0; // 1 SOL
  const price = 190.0; // $190
  
  const sizeFixed = Math.floor(size * 1_000_000);
  const priceFixed = Math.floor(price * 1_000_000);
  
  console.log('Order Parameters:');
  console.log('  Side: BUY');
  console.log('  Size:', size, 'SOL');
  console.log('  Price: $' + price);
  console.log('  Size (fixed):', sizeFixed);
  console.log('  Price (fixed):', priceFixed);
  console.log('');
  
  // Build instruction data
  const numSplits = 1;
  const instructionData = new Uint8Array(1 + 1 + 17);
  const dataView = new DataView(instructionData.buffer);
  
  instructionData[0] = 4; // ExecuteCrossSlab discriminator
  instructionData[1] = numSplits;
  
  let offset = 2;
  instructionData[offset] = side;
  offset += 1;
  dataView.setBigInt64(offset, BigInt(sizeFixed), true);
  offset += 8;
  dataView.setBigInt64(offset, BigInt(priceFixed), true);
  
  console.log('Instruction Data:', Buffer.from(instructionData).toString('hex'));
  console.log('');
  
  // Build account list
  console.log('Accounts for ExecuteCrossSlab:');
  console.log('  0. portfolio (writable):', portfolioAddress.toString());
  console.log('  1. user (signer):', devWallet.publicKey.toString());
  console.log('  2. vault (writable):', vaultPDA.toString());
  console.log('  3. registry (writable):', registryAddress.toString());
  console.log('  4. router_authority:', routerAuthorityPDA.toString());
  console.log('  5. oracle_0:', oracleAddress.toString());
  console.log('  6. slab_0 (writable):', slabAddress.toString());
  console.log('  7. receipt_0 (writable):', receiptKeypair.publicKey.toString());
  console.log('');
  
  const placeOrderIx = new TransactionInstruction({
    keys: [
      { pubkey: portfolioAddress, isSigner: false, isWritable: true },
      { pubkey: devWallet.publicKey, isSigner: true, isWritable: false },
      { pubkey: vaultPDA, isSigner: false, isWritable: true },
      { pubkey: registryAddress, isSigner: false, isWritable: true },
      { pubkey: routerAuthorityPDA, isSigner: false, isWritable: false },
      { pubkey: oracleAddress, isSigner: false, isWritable: false },
      { pubkey: slabAddress, isSigner: false, isWritable: true },
      { pubkey: receiptKeypair.publicKey, isSigner: false, isWritable: true },
    ],
    programId: routerProgramId,
    data: instructionData,
  });
  
  // Build and send transaction
  const transaction = new Transaction();
  transaction.add(createReceiptIx);
  transaction.add(placeOrderIx);
  
  console.log('Sending transaction...');
  try {
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet, receiptKeypair],
      {
        commitment: 'confirmed',
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      }
    );
    
    console.log('');
    console.log('✅ Order placed successfully!');
    console.log('   Signature:', signature);
    console.log('');
    console.log('View on Solana Explorer:');
    console.log('   https://explorer.solana.com/tx/' + signature + '?cluster=devnet');
    
  } catch (error) {
    console.error('');
    console.error('❌ Order failed:', error.message);
    console.error('');
    
    if (error.logs) {
      console.error('Transaction Logs:');
      error.logs.forEach((log, i) => console.error('  ' + i + ':', log));
    }
    
    throw error;
  }
}

testOrder().catch(console.error);

