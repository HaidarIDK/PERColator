import express from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import { hyperliquidRouter } from './routes/hyperliquid';
import { slabRouter } from './routes/slab';
import aiRouter from './routes/ai';
import { initializeWebSocketServer } from './services/websocket-server';

dotenv.config();

const app = express();
const PORT = process.env.PORT || 5001;
const HOST = process.env.HOST || '0.0.0.0';

// CORS - Allow frontend
const allowedOrigins = [
  'http://localhost:5000',
  'http://localhost:3000',
  process.env.FRONTEND_URL,
].filter(Boolean);

app.use(cors({
  origin: (origin, callback) => {
    if (!origin || allowedOrigins.includes(origin)) {
      callback(null, true);
    } else {
      console.warn(`CORS blocked: ${origin}`);
      callback(null, true); // Allow in dev
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
    name: 'Percolator API',
    version: '1.0.0',
    status: 'running',
    endpoints: {
      hyperliquid: '/api/hyperliquid/*',
      slab: '/api/slab/*',
      ai: '/api/ai/*',
      health: '/api/health',
      websocket: 'ws://localhost:5001/ws'
    }
  });
});

// Health check endpoint
app.get('/api/health', (req, res) => {
  res.json({
    success: true,
    status: 'online',
    timestamp: Date.now()
  });
});

// Routes
app.use('/api/hyperliquid', hyperliquidRouter);
app.use('/api/slab', slabRouter);
app.use('/api/ai', aiRouter);

// 404 handler
app.use((req, res) => {
  res.status(404).json({
    error: 'Route not found',
    path: req.path
  });
});

// Error handling
app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
  console.error('Error:', err);
  res.status(err.status || 500).json({
    error: err.message || 'Internal server error'
  });
});

// Start server
async function start() {
  try {
    const server = app.listen(Number(PORT), () => {
      console.log(`Percolator API running on http://${HOST}:${PORT}`);
    });

    // Start WebSocket server for real-time updates
    initializeWebSocketServer(server);

  } catch (error) {
    console.error('Failed to start server:', error);
    process.exit(1);
  }
}

start();

