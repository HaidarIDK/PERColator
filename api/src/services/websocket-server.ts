import WebSocket from 'ws';
import { Server } from 'http';
import { hyperliquidWS, ProcessedCandle } from './hyperliquid-websocket';

// Types for our WebSocket server
export interface ClientSubscription {
  symbol: string;
  interval: string;
  subscriptionId: string;
}

export interface ClientMessage {
  type?: 'subscribe' | 'unsubscribe';
  method?: 'subscribe' | 'unsubscribe';
  symbol?: string;
  interval?: string;
  subscription?: {
    type: 'candle';
    coin: string;
    interval: string;
  };
}

export interface ServerMessage {
  type: 'candle' | 'subscription' | 'error';
  subscriptionId?: string;
  data?: ProcessedCandle;
  error?: string;
}

export class WebSocketServer {
  private wss: WebSocket.Server | null = null;
  private clients: Map<WebSocket, Set<string>> = new Map(); // client -> subscription IDs
  private subscriptions: Map<string, Set<WebSocket>> = new Map(); // subscription ID -> clients
  private hyperliquidSubscriptions: Map<string, string> = new Map(); // subscription ID -> hyperliquid subscription ID

  constructor(private server: Server) {}

  /**
   * Start the WebSocket server
   */
  public start(): void {
    console.log('🚀 Starting WebSocket server...');
    
    this.wss = new WebSocket.Server({ 
      server: this.server,
      path: '/ws'
    });

    this.wss.on('connection', (ws: WebSocket, req) => {
      console.log(`🔌 New WebSocket client connected from ${req.socket.remoteAddress}`);
      
      // Initialize client subscriptions
      this.clients.set(ws, new Set());
      
      // Handle client messages
      ws.on('message', (data: Buffer) => {
        this.handleClientMessage(ws, data);
      });

      // Handle client disconnect
      ws.on('close', (code: number, reason: Buffer) => {
        console.log(`🔌 Client disconnected. Code: ${code}, Reason: ${reason.toString()}`);
        this.handleClientDisconnect(ws);
      });

      // Handle client errors
      ws.on('error', (error: Error) => {
        console.error('❌ WebSocket client error:', error);
        this.handleClientDisconnect(ws);
      });

      // Send welcome message
      this.sendToClient(ws, {
        type: 'subscription',
        subscriptionId: 'welcome',
        data: {
          symbol: 'SYSTEM',
          timeframe: 'welcome',
          timestamp: Date.now(),
          open: 0,
          high: 0,
          low: 0,
          close: 0,
          volume: 0,
          trades: 0,
          priceChange: 0,
          priceChangePercent: 0
        }
      });
    });

    // Set up Hyperliquid WebSocket event listeners
    this.setupHyperliquidListeners();

    console.log('✅ WebSocket server started on /ws');
  }

  /**
   * Stop the WebSocket server
   */
  public stop(): void {
    if (this.wss) {
      console.log('🛑 Stopping WebSocket server...');
      this.wss.close();
      this.wss = null;
      
      // Clear all subscriptions
      this.clients.clear();
      this.subscriptions.clear();
      this.hyperliquidSubscriptions.clear();
      
      console.log('✅ WebSocket server stopped');
    }
  }

