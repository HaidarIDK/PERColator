import { Router, Request, Response } from 'express';
import { Connection, PublicKey } from '@solana/web3.js';
import fs from 'fs';
import path from 'path';
import { parseSlabOrderbook } from '../services/slab-parser';

export const slabRouter = Router();

// Read config to get slab address
const configPath = path.join(process.cwd(), '../../keypairs/config.json');
let SLAB_ACCOUNT: PublicKey;
let SLAB_PROGRAM_ID: PublicKey;

try {
  const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
  SLAB_ACCOUNT = new PublicKey(config.trading_pairs['SOL-USD'].slab_address);
  SLAB_PROGRAM_ID = new PublicKey(config.programs.slab.program_id);
  console.log('✅ Loaded slab config:', {
    slab: SLAB_ACCOUNT.toBase58(),
    program: SLAB_PROGRAM_ID.toBase58()
  });
} catch (error) {
  console.error('Failed to read config.json, using fallback slab address', error);
  SLAB_ACCOUNT = new PublicKey('7pyCSG18qBXipgUCApPxBo48T3a1M7arhrf1AbkMtAFL');
  SLAB_PROGRAM_ID = new PublicKey('sDoE9Fb3H516S5ZuxBD1SUFgP1Z7ddXBpi9fweZb6Mk');
}

// Dynamic mock data generator
function generateDynamicOrderbook(basePrice: number = 186) {
  // Add some random volatility (±2%)
  const priceVolatility = (Math.random() - 0.5) * 0.04;
  const markPrice = basePrice * (1 + priceVolatility);
  
  // Generate 8-10 bids
  const numBids = 8 + Math.floor(Math.random() * 3);
  const bids = [];
  for (let i = 0; i < numBids; i++) {
    const offset = (0.005 + (i * 0.004)) * markPrice; // 0.5% to 4% below
    const price = Math.floor(markPrice - offset);
    const quantity = parseFloat((Math.random() * 4 + 1).toFixed(3)); // 1-5 SOL
    bids.push({ price, quantity });
  }
  
  // Generate 8-10 asks
  const numAsks = 8 + Math.floor(Math.random() * 3);
  const asks = [];
  for (let i = 0; i < numAsks; i++) {
    const offset = (0.005 + (i * 0.004)) * markPrice; // 0.5% to 4% above
    const price = Math.ceil(markPrice + offset);
    const quantity = parseFloat((Math.random() * 4 + 1).toFixed(3)); // 1-5 SOL
    asks.push({ price, quantity });
  }
  
  return { bids, asks, markPrice };
}

const mockRecentTrades = [
  {
    price: 186.75,
    quantity: 0.5,
    timestamp: Date.now() - 30000,
    side: 'buy',
    solscanLink: 'https://solscan.io/tx/example?cluster=devnet'
  },
  {
    price: 186.50,
    quantity: 1.2,
    timestamp: Date.now() - 60000,
    side: 'sell',
    solscanLink: 'https://solscan.io/tx/example2?cluster=devnet'
  },
];

/**
 * GET /api/slab/orderbook
 * Fetch order book data from the Slab account
 * Query params:
 * - coin: SOL, ETH, BTC (defaults to SOL)
 */
