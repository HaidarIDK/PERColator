// Check if a portfolio exists for a user
const { Connection, PublicKey } = require('@solana/web3.js');
const readline = require('readline');

const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
const ROUTER_PROGRAM_ID = new PublicKey('rPB77V5pFZ3dzwgmUH5jvXDwzK7FTqs92nyFgBecxMh');

async function checkPortfolio(userAddress) {
  try {
    const userPubkey = new PublicKey(userAddress);
    
    console.log('\n=== Portfolio Check ===\n');
    console.log('User Address:', userPubkey.toString());
    console.log('Router Program:', ROUTER_PROGRAM_ID.toString());
    
    // Derive portfolio PDA
    const [portfolioPDA, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('portfolio'), userPubkey.toBuffer()],
      ROUTER_PROGRAM_ID
    );
    
    console.log('\nPortfolio PDA:', portfolioPDA.toString());
    console.log('Bump:', bump);
    
    // Check if account exists
    const accountInfo = await connection.getAccountInfo(portfolioPDA);
    
    if (accountInfo) {
      console.log('\n✓ Portfolio EXISTS!\n');
      console.log('Account Balance:', (accountInfo.lamports / 1e9).toFixed(4), 'SOL');
      console.log('Data Length:', accountInfo.data.length, 'bytes');
      console.log('Owner:', accountInfo.owner.toString());
      console.log('\nPortfolio is active and ready to trade!');
    } else {
      console.log('\n✗ Portfolio does NOT exist\n');
      console.log('This user needs to create a portfolio first.');
    }
    
  } catch (error) {
    console.error('\nError:', error.message);
  }
}

// Check if address provided as argument
const userAddress = process.argv[2];

if (userAddress) {
  checkPortfolio(userAddress);
} else {
  // Prompt for address
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });
  
  rl.question('Enter your wallet address: ', (address) => {
    rl.close();
    checkPortfolio(address.trim());
  });
}

