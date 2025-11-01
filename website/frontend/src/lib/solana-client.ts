// Solana client for interacting with Percolator programs
import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Keypair,
} from '@solana/web3.js';
import {
  RPC_ENDPOINT,
  ROUTER_PROGRAM_ID,
  getRegistryAddress,
  derivePortfolioPDA,
  derivePortfolioAddress,
  deriveVaultPDA,
  deriveRouterAuthorityPDA,
  deriveReceiptPDA,
  getSlabForCoin,
  getOracleForCoin,
} from './program-config';

export const connection = new Connection(RPC_ENDPOINT, 'confirmed');

// Portfolio sizes
export const PORTFOLIO_SIZES = {
  TEST: 10 * 1024, // 10 KB - cheap for testing (~0.07 SOL)
  FULL: 500 * 1024, // 500 KB - full production size (~3.56 SOL)
};

export type PortfolioSize = 'TEST' | 'FULL';

// Get rent cost for portfolio size
export async function getPortfolioRentCost(size: PortfolioSize): Promise<number> {
  const sizeInBytes = PORTFOLIO_SIZES[size];
  const rent = await connection.getMinimumBalanceForRentExemption(sizeInBytes);
  return rent / 1e9; // Return in SOL
}

// Check if portfolio exists
export async function checkPortfolioExists(userPubkey: PublicKey): Promise<boolean> {
  const portfolioAddress = await derivePortfolioAddress(userPubkey);
  console.log('[Portfolio Check] User:', userPubkey.toString());
  console.log('[Portfolio Check] Address:', portfolioAddress.toString());
  
  const accountInfo = await connection.getAccountInfo(portfolioAddress);
  console.log('[Portfolio Check] Account exists:', accountInfo !== null);
  
  if (accountInfo) {
    console.log('[Portfolio Check] Balance:', accountInfo.lamports / 1e9, 'SOL');
    console.log('[Portfolio Check] Data length:', accountInfo.data.length, 'bytes');
  }
  
  return accountInfo !== null && accountInfo.data.length > 0;
}

// Create portfolio instructions (create account + initialize)
export async function createInitializePortfolioInstructions(
  userPubkey: PublicKey,
  size: PortfolioSize = 'TEST'
): Promise<TransactionInstruction[]> {
  const portfolioAddress = await derivePortfolioAddress(userPubkey);
  const portfolioSize = PORTFOLIO_SIZES[size];
  const rent = await connection.getMinimumBalanceForRentExemption(portfolioSize);

  console.log('[Portfolio Create] Size:', size, '=', portfolioSize, 'bytes');
  console.log('[Portfolio Create] Rent:', rent / 1e9, 'SOL');
  console.log('[Portfolio Create] Portfolio Address:', portfolioAddress.toString());

  const instructions: TransactionInstruction[] = [];

  // Check if account already exists
  const existingAccount = await connection.getAccountInfo(portfolioAddress);
  if (existingAccount) {
    console.warn('[Portfolio Create] Account already exists!');
    throw new Error('Portfolio already exists for this wallet');
  }

  // 1. Create the portfolio account using createAccountWithSeed
  const seed = 'portfolio';
  
  instructions.push(
    SystemProgram.createAccountWithSeed({
      fromPubkey: userPubkey,
      basePubkey: userPubkey,
      seed: seed,
      newAccountPubkey: portfolioAddress,
      lamports: rent,
      space: portfolioSize,
      programId: ROUTER_PROGRAM_ID,
    })
  );

  // 2. Initialize the portfolio (browser-compatible)
  const instructionData = new Uint8Array(33);
  instructionData[0] = 1; // InitializePortfolio discriminator
  instructionData.set(userPubkey.toBytes(), 1); // User pubkey at offset 1

  instructions.push(
    new TransactionInstruction({
      keys: [
        { pubkey: portfolioAddress, isSigner: false, isWritable: true },
        { pubkey: userPubkey, isSigner: true, isWritable: true },
      ],
      programId: ROUTER_PROGRAM_ID,
      data: instructionData,
    })
  );

  return instructions;
}

