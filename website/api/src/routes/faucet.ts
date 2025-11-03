import { Router } from 'express';
import { PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { getConnection } from '../services/solana';

export const faucetRouter = Router();

/**
 * POST /api/faucet/airdrop
 * Airdrop devnet SOL to user wallet (simplified endpoint)
 */
faucetRouter.post('/airdrop', async (req, res) => {
  try {
    const { wallet, amount = 2 } = req.body;
    
    if (!wallet) {
      return res.status(400).json({ error: 'wallet address required' });
    }

    const userPubkey = new PublicKey(wallet);
    const connection = getConnection();
    
    try {
      // Request airdrop from Solana devnet
      const amountLamports = amount * LAMPORTS_PER_SOL;
      console.log(`💰 Requesting ${amount} SOL airdrop for ${wallet}...`);
      
      const signature = await connection.requestAirdrop(userPubkey, amountLamports);
      console.log(`⏳ Airdrop signature: ${signature}`);
      
      // Wait for confirmation
      await connection.confirmTransaction(signature, 'confirmed');
      console.log(`✅ Airdrop confirmed!`);

      // Get updated balance
      const balance = await connection.getBalance(userPubkey);
      const balanceSOL = balance / LAMPORTS_PER_SOL;

      res.json({
        success: true,
        wallet,
        amount,
        signature,
        balance_after: balanceSOL,
        message: `Airdropped ${amount} SOL to ${wallet}`,
        explorer_url: `https://explorer.solana.com/tx/${signature}?cluster=devnet`
      });
    } catch (airdropError: any) {
      console.error('Airdrop error:', airdropError);
      
      // If airdrop fails (rate limit, RPC not available, etc)
      if (airdropError.message?.includes('airdrop') || airdropError.message?.includes('rate')) {
        return res.status(429).json({
          success: false,
          error: 'Airdrop rate limit exceeded. Please wait 1 minute and try again.',
          hint: 'You can also use https://faucet.solana.com for manual airdrops'
        });
      }
      
      // RPC not available
      return res.status(503).json({
        success: false,
        error: 'Solana RPC not available. Cannot process airdrop.',
        hint: 'Use https://faucet.solana.com to get devnet SOL manually'
      });
    }

  } catch (error: any) {
    res.status(500).json({ 
      success: false,
      error: error.message 
    });
  }
});

/**
 * POST /api/faucet/airdrop-sol
 * Airdrop devnet SOL to user wallet (original endpoint)
 */
faucetRouter.post('/airdrop-sol', async (req, res) => {
  try {
    const { user, amount = 2 } = req.body;
    
    if (!user) {
      return res.status(400).json({ error: 'user wallet address required' });
    }

    const userPubkey = new PublicKey(user);
    const connection = getConnection();
    
    // Check if we're on devnet
    const network = process.env.SOLANA_NETWORK;
    if (network !== 'devnet' && network !== 'localnet') {
      return res.status(403).json({ 
        error: 'Faucet only available on devnet/localnet',
        network: network || 'unknown'
      });
    }

    try {
      // Request airdrop
      const amountLamports = amount * LAMPORTS_PER_SOL;
      const signature = await connection.requestAirdrop(userPubkey, amountLamports);
      
      // Wait for confirmation
      await connection.confirmTransaction(signature, 'confirmed');

      res.json({
        success: true,
        user,
        amount,
        signature,
        message: `Airdropped ${amount} SOL to ${user}`,
        balance_after: amount, // Simplified, could fetch actual balance
      });
    } catch (error: any) {
      // If airdrop fails (rate limit, etc), return helpful error
      if (error.message?.includes('airdrop')) {
        return res.status(429).json({
          error: 'Airdrop rate limit exceeded. Try again in 1 minute.',
          hint: 'Use https://faucet.solana.com for manual airdrop'
        });
      }
      throw error;
    }

  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/faucet/balance/:user
 * Get user's SOL balance
 */
faucetRouter.get('/balance/:user', async (req, res) => {
  try {
    const { user } = req.params;
    const userPubkey = new PublicKey(user);
    const connection = getConnection();
    
    try {
      const balance = await connection.getBalance(userPubkey);
      const balanceSOL = balance / LAMPORTS_PER_SOL;

      res.json({
        user,
        balance: balanceSOL,
        balance_lamports: balance,
        network: process.env.SOLANA_NETWORK || 'localnet',
      });
    } catch (error) {
      // If RPC not available, return mock
      res.json({
        user,
        balance: 2.0,
        balance_lamports: 2 * LAMPORTS_PER_SOL,
        network: 'mock',
        note: 'Solana RPC not available, showing mock balance'
      });
    }

  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/faucet/info
 * Get faucet configuration and limits
 */
faucetRouter.get('/info', async (req, res) => {
  res.json({
    network: process.env.SOLANA_NETWORK || 'localnet',
    available: process.env.SOLANA_NETWORK === 'devnet' || process.env.SOLANA_NETWORK === 'localnet',
    airdrop_amount: 2,
    rate_limit: '1 airdrop per minute per wallet',
    collateral_token: 'SOL',
    note: 'Users trade with devnet SOL directly - no token swap needed',
    external_faucet: 'https://faucet.solana.com',
  });
});