  /**
   * Handle incoming messages from clients
   */
  private handleClientMessage(ws: WebSocket, data: Buffer): void {
    try {
      const message: ClientMessage = JSON.parse(data.toString());
      console.log(`📨 Received from client:`, message);

      // Extract message type and parameters
      let messageType: string;
      let symbol: string;
      let interval: string;

      // Handle Hyperliquid-style messages (method + subscription)
      if (message.method && message.subscription) {
        messageType = message.method;
        symbol = message.subscription.coin;
        interval = message.subscription.interval;
      }
      // Handle our custom format (type + symbol/interval)
      else if (message.type && message.symbol && message.interval) {
        messageType = message.type;
        symbol = message.symbol;
        interval = message.interval;
      }
      else {
        this.sendErrorToClient(ws, `Invalid message format. Expected either: {"type": "subscribe", "symbol": "SOL", "interval": "1m"} or {"method": "subscribe", "subscription": {"type": "candle", "coin": "SOL", "interval": "1m"}}`);
        return;
      }

      if (messageType === 'subscribe') {
        this.handleSubscription(ws, symbol, interval);
      } else if (messageType === 'unsubscribe') {
        this.handleUnsubscription(ws, symbol, interval);
      } else {
        this.sendErrorToClient(ws, `Unknown message type: ${messageType}`);
      }
    } catch (error) {
      console.error('❌ Failed to parse client message:', error);
      this.sendErrorToClient(ws, 'Invalid message format');
    }
  }

  /**
   * Handle client subscription request
   */
  private handleSubscription(ws: WebSocket, symbol: string, interval: string): void {
    const subscriptionId = `${symbol}-${interval}`;
    
    // Add client to subscription
    if (!this.subscriptions.has(subscriptionId)) {
      this.subscriptions.set(subscriptionId, new Set());
    }
    this.subscriptions.get(subscriptionId)!.add(ws);
    
    // Add subscription to client
    this.clients.get(ws)!.add(subscriptionId);

    // Subscribe to Hyperliquid if not already subscribed
    if (!this.hyperliquidSubscriptions.has(subscriptionId)) {
      console.log(`Subscribing to Hyperliquid: ${symbol} ${interval}`);
      const hyperliquidSubId = hyperliquidWS.subscribeToCandles(symbol, interval);
      this.hyperliquidSubscriptions.set(subscriptionId, hyperliquidSubId);
      
      // Set up listener for this subscription
      hyperliquidWS.onCandleUpdate(hyperliquidSubId, (candle: ProcessedCandle) => {
        this.broadcastCandleData(subscriptionId, candle);
      });
    }

    // Send confirmation to client
    this.sendToClient(ws, {
      type: 'subscription',
      subscriptionId: subscriptionId,
      data: {
        symbol: symbol,
        timeframe: interval,
        timestamp: Date.now(),
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 0,
        trades: 0,
        priceChange: 0,
        priceChangePercent: 0
      }
    });

    console.log(`Client subscribed to ${symbol} ${interval}`);
  }

  /**
   * Handle client unsubscription request
   */
  private handleUnsubscription(ws: WebSocket, symbol: string, interval: string): void {
    const subscriptionId = `${symbol}-${interval}`;
    
    // Remove client from subscription
    const clients = this.subscriptions.get(subscriptionId);
    if (clients) {
      clients.delete(ws);
      
      // If no more clients, unsubscribe from Hyperliquid
      if (clients.size === 0) {
        console.log(`📊 Unsubscribing from Hyperliquid: ${symbol} ${interval}`);
        const hyperliquidSubId = this.hyperliquidSubscriptions.get(subscriptionId);
        if (hyperliquidSubId) {
          hyperliquidWS.unsubscribeFromCandles(hyperliquidSubId);
          this.hyperliquidSubscriptions.delete(subscriptionId);
        }
        this.subscriptions.delete(subscriptionId);
      }
    }
    
    // Remove subscription from client
    this.clients.get(ws)?.delete(subscriptionId);

    console.log(`Client unsubscribed from ${symbol} ${interval}`);
  }

  /**
   * Handle client disconnect
   */
  private handleClientDisconnect(ws: WebSocket): void {
    const clientSubscriptions = this.clients.get(ws);
    if (clientSubscriptions) {
      // Unsubscribe from all subscriptions
      for (const subscriptionId of clientSubscriptions) {
        const clients = this.subscriptions.get(subscriptionId);
        if (clients) {
          clients.delete(ws);
          
          // If no more clients, unsubscribe from Hyperliquid
          if (clients.size === 0) {
            const hyperliquidSubId = this.hyperliquidSubscriptions.get(subscriptionId);
            if (hyperliquidSubId) {
              hyperliquidWS.unsubscribeFromCandles(hyperliquidSubId);
              this.hyperliquidSubscriptions.delete(subscriptionId);
            }
            this.subscriptions.delete(subscriptionId);
          }
        }
      }
    }
    
    this.clients.delete(ws);
  }

