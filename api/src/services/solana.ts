import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import { AnchorProvider, Program, Wallet } from '@coral-xyz/anchor';
import fs from 'fs';

let connection: Connection;
let provider: AnchorProvider;
let slabProgram: Program | null = null;
let wallet: Wallet;

export async function initializeSolana() {
  const rpcUrl = process.env.SOLANA_RPC_URL || 'http://localhost:8899';
  connection = new Connection(rpcUrl, 'confirmed');

  // Load wallet (for signing transactions if needed)
  const walletPath = process.env.WALLET_PATH || `${process.env.HOME}/.config/solana/id.json`;
  let keypair: Keypair;
  
  try {
    const walletData = JSON.parse(fs.readFileSync(walletPath, 'utf-8'));
    keypair = Keypair.fromSecretKey(new Uint8Array(walletData));
    console.log('Loaded wallet from', walletPath);
  } catch (error) {
    console.warn('No wallet found, using dummy keypair (read-only mode)');
    keypair = Keypair.generate();
  }

  wallet = new Wallet(keypair);
  provider = new AnchorProvider(connection, wallet, {
    commitment: 'confirmed',
    preflightCommitment: 'confirmed',
  });

  // Test connection
  try {
    const version = await connection.getVersion();
    console.log('Connected to Solana cluster version:', version['solana-core']);
  } catch (error) {
    console.warn('Solana RPC not available, API will work with mock data only');
  }

  // TODO: Load slab program IDL and initialize Program
  // const slabProgramId = new PublicKey(process.env.SLAB_PROGRAM_ID!);
  // slabProgram = new Program(IDL, slabProgramId, provider);
}

export function getConnection(): Connection {
  if (!connection) throw new Error('Solana connection not initialized');
  return connection;
}

export function getProvider(): AnchorProvider {
  if (!provider) throw new Error('Anchor provider not initialized');
  return provider;
}

export function getSlabProgram(): Program {
  if (!slabProgram) throw new Error('Slab program not initialized');
  return slabProgram;
}

export function getWallet(): Wallet {
  if (!wallet) throw new Error('Wallet not initialized');
  return wallet;
}

// Helper to fetch slab state
export async function fetchSlabState(slabAddress: PublicKey): Promise<any> {
  const accountInfo = await connection.getAccountInfo(slabAddress);
  if (!accountInfo) throw new Error('Slab account not found');
  
  // TODO: Parse SlabState from account data
  // For now, return raw data
  return {
    data: accountInfo.data,
    owner: accountInfo.owner.toBase58(),
    lamports: accountInfo.lamports,
  };
}

