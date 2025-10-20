import { Router } from 'express';
import { PublicKey } from '@solana/web3.js';

export const userRouter = Router();

/**
 * GET /api/user/balance
 * Get user's balance in the slab
 */
userRouter.get('/balance', async (req, res) => {
  try {
    const { slab, user } = req.query;
    
    if (!slab || !user) {
      return res.status(400).json({ error: 'slab and user address required' });
    }

    const slabAddress = new PublicKey(slab as string);
    const userAddress = new PublicKey(user as string);
    
    // TODO: Fetch user account from slab state
    res.json({
      user: userAddress.toBase58(),
      slab: slabAddress.toBase58(),
      balance: 10000.00,
      available: 8500.00,
      reserved: 1500.00,
      pnl_unrealized: 250.50,
      pnl_realized: 1234.00,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/user/positions
 * Get user's open positions
 */
userRouter.get('/positions', async (req, res) => {
  try {
    const { slab, user } = req.query;
    
    if (!slab || !user) {
      return res.status(400).json({ error: 'slab and user address required' });
    }

    // TODO: Fetch positions from slab state
    res.json([
      {
        instrument: 0,
        symbol: 'BTC/USDC',
        qty: 0.5,
        side: 'long',
        entry_price: 64500,
        mark_price: 65000,
        pnl: 250.00,
        pnl_pct: 0.78,
      },
    ]);
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/user/orders
 * Get user's open orders
 */
userRouter.get('/orders', async (req, res) => {
  try {
    const { slab, user, status = 'open' } = req.query;
    
    if (!slab || !user) {
      return res.status(400).json({ error: 'slab and user address required' });
    }

    // TODO: Fetch orders from slab state
    res.json([
      {
        order_id: 123456,
        instrument: 0,
        symbol: 'BTC/USDC',
        side: 'buy',
        price: 64900,
        qty: 1.0,
        filled_qty: 0,
        status: 'live',
        created_at: Date.now() - 300000,
      },
    ]);
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/user/trades
 * Get user's trade history
 */
userRouter.get('/trades', async (req, res) => {
  try {
    const { slab, user, limit = '50' } = req.query;
    
    if (!slab || !user) {
      return res.status(400).json({ error: 'slab and user address required' });
    }

    const maxTrades = Math.min(parseInt(limit as string), 1000);
    
    // TODO: Fetch trades from slab state
    res.json([
      {
        trade_id: 789012,
        order_id: 123455,
        instrument: 0,
        symbol: 'BTC/USDC',
        side: 'buy',
        price: 64800,
        qty: 0.5,
        fee: 16.20,
        role: 'taker',
        timestamp: Date.now() - 3600000,
        signature: 'MockTradeSignature123',
      },
    ]);
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/user/portfolio
 * Get user's portfolio summary
 */
userRouter.get('/portfolio', async (req, res) => {
  try {
    const { slab, user } = req.query;
    
    if (!slab || !user) {
      return res.status(400).json({ error: 'slab and user address required' });
    }

    // TODO: Calculate from slab state
    res.json({
      user: user,
      total_value: 11484.50,
      cash: 10000.00,
      positions_value: 1484.50,
      pnl_unrealized: 250.50,
      pnl_realized_24h: 34.00,
      pnl_realized_total: 1234.00,
      num_positions: 1,
      num_open_orders: 1,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