  /**
   * Broadcast candle data to all clients subscribed to a specific symbol/interval
   */
  private broadcastCandleData(subscriptionId: string, candle: ProcessedCandle): void {
    const clients = this.subscriptions.get(subscriptionId);
    if (!clients || clients.size === 0) {
      return;
    }

    const message: ServerMessage = {
      type: 'candle',
      subscriptionId: subscriptionId,
      data: candle
    };

    const messageStr = JSON.stringify(message);
    let sentCount = 0;

    clients.forEach((ws) => {
      if (ws.readyState === WebSocket.OPEN) {
        try {
          ws.send(messageStr);
          sentCount++;
        } catch (error) {
          console.error('❌ Failed to send message to client:', error);
          // Remove client if send fails
          this.handleClientDisconnect(ws);
        }
      } else {
        // Remove client if connection is not open
        this.handleClientDisconnect(ws);
      }
    });

    if (sentCount > 0) {
      console.log(`Broadcasted ${candle.symbol} ${candle.timeframe} candle to ${sentCount} clients`);
    }
  }

  /**
   * Send message to a specific client
   */
  private sendToClient(ws: WebSocket, message: ServerMessage): void {
    if (ws.readyState === WebSocket.OPEN) {
      try {
        ws.send(JSON.stringify(message));
      } catch (error) {
        console.error('❌ Failed to send message to client:', error);
      }
    }
  }

  /**
   * Send error message to a specific client
   */
  private sendErrorToClient(ws: WebSocket, error: string): void {
    this.sendToClient(ws, {
      type: 'error',
      error: error
    });
  }

  /**
   * Set up listeners for Hyperliquid WebSocket events
   */
  private setupHyperliquidListeners(): void {
    // Connect to Hyperliquid WebSocket
    hyperliquidWS.connect().catch((error) => {
      console.error('❌ Failed to connect to Hyperliquid WebSocket:', error);
    });

    // Log connection status changes
    const checkConnection = () => {
      const isConnected = hyperliquidWS.getConnectionStatus();
      console.log(`🔗 Hyperliquid WebSocket status: ${isConnected ? 'Connected' : 'Disconnected'}`);
    };

    // Check connection status every 30 seconds
    setInterval(checkConnection, 30000);
    checkConnection();
  }

  /**
   * Get server statistics
   */
  public getStats(): {
    connectedClients: number;
    activeSubscriptions: number;
    hyperliquidSubscriptions: number;
  } {
    return {
      connectedClients: this.clients.size,
      activeSubscriptions: this.subscriptions.size,
      hyperliquidSubscriptions: this.hyperliquidSubscriptions.size
    };
  }

  /**
   * Broadcast system message to all clients
   */
  public broadcastSystemMessage(message: string): void {
    const systemMessage: ServerMessage = {
      type: 'subscription',
      subscriptionId: 'system',
      data: {
        symbol: 'SYSTEM',
        timeframe: 'system',
        timestamp: Date.now(),
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 0,
        trades: 0,
        priceChange: 0,
        priceChangePercent: 0
      }
    };

    this.clients.forEach((_, ws) => {
      this.sendToClient(ws, systemMessage);
    });
  }
}

// Export singleton instance
let wsServerInstance: WebSocketServer | null = null;

export function initializeWebSocketServer(server: Server): WebSocketServer {
  if (!wsServerInstance) {
    wsServerInstance = new WebSocketServer(server);
    wsServerInstance.start();
  }
  return wsServerInstance;
}

export function getWebSocketServer(): WebSocketServer | null {
  return wsServerInstance;
}
