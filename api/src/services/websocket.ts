import { WebSocketServer, WebSocket } from 'ws';
import { Server } from 'http';

let wss: WebSocketServer;

interface Client {
  ws: WebSocket;
  subscriptions: Set<string>;
}

const clients = new Set<Client>();

export function startWebSocketServer(server: Server) {
  wss = new WebSocketServer({ server, path: '/ws' });

  wss.on('connection', (ws: WebSocket) => {
    console.log('🔌 New WebSocket client connected');
    
    const client: Client = {
      ws,
      subscriptions: new Set(),
    };
    clients.add(client);

    ws.on('message', (data: string) => {
      try {
        const msg = JSON.parse(data.toString());
        handleMessage(client, msg);
      } catch (error) {
        console.error('Invalid WebSocket message:', error);
        ws.send(JSON.stringify({ error: 'Invalid message format' }));
      }
    });

    ws.on('close', () => {
      console.log('🔌 Client disconnected');
      clients.delete(client);
    });

    // Send welcome message
    ws.send(JSON.stringify({
      type: 'connected',
      timestamp: Date.now(),
      message: 'Welcome to Percolator WebSocket API',
    }));
  });

  // Start mock data broadcaster (for testing)
  startMockDataBroadcast();
}

function handleMessage(client: Client, msg: any) {
  switch (msg.type) {
    case 'subscribe':
      handleSubscribe(client, msg);
      break;
    case 'unsubscribe':
      handleUnsubscribe(client, msg);
      break;
    case 'ping':
      client.ws.send(JSON.stringify({ type: 'pong', timestamp: Date.now() }));
      break;
    default:
      client.ws.send(JSON.stringify({ error: 'Unknown message type' }));
  }
}

function handleSubscribe(client: Client, msg: any) {
  const { channel } = msg;
  if (!channel) {
    client.ws.send(JSON.stringify({ error: 'Channel required' }));
    return;
  }

  client.subscriptions.add(channel);
  client.ws.send(JSON.stringify({
    type: 'subscribed',
    channel,
    timestamp: Date.now(),
  }));
  
  console.log(`Client subscribed to: ${channel}`);
}

function handleUnsubscribe(client: Client, msg: any) {
  const { channel } = msg;
  if (!channel) {
    client.ws.send(JSON.stringify({ error: 'Channel required' }));
    return;
  }

  client.subscriptions.delete(channel);
  client.ws.send(JSON.stringify({
    type: 'unsubscribed',
    channel,
    timestamp: Date.now(),
  }));
}

export function broadcast(channel: string, data: any) {
  const message = JSON.stringify({
    type: 'update',
    channel,
    data,
    timestamp: Date.now(),
  });

  clients.forEach(client => {
    if (client.subscriptions.has(channel) && client.ws.readyState === WebSocket.OPEN) {
      client.ws.send(message);
    }
  });
}

// Mock data broadcaster for testing
function startMockDataBroadcast() {
  // Broadcast mock orderbook updates every 2 seconds
  setInterval(() => {
    broadcast('orderbook:BTC/USDC', {
      bids: [
        { price: 65000 + Math.random() * 10, qty: Math.random() * 2 },
        { price: 64995 + Math.random() * 10, qty: Math.random() * 3 },
      ],
      asks: [
        { price: 65005 + Math.random() * 10, qty: Math.random() * 2 },
        { price: 65010 + Math.random() * 10, qty: Math.random() * 3 },
      ],
    });
  }, 2000);

  // Broadcast mock trades every 3 seconds
  setInterval(() => {
    broadcast('trades:BTC/USDC', {
      trade_id: Math.floor(Math.random() * 1000000),
      price: 65000 + (Math.random() - 0.5) * 100,
      qty: Math.random() * 2,
      side: Math.random() > 0.5 ? 'buy' : 'sell',
    });
  }, 3000);
}

