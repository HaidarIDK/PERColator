import { Router } from 'express';
import { PublicKey } from '@solana/web3.js';
import { fetchSlabState } from '../services/solana';

export const marketDataRouter = Router();

/**
 * GET /api/market/instruments
 * Get all available trading instruments
 */
marketDataRouter.get('/instruments', async (req, res) => {
  try {
    const { slab } = req.query;
    if (!slab) {
      return res.status(400).json({ error: 'slab address required' });
    }

    const slabAddress = new PublicKey(slab as string);
    const slabState = await fetchSlabState(slabAddress);
    
    // TODO: Parse instruments from slab state
    // For now, return mock data
    res.json([
      {
        index: 0,
        symbol: 'BTC/USDC',
        contract_size: 1,
        tick_size: 0.01,
        lot_size: 0.001,
        mark_price: 65000.00,
        last_price: 65005.50,
        volume_24h: 1234567,
        open_interest: 500,
        funding_rate: 0.0001,
        next_funding_ms: Date.now() + 3600000,
      },
      {
        index: 1,
        symbol: 'ETH/USDC',
        contract_size: 1,
        tick_size: 0.01,
        lot_size: 0.01,
        mark_price: 3200.00,
        last_price: 3201.25,
        volume_24h: 654321,
        open_interest: 800,
        funding_rate: 0.00008,
        next_funding_ms: Date.now() + 3600000,
      }
    ]);
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/market/orderbook
 * Get orderbook for an instrument
 */
marketDataRouter.get('/orderbook', async (req, res) => {
  try {
    const { slab, instrument } = req.query;
    if (!slab || !instrument) {
      return res.status(400).json({ error: 'slab and instrument required' });
    }

    const slabAddress = new PublicKey(slab as string);
    const instrumentIdx = parseInt(instrument as string);
    
    // TODO: Parse orderbook from slab state
    // For now, return mock orderbook
    res.json({
      instrument_idx: instrumentIdx,
      timestamp: Date.now(),
      bids: [
        { price: 64998.00, qty: 0.5, num_orders: 2 },
        { price: 64995.00, qty: 1.2, num_orders: 3 },
        { price: 64990.00, qty: 2.0, num_orders: 5 },
      ],
      asks: [
        { price: 65002.00, qty: 0.8, num_orders: 1 },
        { price: 65005.00, qty: 1.5, num_orders: 4 },
        { price: 65010.00, qty: 3.0, num_orders: 6 },
      ],
      spread: 4.00,
      mid: 65000.00,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/market/trades
 * Get recent trades for an instrument
 */
marketDataRouter.get('/trades', async (req, res) => {
  try {
    const { slab, instrument, limit = '50' } = req.query;
    if (!slab || !instrument) {
      return res.status(400).json({ error: 'slab and instrument required' });
    }

    const slabAddress = new PublicKey(slab as string);
    const instrumentIdx = parseInt(instrument as string);
    const maxTrades = Math.min(parseInt(limit as string), 1000);
    
    // TODO: Parse trades from slab state
    // For now, return mock trades
    const mockTrades = Array.from({ length: Math.min(maxTrades, 20) }, (_, i) => ({
      trade_id: 1000 - i,
      timestamp: Date.now() - i * 5000,
      price: 65000 + (Math.random() - 0.5) * 100,
      qty: Math.random() * 2,
      side: Math.random() > 0.5 ? 'buy' : 'sell',
    }));

    res.json(mockTrades);
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

/**
 * GET /api/market/stats
 * Get 24h market statistics
 */
marketDataRouter.get('/stats', async (req, res) => {
  try {
    const { slab } = req.query;
    if (!slab) {
      return res.status(400).json({ error: 'slab address required' });
    }

    res.json({
      volume_24h: 10234567,
      trades_24h: 5432,
      high_24h: 65500,
      low_24h: 64200,
      open_24h: 64800,
      last_price: 65005,
      price_change_24h: 205,
      price_change_pct_24h: 0.32,
    });
  } catch (error: any) {
    res.status(500).json({ error: error.message });
  }
});

