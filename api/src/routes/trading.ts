import { Router } from 'express';
import { PublicKey } from '@solana/web3.js';

export const tradingRouter = Router();

/**
 * POST /api/trade/order
 * Place a new order (simplified single-phase for MVP)
 */
tradingRouter.post('/order', async (req, res) => {
  try {
    const { slab, user, instrument, side, price, qty, order_type = 'limit' } = req.body;
    
    if (!slab || !user || instrument === undefined || !side || !price || !qty) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // TODO: Build and send transaction to place order
    // For MVP, return mock response
    res.json({
      success: true,
      order_id: Math.floor(Math.random() * 1000000),
      status: 'pending',
      slab,
      user,
      instrument,
      side,
      price: parseFloat(price),
      qty: parseFloat(qty),
      filled_qty: 0,
      timestamp: Date.now(),
      signature: 'MockSignature' + Math.random().toString(36).substring(7),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/cancel
 * Cancel an existing order
 */
tradingRouter.post('/cancel', async (req, res) => {
  try {
    const { slab, user, order_id } = req.body;
    
    if (!slab || !user || !order_id) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // TODO: Build and send cancel transaction
    res.json({
      success: true,
      order_id,
      status: 'cancelled',
      timestamp: Date.now(),
      signature: 'MockCancelSignature' + Math.random().toString(36).substring(7),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/reserve
 * Reserve liquidity (two-phase execution, step 1)
 */
tradingRouter.post('/reserve', async (req, res) => {
  try {
    const { slab, user, instrument, side, qty, limit_px, ttl_ms = 60000 } = req.body;
    
    if (!slab || !user || instrument === undefined || !side || !qty) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // TODO: Call slab reserve instruction
    res.json({
      success: true,
      hold_id: Math.floor(Math.random() * 1000000),
      vwap_px: parseFloat(limit_px) || 65000,
      worst_px: parseFloat(limit_px) || 65005,
      max_charge: parseFloat(qty) * (parseFloat(limit_px) || 65000),
      expiry_ms: Date.now() + parseInt(ttl_ms as string),
      reserved_qty: parseFloat(qty),
      timestamp: Date.now(),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * POST /api/trade/commit
 * Commit a reservation (two-phase execution, step 2)
 */
tradingRouter.post('/commit', async (req, res) => {
  try {
    const { slab, hold_id } = req.body;
    
    if (!slab || !hold_id) {
      return res.status(400).json({ error: 'Missing required fields' });
    }

    // TODO: Call slab commit instruction
    res.json({
      success: true,
      hold_id,
      fills: [
        { price: 65000, qty: 0.5, fee: 3.25 },
        { price: 65002, qty: 0.3, fee: 1.95 },
      ],
      total_qty: 0.8,
      vwap: 65000.75,
      total_fee: 5.20,
      timestamp: Date.now(),
      signature: 'MockCommitSignature' + Math.random().toString(36).substring(7),
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

