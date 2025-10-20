import express from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import { marketDataRouter } from './routes/marketData';
import { tradingRouter } from './routes/trading';
import { userRouter } from './routes/user';
import { healthRouter } from './routes/health';
import { routerRouter } from './routes/router';
import { claimsRouter } from './routes/claims';
import { initializeSolana } from './services/solana';
import { startWebSocketServer } from './services/websocket';

dotenv.config();

const app = express();
const PORT = process.env.PORT || 3000;
const HOST = process.env.HOST || '0.0.0.0';

// Middleware
app.use(cors());
app.use(express.json());

// Request logging
app.use((req, res, next) => {
  console.log(`[${new Date().toISOString()}] ${req.method} ${req.path}`);
  next();
});

// Root endpoint
app.get('/', (req, res) => {
  res.json({
    name: 'PERColator API',
    version: '1.0.0',
    status: 'running',
    endpoints: {
      health: '/api/health',
      market: '/api/market/*',
      trading: '/api/trade/*',
      user: '/api/user/*',
      router: '/api/router/*',
      claims: '/api/claims/*',
      websocket: 'ws://localhost:3000/ws'
    },
    docs: 'See api/README.md for full API documentation'
  });
});

// Routes
app.use('/api/health', healthRouter);
app.use('/api/market', marketDataRouter);
app.use('/api/trade', tradingRouter);
app.use('/api/user', userRouter);
app.use('/api/router', routerRouter);
app.use('/api/claims', claimsRouter);

// 404 handler for unknown routes
app.use((req, res) => {
  res.status(404).json({
    error: 'Route not found',
    path: req.path,
    method: req.method,
    hint: 'See / for available endpoints'
  });
});

// Error handling
app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
  console.error('Error:', err);
  res.status(err.status || 500).json({
    error: err.message || 'Internal server error',
    ...(process.env.NODE_ENV === 'development' && { stack: err.stack })
  });
});

// Start server
async function start() {
  try {
    // Initialize Solana connection
    await initializeSolana();
    console.log('✅ Solana connection initialized');

    // Start HTTP server
    const server = app.listen(PORT, HOST as any, () => {
      console.log(`🚀 Percolator API server running on http://${HOST}:${PORT}`);
      console.log(`📊 Network: ${process.env.SOLANA_NETWORK}`);
      console.log(`🔗 RPC: ${process.env.SOLANA_RPC_URL}`);
    });

    // Start WebSocket server
    startWebSocketServer(server);
    console.log('🔌 WebSocket server started');

  } catch (error) {
    console.error('❌ Failed to start server:', error);
    process.exit(1);
  }
}

start();

