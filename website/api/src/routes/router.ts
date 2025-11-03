import { Router } from 'express';
import { PublicKey, Transaction, Connection, clusterApiUrl, SystemProgram } from '@solana/web3.js';
import { 
  buildDepositInstruction,
  buildWithdrawInstruction,
  buildExecuteCrossSlabInstruction,
  derivePortfolioPDA,
  getPortfolio,
  ROUTER_PROGRAM_ID,
  SLAB_ACCOUNT
} from '../services/router';
import { getConnection } from '../services/solana';
import { serializeTransaction } from '../services/transactions';

export const routerRouter = Router();

/**
 * POST /api/router/deposit
 * Deposit SOL collateral to Router portfolio
 * Returns an unsigned transaction for the user to sign
 */
routerRouter.post('/deposit', async (req, res) => {
  try {
    const { wallet, amount } = req.body;
    
    if (!wallet || !amount) {
      return res.status(400).json({ 
        success: false,
        error: 'Missing required fields: wallet, amount' 
      });
    }

    // Parse user public key
    let userPubkey: PublicKey;
    try {
      userPubkey = new PublicKey(wallet);
    } catch (error) {
      return res.status(400).json({ 
        success: false,
        error: 'Invalid wallet address' 
      });
    }

    // Convert amount to lamports (1 SOL = 1e9 lamports)
    const amountLamports = Math.floor(parseFloat(amount) * 1e9);

    console.log(`💰 Building deposit transaction for ${wallet}`);
    console.log(`   Amount: ${amount} SOL (${amountLamports} lamports)`);

    // Build deposit instruction
    // Derive Portfolio PDA
    const [portfolioPDA, portfolioBump] = derivePortfolioPDA(userPubkey);
    
    console.log(`📋 Portfolio PDA: ${portfolioPDA.toBase58()}`);
    console.log(`   Bump: ${portfolioBump}`);
    console.log(`   User: ${userPubkey.toBase58()}`);
    console.log(`   Router Program: ${ROUTER_PROGRAM_ID.toBase58()}`);
    
    // Check if portfolio exists
    const connection = getConnection();
    const portfolioAccountInfo = await connection.getAccountInfo(portfolioPDA);
    
    const transaction = new Transaction();
    
    // If portfolio doesn't exist, user needs to initialize it first
    if (!portfolioAccountInfo) {
      return res.status(400).json({
        success: false,
        error: 'Portfolio not initialized. Click "Initialize New Slab" button first to set up trading.',
        portfolioAddress: portfolioPDA.toBase58(),
        needsInit: true,
      });
    }
    
    console.log(`📋 Portfolio account exists: ${portfolioPDA.toBase58()}`);
    console.log(`   Owner: ${portfolioAccountInfo.owner.toBase58()}`);
    console.log(`   Lamports: ${portfolioAccountInfo.lamports}`);

    const depositIx = buildDepositInstruction({
      userAuthority: userPubkey,
      amount: amountLamports,
    });

    transaction.add(depositIx);

    // Get recent blockhash
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('finalized');
    transaction.recentBlockhash = blockhash;
    transaction.lastValidBlockHeight = lastValidBlockHeight;
    transaction.feePayer = userPubkey;

    // Serialize for frontend
    const serializedTx = serializeTransaction(transaction);

    console.log(`✅ Deposit transaction built`);
    console.log(`   Portfolio PDA: ${portfolioPDA.toBase58()}`);

    res.json({
      success: true,
      needsSigning: true,
      transaction: serializedTx,
      portfolioAddress: portfolioPDA.toBase58(),
      amount: parseFloat(amount),
      amountLamports,
      message: 'Sign this transaction to deposit SOL collateral into your trading portfolio',
    });
  } catch (error: any) {
    console.error('❌ Deposit error:', error);
    res.status(500).json({ 
      success: false,
      error: error.message 
    });
  }
});

/**
 * POST /api/router/withdraw
 * Withdraw collateral from router vault
 */