// Deposit instruction
export async function createDepositInstruction(
  userPubkey: PublicKey,
  amount: number // in lamports
): Promise<TransactionInstruction> {
  const portfolioAddress = await derivePortfolioAddress(userPubkey);

  // Instruction data: discriminator (1) + amount (8) - browser-compatible
  const instructionData = new Uint8Array(9);
  const dataView = new DataView(instructionData.buffer);
  
  instructionData[0] = 2; // Deposit discriminator
  dataView.setBigUint64(1, BigInt(amount), true); // amount in little-endian

  return new TransactionInstruction({
    keys: [
      { pubkey: portfolioAddress, isSigner: false, isWritable: true },
      { pubkey: userPubkey, isSigner: true, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: ROUTER_PROGRAM_ID,
    data: instructionData,
  });
}

// Place order via ExecuteCrossSlab instruction
export interface PlaceOrderParams {
  userPubkey: PublicKey;
  coin: 'SOL' | 'ETH' | 'BTC';
  side: 'buy' | 'sell';
  price: number; // in USD (will be converted to 1e6 scale)
  size: number; // in base units (e.g., SOL, will be converted to 1e6 scale)
}

export async function createPlaceOrderInstruction(params: PlaceOrderParams): Promise<{ instructions: TransactionInstruction[]; receiptKeypair: Keypair }> {
  const { userPubkey, coin, side, price, size } = params;

  // Get slab and oracle addresses for this coin
  const slabPubkey = getSlabForCoin(coin);
  if (!slabPubkey) {
    throw new Error(`Slab not found for ${coin}`);
  }
  
  const oraclePubkey = getOracleForCoin(coin);
  if (!oraclePubkey) {
    throw new Error(`Oracle not found for ${coin}`);
  }

  // Derive addresses
  const portfolioAddress = await derivePortfolioAddress(userPubkey);
  const registryAddress = await getRegistryAddress();
  const [vaultPDA] = deriveVaultPDA();
  const [routerAuthorityPDA] = deriveRouterAuthorityPDA();
  
  // Create a temporary receipt keypair (slab will write fill data to it)
  // FillReceipt size = 48 bytes
  const receiptKeypair = Keypair.generate();
  const receiptPDA = receiptKeypair.publicKey;
  
  // Get rent exemption for receipt
  const RECEIPT_SIZE = 48;
  const rentExemption = await connection.getMinimumBalanceForRentExemption(RECEIPT_SIZE);
  
  // Create the receipt account first
  const createReceiptInstruction = SystemProgram.createAccount({
    fromPubkey: userPubkey,
    newAccountPubkey: receiptPDA,
    lamports: rentExemption,
    space: RECEIPT_SIZE,
    programId: ROUTER_PROGRAM_ID,
  });

  // Convert to fixed-point (1e6 scale)
  const priceFixed = Math.floor(price * 1_000_000);
  const sizeFixed = Math.floor(size * 1_000_000);
  const sideU8 = side === 'buy' ? 0 : 1;

  // Build instruction data for ExecuteCrossSlab (browser-compatible)
  // Layout: discriminator (1) + num_splits (1) + [side (1) + qty (8) + limit_px (8)] per split
  const numSplits = 1;
  const instructionData = new Uint8Array(1 + 1 + 17);
  const dataView = new DataView(instructionData.buffer);
  
  instructionData[0] = 4; // ExecuteCrossSlab discriminator
  instructionData[1] = numSplits; // Number of splits
  
  // Split 0
  let offset = 2;
  instructionData[offset] = sideU8; // side
  offset += 1;
  
  // qty (8 bytes, little-endian i64)
  dataView.setBigInt64(offset, BigInt(sizeFixed), true);
  offset += 8;
  
  // limit_px (8 bytes, little-endian i64)
  dataView.setBigInt64(offset, BigInt(priceFixed), true);
  
  // Build account list
  // For num_splits=1, the account order MUST be:
  // 0. portfolio (writable)
  // 1. user (signer)
  // 2. vault (writable)
  // 3. registry (writable)
  // 4. router_authority (PDA)
  // 5. oracle_0 (first oracle)
  // 6. slab_0 (first slab, writable)
  // 7. receipt_0 (first receipt, writable)
  //
  // Program slices: oracles[5..6], slabs[6..7], receipts[7..8]
  
  const placeOrderInstruction = new TransactionInstruction({
    keys: [
      { pubkey: portfolioAddress, isSigner: false, isWritable: true },
      { pubkey: userPubkey, isSigner: true, isWritable: false },
      { pubkey: vaultPDA, isSigner: false, isWritable: true },
      { pubkey: registryAddress, isSigner: false, isWritable: true },
      { pubkey: routerAuthorityPDA, isSigner: false, isWritable: false },
      // Oracle for split 0 (real oracle account)
      { pubkey: oraclePubkey, isSigner: false, isWritable: false },
      // Slab for split 0
      { pubkey: slabPubkey, isSigner: false, isWritable: true },
      // Receipt for split 0
      { pubkey: receiptPDA, isSigner: false, isWritable: true },
    ],
    programId: ROUTER_PROGRAM_ID,
    data: instructionData,
  });
  
  return {
    instructions: [createReceiptInstruction, placeOrderInstruction],
    receiptKeypair,
  };
}

// Get portfolio balance
export async function getPortfolioBalance(userPubkey: PublicKey): Promise<number> {
  const portfolioAddress = await derivePortfolioAddress(userPubkey);
  const accountInfo = await connection.getAccountInfo(portfolioAddress);
  
  if (!accountInfo || accountInfo.data.length === 0) {
    console.log('[Portfolio Balance] No account found');
    return 0;
  }
  
  console.log('[Portfolio Balance] Account lamports:', accountInfo.lamports);
  
  // Read principal balance from portfolio data (offset depends on struct layout)
  // For now, return account balance minus rent
  return accountInfo.lamports;
}

// Percolator client object (for backward compatibility with existing components)
export const percolatorClient = {
  connection,
  
  async portfolioExists(userPubkey: PublicKey): Promise<boolean> {
    return checkPortfolioExists(userPubkey);
  },
  
  async getBalance(userPubkey: PublicKey): Promise<number> {
    const balance = await connection.getBalance(userPubkey);
    return balance / 1e9; // Return in SOL
  },
  
  async getPortfolio(userPubkey: PublicKey) {
    const balance = await getPortfolioBalance(userPubkey);
    return balance > 0 ? { balance } : null;
  },
  
  async createInitPortfolioInstructions(userPubkey: PublicKey, size: PortfolioSize = 'TEST'): Promise<TransactionInstruction[]> {
    return await createInitializePortfolioInstructions(userPubkey, size);
  },
  
  async createDepositInstruction(userPubkey: PublicKey, lamports: number): Promise<TransactionInstruction> {
    return await createDepositInstruction(userPubkey, lamports);
  },
};
