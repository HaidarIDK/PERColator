import { Router } from 'express';
import { getConnection } from '../services/solana';

export const healthRouter = Router();

healthRouter.get('/', async (req, res) => {
  try {
    const connection = getConnection();
    
    let solanaStatus;
    try {
      const slot = await connection.getSlot();
      const blockTime = await connection.getBlockTime(slot);
      solanaStatus = {
        network: process.env.SOLANA_NETWORK || 'localnet',
        rpc: process.env.SOLANA_RPC_URL || 'http://localhost:8899',
        slot,
        blockTime,
        latency_ms: blockTime ? Date.now() - blockTime * 1000 : null,
        connected: true,
      };
    } catch (error) {
      solanaStatus = {
        network: process.env.SOLANA_NETWORK || 'localnet',
        rpc: process.env.SOLANA_RPC_URL || 'http://localhost:8899',
        connected: false,
        note: 'API works with mock data (Solana RPC not required)',
      };
    }
    
    res.json({
      status: 'healthy',
      timestamp: Date.now(),
      solana: solanaStatus,
      api: {
        version: '1.0.0',
        uptime: process.uptime(),
        mode: solanaStatus.connected ? 'live' : 'mock',
      }
    });
  } catch (error: any) {
    res.status(503).json({
      status: 'degraded',
      error: error.message,
    });
  }
});