routerRouter.post('/withdraw', async (req, res) => {
  try {
    const { user, mint, amount } = req.body;
    
    if (!user || !mint || !amount) {
      return res.status(400).json({ error: 'user, mint, and amount required' });
    }

    // TODO: Build and send withdraw transaction
    res.json({
      success: true,
      user,
      mint,
      amount: parseFloat(amount),
      vault_balance: 10000 - parseFloat(amount),
      timestamp: Date.now(),
      signature: 'MockWithdrawSignature' + Math.random().toString(36).substring(7),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/router/portfolio/:wallet
 * Get cross-slab portfolio for user from Router program
 */
routerRouter.get('/portfolio/:wallet', async (req, res) => {
  try {
    const { wallet } = req.params;
    
    if (!wallet) {
      return res.status(400).json({ 
        success: false,
        error: 'wallet address required' 
      });
    }

    // Parse wallet address
    let userPubkey: PublicKey;
    try {
      userPubkey = new PublicKey(wallet);
    } catch (error) {
      return res.status(400).json({ 
        success: false,
        error: 'Invalid wallet address' 
      });
    }

    console.log(`📊 Fetching portfolio for ${wallet}`);

    // Get portfolio from Router program
    const connection = getConnection();
    const portfolio = await getPortfolio(connection, userPubkey);

    if (!portfolio || !portfolio.exists) {
      console.log(`   Portfolio doesn't exist yet`);
      return res.json({
        success: true,
        exists: false,
        wallet,
        portfolioAddress: derivePortfolioPDA(userPubkey)[0].toBase58(),
        message: 'Portfolio not created yet. Make a deposit to create your portfolio.',
        equity: 0,
        collateral: 0,
        positions: [],
      });
    }

    console.log(`✅ Portfolio found: ${portfolio.address}`);
    console.log(`   Lamports: ${portfolio.lamports}`);

    // Return portfolio data
    res.json({
      success: true,
      exists: true,
      wallet,
      portfolioAddress: portfolio.address,
      equity: portfolio.lamports / 1e9, // Convert to SOL
      collateral: portfolio.lamports / 1e9,
      im: 0, // TODO: Calculate from positions
      mm: 0,
      free_collateral: portfolio.lamports / 1e9,
      positions: [], // TODO: Parse position data
      leverage: 1.0,
      health: 100,
    });
  } catch (error: any) {
    console.error('❌ Portfolio fetch error:', error);
    res.status(500).json({ 
      success: false,
      error: error.message 
    });
  }
});

/**
 * GET /api/router/slabs
 * Get all registered slabs with current liquidity and pricing
 * Query params: coin (ethereum, bitcoin, solana)
 */
routerRouter.get('/slabs', async (req, res) => {
  try {
    const { coin } = req.query;
    
    // Get base price for the selected coin
    const getBasePrice = (coinType: string) => {
      switch(coinType) {
        case 'ethereum': return 3882;   // ETH/USDC
        case 'bitcoin': return 97500;   // BTC/USDC
        case 'solana': return 185;      // SOL/USDC
        default: return 3882;
      }
    };

    const basePrice = getBasePrice(coin as string || 'ethereum');
    
    // TODO: Fetch real slab data from on-chain state
    // For now, return mock slabs with coin-specific pricing
    
    res.json({
      slabs: [
        {
          id: 1,
          name: "Slab A",
          slab_id: 'Slab1111111111111111111111111111111',
          liquidity: 1500, // Available liquidity in base units
          vwap: basePrice * 1.00005, // Slightly above market
          fee: 0.02, // Fee percentage (0.02 = 2%)
          instruments: ['BTC/USDC', 'ETH/USDC', 'SOL/USDC'],
          imr: 500, // Initial margin ratio (bps)
          mmr: 250, // Maintenance margin ratio (bps)
          active: true,
          volume_24h: 1234567,
        },
        {
          id: 2,
          name: "Slab B",
          slab_id: 'Slab2222222222222222222222222222222',
          liquidity: 2300,
          vwap: basePrice * 1.00008, // Slightly higher
          fee: 0.015, // 1.5% fee
          instruments: ['BTC/USDC', 'ETH/USDC'],
          imr: 600,
          mmr: 300,
          active: true,
          volume_24h: 654321,
        },
        {
          id: 3,
          name: "Slab C",
          slab_id: 'Slab3333333333333333333333333333333',
          liquidity: 980,
          vwap: basePrice * 0.99995, // Best price (slightly below market)
          fee: 0.025, // 2.5% fee
          instruments: ['ETH/USDC', 'SOL/USDC'],
          imr: 500,
          mmr: 250,
          active: true,
          volume_24h: 456789,
        }
      ],
      coin: coin || 'ethereum',
      basePrice,
      timestamp: Date.now(),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/router/reserve-multi
 * Reserve across multiple slabs
 */
routerRouter.post('/reserve-multi', async (req, res) => {
  try {
    const { user, instrument, side, qty, limit_px, slabs } = req.body;
    
    if (!user || instrument === undefined || !side || !qty) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // TODO: Call reserve on multiple slabs and optimize routing
    res.json({
      success: true,
      route_id: Math.floor(Math.random() * 1000000),
      reservations: [
        {
          slab_id: slabs?.[0] || 'Slab1...',
          hold_id: Math.floor(Math.random() * 1000000),
          qty: parseFloat(qty) * 0.7,
          vwap: parseFloat(limit_px) || 65000,
          max_charge: parseFloat(qty) * 0.7 * 65000,
        },
        {
          slab_id: slabs?.[1] || 'Slab2...',
          hold_id: Math.floor(Math.random() * 1000000),
          qty: parseFloat(qty) * 0.3,
          vwap: parseFloat(limit_px) || 65005,
          max_charge: parseFloat(qty) * 0.3 * 65005,
        }
      ],
      total_qty: parseFloat(qty),
      blended_vwap: 65002,
      total_cost: parseFloat(qty) * 65002,
      timestamp: Date.now(),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/router/commit-multi
 * Commit multi-slab reservation
 */
routerRouter.post('/commit-multi', async (req, res) => {
  try {
    const { route_id } = req.body;
    
    if (!route_id) {
      return res.status(400).json({ error: 'route_id required' });
    }

    // TODO: Atomic commit across slabs
    res.json({
      success: true,
      route_id,
      fills: [
        { slab: 'Slab1...', qty: 0.7, price: 65000, fee: 22.75 },
        { slab: 'Slab2...', qty: 0.3, price: 65005, fee: 9.75 },
      ],
      total_qty: 1.0,
      blended_vwap: 65002,
      total_fee: 32.50,
      timestamp: Date.now(),
      signatures: [
        'MockCommitSig1' + Math.random().toString(36).substring(7),
        'MockCommitSig2' + Math.random().toString(36).substring(7),
      ]
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/router/execute-cross-slab
 * ARCHITECTURE FLOW:
 * 1. Frontend calls this endpoint
 * 2. Backend/SDK builds ExecuteCrossSlab instruction
 * 3. Returns serialized transaction
 * 4. Frontend signs and submits → Router Program
 * 5. Router Program CPIs to Slab Program
 * 6. Portfolio updated with net exposure
 */
routerRouter.post('/execute-cross-slab', async (req, res) => {
  try {
    const { wallet, side, instrumentIdx, quantity, limitPrice } = req.body;
    
    if (!wallet || !side || quantity === undefined) {
      return res.status(400).json({ 
        success: false,
        error: 'Missing required fields: wallet, side, quantity' 
      });
    }

    // Parse user public key
    let userPubkey: PublicKey;
    try {
      userPubkey = new PublicKey(wallet);
    } catch (error) {
      return res.status(400).json({ 
        success: false,
        error: 'Invalid wallet address' 
      });
    }

    console.log(`🎯 Building ExecuteCrossSlab transaction for ${wallet}`);
    console.log(`   Side: ${side}`);
    console.log(`   Quantity: ${quantity}`);
    console.log(`   Price: ${limitPrice}`);
    console.log(`   Instrument: ${instrumentIdx || 0}`);

    // Build execute instruction
    const executeIx = buildExecuteCrossSlabInstruction({
      userAuthority: userPubkey,
      slabAccount: SLAB_ACCOUNT,
      instrumentIdx: instrumentIdx || 0,
      side,
      quantity: parseFloat(quantity),
      limitPrice: parseFloat(limitPrice),
    });

    // Create transaction
    const transaction = new Transaction();
    transaction.add(executeIx);

    // Get recent blockhash
    const connection = getConnection();
    const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('finalized');
    transaction.recentBlockhash = blockhash;
    transaction.lastValidBlockHeight = lastValidBlockHeight;
    transaction.feePayer = userPubkey;

    // Serialize for frontend
    const serializedTx = serializeTransaction(transaction);

    const [portfolioPDA] = derivePortfolioPDA(userPubkey);

    console.log(`✅ ExecuteCrossSlab transaction built`);
    console.log(`   Portfolio: ${portfolioPDA.toBase58()}`);
    console.log(`   Slab: ${SLAB_ACCOUNT.toBase58()}`);

    res.json({
      success: true,
      needsSigning: true,
      transaction: serializedTx,
      portfolioAddress: portfolioPDA.toBase58(),
      slabAccount: SLAB_ACCOUNT.toBase58(),
      routeId: Date.now(),
      side,
      quantity: parseFloat(quantity),
      limitPrice: parseFloat(limitPrice),
      estimatedFees: {
        protocol: parseFloat(quantity) * parseFloat(limitPrice) * 0.0002, // 0.02%
        network: 0.000005 // SOL
      },
      message: `Sign to execute ${side} order via Router → Slab CPI`,
    });
  } catch (error: any) {
    console.error('❌ SDK Error:', error);
    res.status(500).json({ 
      success: false,
      error: error.message 
    });
  }
});

/**
 * GET /api/router/vault
 * Get vault balance for a mint
 */
routerRouter.get('/vault/:mint', async (req, res) => {
  try {
    const { mint } = req.params;
    
    // TODO: Fetch vault state
    res.json({
      mint,
      balance: 1000000.00,
      pledged: 250000.00,
      available: 750000.00,
      num_users: 42,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

