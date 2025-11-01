// Oracle keeper - automatically updates oracle price every 30 seconds
const { Connection, PublicKey, Transaction, TransactionInstruction, Keypair, sendAndConfirmTransaction } = require('@solana/web3.js');
const fs = require('fs');

let isRunning = true;

// Handle graceful shutdown
process.on('SIGINT', () => {
  console.log('\n🛑 Stopping oracle keeper...');
  isRunning = false;
  process.exit(0);
});

async function fetchRealSolPrice() {
  try {
    // Fetch real-time SOL price from CoinGecko
    const response = await fetch('https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd');
    const data = await response.json();
    return data.solana.usd;
  } catch (error) {
    console.error('Failed to fetch SOL price, using fallback:', error.message);
    return 195; // Fallback price if API fails
  }
}

async function updateOracle() {
  try {
    // Load config
    const config = JSON.parse(fs.readFileSync('keypairs/config.json', 'utf8'));
    const devWalletData = JSON.parse(fs.readFileSync(config.keypairs.devwallet.path, 'utf8'));
    const devWallet = Keypair.fromSecretKey(Uint8Array.from(devWalletData));
    
    // Oracle program ID (deployed)
    const oracleProgramId = new PublicKey('omYNPX19JABfHteKSEdHUggAM43EtTtAA1Uw3xp9sa4');
    const oracleAddress = new PublicKey(config.oracles['SOL-USD'].address);
    const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
    
    // Fetch REAL SOL price from CoinGecko API
    const currentPrice = await fetchRealSolPrice();
    const priceFixed = Math.floor(currentPrice * 1_000_000);
    const confidence = Math.floor(0.5 * 1_000_000); // $0.50 confidence interval
    
    // Build UpdatePrice instruction
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
    
    const transaction = new Transaction().add(updatePriceIx);
    
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [devWallet],
      { skipPreflight: false, commitment: 'confirmed' }
    );
    
    const timestamp = new Date().toISOString();
    console.log(`[${timestamp}] ✅ Oracle updated | Price: $${currentPrice} | Sig: ${signature.slice(0, 8)}...`);
    
  } catch (error) {
    const timestamp = new Date().toISOString();
    console.error(`[${timestamp}] ❌ Oracle update failed:`, error.message);
  }
}

async function runKeeper() {
  console.log('🤖 Oracle Keeper Started');
  console.log('📊 Fetching REAL SOL price from CoinGecko');
  console.log('🔄 Updating on-chain oracle every 10 seconds');
  console.log('Press Ctrl+C to stop\n');
  
  // Update immediately on start
  await updateOracle();
  
  // Then update every 10 seconds with REAL price data
  const interval = setInterval(async () => {
    if (!isRunning) {
      clearInterval(interval);
      return;
    }
    await updateOracle();
  }, 10000); // 10 seconds
}

runKeeper().catch(console.error);