slabRouter.get('/orderbook', async (req: Request, res: Response) => {
  try {
    const coin = (req.query.coin as string) || 'SOL';
    
    // Get slab address for this coin
    let slabAccount: PublicKey;
    try {
      const config = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
      const pairKey = `${coin}-USD`;
      
      if (config.trading_pairs[pairKey]?.slab_address) {
        slabAccount = new PublicKey(config.trading_pairs[pairKey].slab_address);
      } else {
        slabAccount = SLAB_ACCOUNT; // Fallback to default SOL slab
      }
    } catch (error) {
      slabAccount = SLAB_ACCOUNT; // Fallback to default
    }
    
    console.log(`Fetching order book for ${coin} from Slab:`, slabAccount.toBase58());
    
    // Connect to Solana devnet
    const connection = new Connection(
      process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      'confirmed'
    );
    
    // Fetch Slab account data
    const accountInfo = await connection.getAccountInfo(slabAccount);
    
    if (!accountInfo) {
      // Generate dynamic mock data
      const { bids, asks, markPrice } = generateDynamicOrderbook(coin === 'BTC' ? 45000 : coin === 'ETH' ? 2500 : 186);
      const bestBid = bids[0]?.price || 0;
      const bestAsk = asks[0]?.price || 0;
      const spread = (bestBid && bestAsk) ? ((bestAsk - bestBid) / markPrice) * 100 : 0;
      
      return res.json({
        success: true,
        coin,
        slabAccount: slabAccount.toBase58(),
        programId: SLAB_PROGRAM_ID.toBase58(),
        orderbook: {
          bids,
          asks,
          midPrice: markPrice,
          spread,
          lastUpdate: Date.now()
        },
        recentTrades: mockRecentTrades,
        message: `No slab found for ${coin}-USD (showing live simulated data)`
      });
    }
    
    console.log('Slab account found:', accountInfo.data.length, 'bytes');
    
    // Parse the binary data to extract real order book using new parser
    const { bids, asks, markPrice, tickSize, lotSize } = parseSlabOrderbook(accountInfo.data);
    
    // Calculate mid price and spread
    const bestBid = bids.length > 0 ? bids[0].price : 0;
    const bestAsk = asks.length > 0 ? asks[0].price : 0;
    const midPrice = (bestBid && bestAsk) ? (bestBid + bestAsk) / 2 : markPrice;
    const spread = (bestBid && bestAsk) ? ((bestAsk - bestBid) / midPrice) * 100 : 0;
    
    // If no orders found, generate dynamic data
    const useMockData = bids.length === 0 && asks.length === 0;
    
    if (useMockData) {
      const { bids: dynamicBids, asks: dynamicAsks, markPrice: dynamicMark } = generateDynamicOrderbook(
        coin === 'BTC' ? 45000 : coin === 'ETH' ? 2500 : 186
      );
      const bestBid = dynamicBids[0]?.price || 0;
      const bestAsk = dynamicAsks[0]?.price || 0;
      const dynamicSpread = (bestBid && bestAsk) ? ((bestAsk - bestBid) / dynamicMark) * 100 : 0;
      
      res.json({
        success: true,
        coin,
        slabAccount: slabAccount.toBase58(),
        programId: SLAB_PROGRAM_ID.toBase58(),
        accountSize: accountInfo.data.length,
        lamports: accountInfo.lamports,
        owner: accountInfo.owner.toBase58(),
        orderbook: {
          bids: dynamicBids,
          asks: dynamicAsks,
          midPrice: dynamicMark,
          spread: dynamicSpread,
          lastUpdate: Date.now()
        },
        recentTrades: mockRecentTrades,
        message: `Live simulated orderbook for ${coin}-USD`
      });
    } else {
      res.json({
        success: true,
        coin,
        slabAccount: slabAccount.toBase58(),
        programId: SLAB_PROGRAM_ID.toBase58(),
        accountSize: accountInfo.data.length,
        lamports: accountInfo.lamports,
        owner: accountInfo.owner.toBase58(),
        slabConfig: {
          markPrice,
          tickSize,
          lotSize,
        },
        orderbook: {
          bids,
          asks,
          midPrice,
          spread,
          lastUpdate: Date.now()
        },
        recentTrades: mockRecentTrades,
        message: `✅ Real on-chain: ${bids.length} bids, ${asks.length} asks from ${coin}-USD slab`
      });
    }
    
  } catch (error: any) {
    console.error('Error fetching Slab orderbook:', error);
    res.status(500).json({ 
      success: false,
      error: error.message
    });
  }
});

/**
 * GET /api/slab/transactions
 * Get recent transactions for the Slab account
 */
slabRouter.get('/transactions', async (req: Request, res: Response) => {
  try {
    const connection = new Connection(
      process.env.SOLANA_RPC_URL || 'https://api.devnet.solana.com',
      'confirmed'
    );
    
    const limit = parseInt(req.query.limit as string) || 50;
    const walletAddress = req.query.wallet as string;
    
    // Fetch signatures for this account
    const signatures = await connection.getSignaturesForAddress(
      SLAB_ACCOUNT,
      { limit }
    );
    
    // Fetch transaction details to get actual wallet signers
    const transactionsWithSigners = await Promise.all(
      signatures.slice(0, Math.min(10, signatures.length)).map(async (sig) => {
        try {
          const txDetails = await connection.getTransaction(sig.signature, {
            commitment: 'confirmed',
            maxSupportedTransactionVersion: 0
          });
          
          let signer = '';
          if (txDetails) {
            const accountKeys = txDetails.transaction.message.getAccountKeys();
            if (accountKeys && accountKeys.length > 0) {
              signer = accountKeys.get(0)?.toBase58() || '';
            }
          }
          
          return {
            signature: sig.signature,
            slot: sig.slot,
            blockTime: sig.blockTime,
            err: sig.err,
            memo: sig.memo,
            signer: signer,
            solscanLink: `https://solscan.io/tx/${sig.signature}?cluster=devnet`
          };
        } catch (error: any) {
          console.error(`Failed to fetch signer for ${sig.signature.substring(0, 8)}:`, error.message);
          return {
            signature: sig.signature,
            slot: sig.slot,
            blockTime: sig.blockTime,
            err: sig.err,
            memo: sig.memo,
            signer: '',
            solscanLink: `https://solscan.io/tx/${sig.signature}?cluster=devnet`
          };
        }
      })
    );
    
    // Filter by wallet if provided
    const filteredTransactions = walletAddress
      ? transactionsWithSigners.filter(tx => tx.signer === walletAddress)
      : transactionsWithSigners;
    
    res.json({
      success: true,
      slabAccount: SLAB_ACCOUNT.toBase58(),
      transactions: filteredTransactions
    });
    
  } catch (error: any) {
    console.error('Error fetching Slab transactions:', error);
    res.status(500).json({ 
      success: false,
      error: error.message 
    });
  }
});

/**
 * GET /api/health
 * Health check endpoint
 */
slabRouter.get('/health', (req: Request, res: Response) => {
  res.json({
    success: true,
    status: 'online',
    timestamp: Date.now()
  });
});

