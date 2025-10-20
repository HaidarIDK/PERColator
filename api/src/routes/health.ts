import { Router } from 'express';
import { getConnection } from '../services/solana';

export const healthRouter = Router();

healthRouter.get('/', async (req, res) => {
  try {
    const connection = getConnection();
    const slot = await connection.getSlot();
    const blockTime = await connection.getBlockTime(slot);
    
    res.json({
      status: 'healthy',
      timestamp: Date.now(),
      solana: {
        network: process.env.SOLANA_NETWORK,
        rpc: process.env.SOLANA_RPC_URL,
        slot,
        blockTime,
        latency_ms: Date.now() - (blockTime || 0) * 1000,
      },
      api: {
        version: '1.0.0',
        uptime: process.uptime(),
      }
    });
  } catch (error: any) {
    res.status(503).json({
      status: 'degraded',
      error: error.message,
    });
  }
});

