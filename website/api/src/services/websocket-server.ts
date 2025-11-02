import WebSocket from 'ws';
import { Server } from 'http';
import { hyperliquidWS } from './hyperliquid-websocket';
import { handleCLIWebSocket } from './cli-websocket';

export interface ClientSubscription {
  symbol: string;
  interval: string;
  subscriptionId: string;
}

export interface ClientMessage {
  type: 'subscribe' | 'unsubscribe';
  symbol?: string;
  interval?: string;
}

export interface ServerMessage {
  type: 'candle' | 'subscription' | 'error';
  subscriptionId?: string;
  data?: {
    symbol: string;
    timeframe: string;
    timestamp: number;
    open: number;
    high: number;
    low: number;
    close: number;
    volume: number;
  };
  error?: string;
}

export class WebSocketServer {
  private wss: WebSocket.Server | null = null;
  private cliWss: WebSocket.Server | null = null;
  private clients: Map<WebSocket, Set<string>> = new Map();
  private subscriptions: Map<string, Set<WebSocket>> = new Map();
  private hyperliquidSubscriptions: Map<string, string> = new Map();

  constructor(private server: Server) {}

  public start(): void {
    console.log('Starting WebSocket server...');
    
    // WebSocket for chart data
    this.wss = new WebSocket.Server({ 
      server: this.server,
      path: '/ws'
    });

    // WebSocket for CLI
    this.cliWss = new WebSocket.Server({
      server: this.server,
      path: '/ws/cli'
    });

    this.wss.on('connection', (ws: WebSocket) => {
      console.log('New WebSocket client connected');
      
      this.clients.set(ws, new Set());
      
      ws.on('message', (data: Buffer) => {
        this.handleClientMessage(ws, data);
      });

      ws.on('close', () => {
        console.log('Client disconnected');
        this.handleClientDisconnect(ws);
      });

      ws.on('error', (error: Error) => {
        console.error('WebSocket error:', error);
        this.handleClientDisconnect(ws);
      });

      // Send welcome
      this.sendToClient(ws, {
        type: 'subscription',
        subscriptionId: 'welcome'
      });
    });

    // Handle CLI WebSocket connections
    this.cliWss.on('connection', (ws: WebSocket) => {
      console.log('New CLI WebSocket client connected');
      handleCLIWebSocket(ws);
    });

    this.setupHyperliquidListeners();

    console.log('WebSocket server started on /ws (charts) and /ws/cli (CLI)');
  }

  public stop(): void {
    if (this.wss) {
      this.wss.close();
      this.wss = null;
    }
    if (this.cliWss) {
      this.cliWss.close();
      this.cliWss = null;
    }
    this.clients.clear();
    this.subscriptions.clear();
    this.hyperliquidSubscriptions.clear();
  }

  private handleClientMessage(ws: WebSocket, data: Buffer): void {
    try {
      const message: ClientMessage = JSON.parse(data.toString());

      if (message.type === 'subscribe' && message.symbol && message.interval) {
        this.handleSubscription(ws, message.symbol, message.interval);
      } else if (message.type === 'unsubscribe' && message.symbol && message.interval) {
        this.handleUnsubscription(ws, message.symbol, message.interval);
      } else {
        this.sendErrorToClient(ws, 'Invalid message format');
      }
    } catch (error) {
      console.error('Failed to parse client message:', error);
      this.sendErrorToClient(ws, 'Invalid message format');
    }
  }

  private handleSubscription(ws: WebSocket, symbol: string, interval: string): void {
    const subscriptionId = `${symbol}-${interval}`;
    
    if (!this.subscriptions.has(subscriptionId)) {
      this.subscriptions.set(subscriptionId, new Set());
    }
    this.subscriptions.get(subscriptionId)!.add(ws);
    this.clients.get(ws)!.add(subscriptionId);

    // Subscribe to Hyperliquid if not already
    if (!this.hyperliquidSubscriptions.has(subscriptionId)) {
      console.log(`Subscribing to Hyperliquid: ${symbol} ${interval}`);
      const hyperliquidSubId = hyperliquidWS.subscribeToCandles(symbol, interval);
      this.hyperliquidSubscriptions.set(subscriptionId, hyperliquidSubId);
      
      hyperliquidWS.onCandleUpdate(hyperliquidSubId, (candle) => {
        this.broadcastCandleData(subscriptionId, {
          symbol: candle.symbol,
          timeframe: candle.timeframe,
          timestamp: candle.timestamp,
          open: candle.open,
          high: candle.high,
          low: candle.low,
          close: candle.close,
          volume: candle.volume,
        });
      });
    }

    this.sendToClient(ws, {
      type: 'subscription',
      subscriptionId: subscriptionId
    });

    console.log(`Client subscribed to ${symbol} ${interval}`);
  }

  private handleUnsubscription(ws: WebSocket, symbol: string, interval: string): void {
    const subscriptionId = `${symbol}-${interval}`;
    
    const clients = this.subscriptions.get(subscriptionId);
    if (clients) {
      clients.delete(ws);
      
      if (clients.size === 0) {
        const hyperliquidSubId = this.hyperliquidSubscriptions.get(subscriptionId);
        if (hyperliquidSubId) {
          hyperliquidWS.unsubscribeFromCandles(hyperliquidSubId);
          this.hyperliquidSubscriptions.delete(subscriptionId);
        }
        this.subscriptions.delete(subscriptionId);
      }
    }
    
    this.clients.get(ws)?.delete(subscriptionId);
    console.log(`Client unsubscribed from ${symbol} ${interval}`);
  }

  private handleClientDisconnect(ws: WebSocket): void {
    const clientSubscriptions = this.clients.get(ws);
    if (clientSubscriptions) {
      for (const subscriptionId of clientSubscriptions) {
        const clients = this.subscriptions.get(subscriptionId);
        if (clients) {
          clients.delete(ws);
          
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

  private broadcastCandleData(subscriptionId: string, candle: ServerMessage['data']): void {
    const clients = this.subscriptions.get(subscriptionId);
    if (!clients || clients.size === 0) return;

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
          console.error('Failed to send message:', error);
          this.handleClientDisconnect(ws);
        }
      } else {
        this.handleClientDisconnect(ws);
      }
    });

    if (sentCount > 0) {
      console.log(`📡 Broadcasted ${candle?.symbol} ${candle?.timeframe} to ${sentCount} clients`);
    }
  }

  private sendToClient(ws: WebSocket, message: ServerMessage): void {
    if (ws.readyState === WebSocket.OPEN) {
      try {
        ws.send(JSON.stringify(message));
      } catch (error) {
        console.error('Failed to send message:', error);
      }
    }
  }

  private sendErrorToClient(ws: WebSocket, error: string): void {
    this.sendToClient(ws, {
      type: 'error',
      error: error
    });
  }

  private setupHyperliquidListeners(): void {
    hyperliquidWS.connect().catch((error) => {
      console.error('Failed to connect to Hyperliquid WebSocket:', error);
    });

    const checkConnection = () => {
      const isConnected = hyperliquidWS.getConnectionStatus();
      if (!isConnected) {
        console.log('Reconnecting to Hyperliquid...');
        hyperliquidWS.connect().catch(console.error);
      }
    };

    setInterval(checkConnection, 30000); // Check every 30s
  }
}

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

