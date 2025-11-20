import express from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import { marketDataRouter } from './routes/marketData';
import { dashboardRouter } from './routes/dashboard';
import { tradingRouter } from './routes/trading';
import { userRouter } from './routes/user';
import { slabRouter } from './routes/slab';
import { healthRouter } from './routes/health';
import { routerRouter } from './routes/router';
import { claimsRouter } from './routes/claims';
import { faucetRouter } from './routes/faucet';
import { monitorRouter } from './routes/monitor';
import { cliRouter } from './routes/cli';
import { aiAssistantRouter } from './routes/ai-assistant';
import { initializeSolana } from './services/solana';
import { initializeWebSocketServer } from './services/websocket-server';
import WebSocket from 'ws';
import { cliWebSocketHandler } from './services/cli-websocket';

dotenv.config();

const app = express();
const PORT = process.env.PORT || 5001;
const HOST = process.env.HOST || '0.0.0.0';

// Middleware
// CORS - Allow frontend from both local and production
const allowedOrigins = [
  'http://localhost:5000',
  'http://localhost:3001',
  'http://localhost:3000',
  'https://percolator.site', // Production frontend URL
  'https://www.percolator.site', // WWW variant
  'https://dex.percolator.site', // Alternative production URL
  'https://www.dex.percolator.site', // WWW variant
  'https://frontend-1kpt12t6l-haidars-projects-a4025cbc.vercel.app', // Vercel deployment URL
  process.env.FRONTEND_URL, // Additional custom frontend URL
  'https://percolator-frontend.onrender.com', // Default Render URL
];

app.use(cors({
  origin: (origin, callback) => {
    // Allow requests with no origin (mobile apps, Postman, etc.)
    if (!origin) return callback(null, true);
    
    if (allowedOrigins.some(allowed => allowed && origin.startsWith(allowed))) {
      callback(null, true);
    } else {
      console.warn(`⚠️  CORS blocked request from: ${origin}`);
      callback(null, true); // Allow in development, can change to false in strict production
    }
  },
  credentials: true
}));
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
      faucet: '/api/faucet/*',
      monitor: '/api/monitor/*',
      cli: '/api/cli/*',
      aiAssistant: '/api/ai-assistant/*',
      hyperliquid: '/api/hyperliquid/*',
      websocket: 'ws://localhost:5001/ws'
    },
    docs: 'See api/README.md for full API documentation'
  });
});

// Routes
app.use('/api/health', healthRouter);
app.use('/api/market', dashboardRouter); // Dashboard API with real-time data
app.use('/api/dashboard', dashboardRouter); // Also expose under /api/dashboard for frontend
app.use('/api/slab', marketDataRouter); // Original Solana slab data
app.use('/api/slab-live', slabRouter); // LIVE on-chain Slab account data
app.use('/api/trade', tradingRouter);
app.use('/api/user', userRouter);
app.use('/api/router', routerRouter);
app.use('/api/claims', claimsRouter);
app.use('/api/faucet', faucetRouter);
app.use('/api/monitor', monitorRouter);
app.use('/api/cli', cliRouter);
app.use('/api/ai-assistant', aiAssistantRouter);

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
    const server = app.listen(Number(PORT), () => {
      console.log(`🚀 Percolator API server running on http://${HOST}:${PORT}`);
      console.log(`📊 Network: ${process.env.SOLANA_NETWORK}`);
      console.log(`🔗 RPC: ${process.env.SOLANA_RPC_URL}`);
    });

    // Start WebSocket server for market data
    initializeWebSocketServer(server);

    // Start CLI WebSocket server
    const cliWss = new WebSocket.Server({
      server,
      path: '/ws/cli',
    });

    cliWss.on('connection', (ws: WebSocket, req) => {
      console.log(`🔌 CLI WebSocket client connected from ${req.socket.remoteAddress}`);

      // Send welcome message
      ws.send(JSON.stringify({
        type: 'connected',
        message: 'Connected to Percolator CLI WebSocket',
        timestamp: Date.now(),
      }));

      // Handle messages
      ws.on('message', (data: Buffer) => {
        cliWebSocketHandler.handleMessage(ws, data);
      });

      // Handle disconnect
      ws.on('close', () => {
        console.log('🔌 CLI WebSocket client disconnected');
        cliWebSocketHandler.handleDisconnect(ws);
      });

      // Handle errors
      ws.on('error', (error) => {
        console.error('❌ CLI WebSocket error:', error);
        cliWebSocketHandler.handleDisconnect(ws);
      });
    });

    console.log('✅ CLI WebSocket server started on /ws/cli');

  } catch (error) {
    console.error('❌ Failed to start server:', error);
    process.exit(1);
  }
}

// Export the app for Vercel serverless functions
export default app;

// Only start the server if not in Vercel environment
if (process.env.VERCEL !== '1') {
  start();
}

