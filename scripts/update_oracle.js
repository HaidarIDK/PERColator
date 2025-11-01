// Update oracle price to current timestamp
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

async function updateOracle() {
  // Load config
  const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
  const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
  const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
  
  // Oracle program ID (deployed)
  const oracleProgramId = new PublicKey('omYNPX19JABfHteKSEdHUggAM43EtTtAA1Uw3xp9sa4');
  const oracleAddress = new PublicKey(config.oracles['SOL-USD'].address);
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  console.log('Oracle Program:', oracleProgramId.toString());
  console.log('Oracle Account:', oracleAddress.toString());
  console.log('Authority:', devWallet.publicKey.toString());
  console.log('');
  
  // Fetch real SOL price from CoinGecko
  console.log('Fetching real-time SOL price from CoinGecko...');
  const response = await fetch('https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd');
  const data = await response.json();
  const currentPrice = data.solana.usd;
  
  const priceFixed = Math.floor(currentPrice * 1_000_000); // Convert to 1e6 scale
  const confidence = Math.floor(0.5 * 1_000_000); // $0.50 confidence interval
  
  console.log('Real-time SOL price:', currentPrice, 'USD');
  console.log('Confidence:', '$0.50');
  console.log('');
  
  // Build UpdatePrice instruction
  // Discriminator: 1
  // Data: price (8 bytes) + confidence (8 bytes)
  const instructionData = Buffer.alloc(17);
  instructionData.writeUInt8(1, 0); // UpdatePrice discriminator
  instructionData.writeBigInt64LE(BigInt(priceFixed), 1);
  instructionData.writeBigInt64LE(BigInt(confidence), 9);
  
  const updatePriceIx = new TransactionInstruction({
    keys: [
      { pubkey: oracleAddress, isSigner: false, isWritable: true },
      { pubkey: devWallet.publicKey, isSigner: true, isWritable: false },
    ],
    programId: oracleProgramId,
    data: instructionData,
  });
  
  try {
    const transaction = new Transaction().add(updatePriceIx);
    
    console.log('Sending UpdatePrice transaction...');
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet],
      { skipPreflight: false, commitment: 'confirmed' }
    );
    
    console.log('✅ Oracle price updated successfully!');
    console.log('   Signature:', signature);
    console.log('   Price:', currentPrice, 'USD');
    console.log('   Timestamp: Current (fresh)');
    console.log('');
    console.log('🎉 You can now place orders!');
    
  } catch (error) {
    console.error('❌ Oracle update failed:');
    console.error(error);
    if (error.logs) {
      console.error('Logs:', error.logs);
    }
    process.exit(1);
  }
}

updateOracle().catch(console.error);

