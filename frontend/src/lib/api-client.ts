/**
 * API Client for connecting to PERColator backend
 */

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';
const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3000/ws';
const SERVER_WS_URL = process.env.NEXT_PUBLIC_SERVER_WS_URL || 'ws://localhost:3000/ws';

// ============================================
// TYPE DEFINITIONS
// ============================================

export interface MarketData {
  symbol: string;
  price: number;
  change24h: number;
  volume24h: number;
  high24h: number;
  low24h: number;
  fundingRate: number;
  openInterest: number;
  indexPrice: number;
  markPrice: number;
}

export interface OrderbookLevel {
  price: number;
  quantity: number;
  total: number;
}

export interface Orderbook {
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  lastUpdate: number;
}

export interface Position {
  symbol: string;
  side: 'long' | 'short';
  size: number;
  entryPrice: number;
  markPrice: number;
  liquidationPrice: number;
  unrealizedPnl: number;
  leverage: number;
  margin: number;
}

export interface UserPortfolio {
  equity: number;
  freeCollateral: number;
  totalPositionValue: number;
  unrealizedPnl: number;
  marginUsage: number;
  positions: Position[];
}

export interface CandlestickData {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface FaucetInfo {
  isAvailable: boolean;
  amountPerClaim: number;
  cooldownSeconds: number;
  totalClaimed: number;
}

export interface FaucetClaimResult {
  success: boolean;
  signature?: string;
  amount?: number;
  error?: string;
}

// Server WebSocket types (matching your server implementation)
export interface ServerCandleData {
  symbol: string;
  timeframe: string;
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  trades: number;
  priceChange: number;
  priceChangePercent: number;
}

export interface ServerWebSocketMessage {
  type: 'candle' | 'subscription' | 'error';
  subscriptionId?: string;
  data?: ServerCandleData;
  error?: string;
}

// ============================================
// SERVER WEBSOCKET CLIENT
// ============================================

class ServerWebSocketClient {
  private ws: WebSocket | null = null;
  private subscriptions = new Set<string>();
  private messageHandlers = new Map<string, (data: any) => void>();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private reconnectTimeout: NodeJS.Timeout | null = null;

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        console.log('[Server] Already connected');
        resolve();
        return;
      }

      try {
        console.log('[Server] Attempting to connect to:', SERVER_WS_URL);
        this.ws = new WebSocket(SERVER_WS_URL);

        this.ws.onopen = () => {
          console.log('[Server] ✅ Connected successfully');
          this.reconnectAttempts = 0;
          this.reconnectDelay = 1000;
          
          // Resubscribe to all active subscriptions
          this.resubscribeToAll();
          
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const msg: ServerWebSocketMessage = JSON.parse(event.data);
            this.handleMessage(msg);
          } catch (e) {
            console.error('[Server] Parse error:', e, 'Raw data:', event.data);
          }
        };

        this.ws.onerror = (err) => {
          console.error('[Server] ❌ WebSocket error:', err);
          reject(err);
        };

        this.ws.onclose = (event) => {
          console.log('[Server] ⚠️ Disconnected', event.code, event.reason);
          this.ws = null;
          
          // Attempt to reconnect if not a manual close
          if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
            this.scheduleReconnect();
          }
        };
      } catch (error) {
        console.error('[Server] ❌ Failed to create WebSocket:', error);
        reject(error);
      }
    });
  }

  private handleMessage(msg: ServerWebSocketMessage) {
    console.log('[Server] Received message:', msg);
    
    if (msg.type === 'subscription') {
      console.log('[Server] Subscription confirmed:', msg.subscriptionId);
      return;
    }
    
    if (msg.type === 'candle' && msg.data) {
      const candleData = msg.data;
      console.log('[Server] Candle data received:', candleData);
      
      const candle: CandlestickData = {
        time: Math.floor(candleData.timestamp / 1000), // Convert milliseconds to seconds
        open: candleData.open,
        high: candleData.high,
        low: candleData.low,
        close: candleData.close,
        volume: candleData.volume,
      };

      console.log('[Server] Processed candle:', candle);

      this.messageHandlers.forEach((handler) =>
        handler({ 
          type: 'candle', 
          symbol: candleData.symbol,
          timeframe: candleData.timeframe,
          subscriptionId: msg.subscriptionId,
          data: candle,
          serverData: candleData // Include full server data
        })
      );
    }
    
    if (msg.type === 'error') {
      console.error('[Server] Error:', msg.error);
    }
  }

  subscribeToCandle(symbol: string, interval = '1m') {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('[Server] WebSocket not ready for subscription');
      return;
    }

    const key = `${symbol}-${interval}`;
    if (this.subscriptions.has(key)) return;

    const msg = {
      type: 'subscribe',
      symbol: symbol,
      interval: interval
    };

    this.ws.send(JSON.stringify(msg));
    this.subscriptions.add(key);
    console.log(`[Server] Subscribed to ${symbol} ${interval}`, msg);
  }

  unsubscribeFromCandle(symbol: string, interval = '1m') {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;

    const key = `${symbol}-${interval}`;
    if (!this.subscriptions.has(key)) return;

    const msg = {
      type: 'unsubscribe',
      symbol: symbol,
      interval: interval
    };

    this.ws.send(JSON.stringify(msg));
    this.subscriptions.delete(key);
    console.log(`[Server] Unsubscribed from ${symbol} ${interval}`, msg);
  }

  private scheduleReconnect() {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }

    this.reconnectAttempts++;
    console.log(`[Server] Scheduling reconnect attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts} in ${this.reconnectDelay}ms`);

    this.reconnectTimeout = setTimeout(() => {
      this.connect().catch((error) => {
        console.error('[Server] Reconnect failed:', error);
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
          this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
          this.scheduleReconnect();
        } else {
          console.error('[Server] Max reconnect attempts reached');
        }
      });
    }, this.reconnectDelay);
  }

  private resubscribeToAll() {
    console.log('[Server] Resubscribing to all active subscriptions:', Array.from(this.subscriptions));
    
    for (const subscription of this.subscriptions) {
      const [symbol, interval] = subscription.split('-');
      const msg = {
        type: 'subscribe',
        symbol: symbol,
        interval: interval
      };
      
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify(msg));
        console.log(`[Server] Resubscribed to ${symbol} ${interval}`);
      }
    }
  }

  onMessage(handler: (data: any) => void): () => void {
    const id = Math.random().toString(36).slice(2);
    this.messageHandlers.set(id, handler);
    return () => this.messageHandlers.delete(id);
  }

  disconnect() {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
    
    if (this.ws) {
      this.ws.close(1000, 'Manual disconnect');
      this.ws = null;
    }
    this.subscriptions.clear();
    this.messageHandlers.clear();
    this.reconnectAttempts = 0;
    this.reconnectDelay = 1000;
  }

  isConnected(): boolean {
    return !!this.ws && this.ws.readyState === WebSocket.OPEN;
  }
}

// ============================================
// API CLIENT CLASS
// ============================================

class PercolatorAPIClient {
  public baseUrl: string;
  private wsUrl: string;
  private ws: WebSocket | null = null;
  private serverWs: ServerWebSocketClient;

  constructor() {
    this.baseUrl = API_URL;
    this.wsUrl = WS_URL;
    this.serverWs = new ServerWebSocketClient();
  }

  // ==========================================
  // MARKET DATA
  // ==========================================

  async getMarkets(): Promise<MarketData[]> {
    const res = await fetch(`${this.baseUrl}/api/market/list`);
    if (!res.ok) throw new Error('Failed to fetch markets');
    return res.json();
  }

  async getMarketData(symbol: string): Promise<MarketData> {
    // Map symbol to your API format
    const apiSymbol = symbol === 'SOL' ? 'SOL-PERP' : 
                     symbol === 'ETH' ? 'ETH-PERP' : 
                     symbol === 'BTC' ? 'BTC-PERP' : symbol;
    
    const res = await fetch(`${this.baseUrl}/api/market/${apiSymbol}`);
    if (!res.ok) throw new Error(`Failed to fetch market data for ${symbol}`);
    return res.json();
  }

  async getOrderbook(symbol: string): Promise<Orderbook> {
    const res = await fetch(`${this.baseUrl}/api/market/${symbol}/orderbook`);
    if (!res.ok) throw new Error(`Failed to fetch orderbook for ${symbol}`);
    return res.json();
  }

  async getChartData(symbol: string, timeframe: string = '15m', limit: number = 10000, from?: number, to?: number): Promise<CandlestickData[]> {
    // Map timeframe to your API format
    const apiSymbol = symbol === 'SOL' ? 'SOL-PERP' : 
                     symbol === 'ETH' ? 'ETH-PERP' : 
                     symbol === 'BTC' ? 'BTC-PERP' : symbol;
    
    // Build query parameters
    const params = new URLSearchParams({
      timeframe,
      limit: limit.toString()
    });
    
    // Default start date for historical data: 2025-10-01
    const defaultFromMs = new Date('2025-10-01T00:00:00Z').getTime();
    const effectiveFrom = from === undefined ? defaultFromMs : from;
    params.append('from', effectiveFrom.toString());
    
    if (to !== undefined) {
      params.append('to', to.toString());
    }
    
    console.log(`📊 Fetching chart data for ${apiSymbol} with params:`, Object.fromEntries(params));
    
    const res = await fetch(`${this.baseUrl}/api/market/${apiSymbol}/candles?${params.toString()}`);
    if (!res.ok) throw new Error(`Failed to fetch chart data for ${symbol}`);
    return res.json();
  }

  // ==========================================
  // USER DATA
  // ==========================================

  async getPortfolio(walletAddress: string): Promise<UserPortfolio> {
    const res = await fetch(`${this.baseUrl}/api/user/${walletAddress}/portfolio`);
    if (!res.ok) throw new Error('Failed to fetch portfolio');
    return res.json();
  }

  async getPositions(walletAddress: string): Promise<Position[]> {
    const res = await fetch(`${this.baseUrl}/api/user/${walletAddress}/positions`);
    if (!res.ok) throw new Error('Failed to fetch positions');
    return res.json();
  }

  // ==========================================
  // FAUCET
  // ==========================================

  async getFaucetInfo(): Promise<FaucetInfo> {
    const res = await fetch(`${this.baseUrl}/api/faucet/info`);
    if (!res.ok) throw new Error('Failed to fetch faucet info');
    return res.json();
  }

  async claimFaucet(walletAddress: string): Promise<FaucetClaimResult> {
    const res = await fetch(`${this.baseUrl}/api/faucet/claim`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ walletAddress }),
    });

    if (!res.ok) {
      const error = await res.json();
      return { success: false, error: error.error || 'Failed to claim from faucet' };
    }

    return res.json();
  }

  async getTotalClaimed(): Promise<{ totalClaimed: number }> {
    const res = await fetch(`${this.baseUrl}/api/claims/total-claimed`);
    if (!res.ok) throw new Error('Failed to fetch total claimed');
    return res.json();
  }

  // ==========================================
  // WEBSOCKET (Real-time updates)
  // ==========================================

  connectWebSocket(onMessage: (data: unknown) => void): () => void {
    try {
      this.ws = new WebSocket(this.wsUrl);
      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          onMessage(data);
        } catch {
          /* ignore parse errors */
        }
      };
      return () => {
        this.ws?.close();
        this.ws = null;
      };
    } catch {
      return () => {};
    }
  }

  subscribeToMarket(symbol: string) {
    this.ws?.readyState === WebSocket.OPEN &&
      this.ws.send(JSON.stringify({ type: 'subscribe', channel: 'market', symbol }));
  }

  subscribeToOrderbook(symbol: string) {
    this.ws?.readyState === WebSocket.OPEN &&
      this.ws.send(JSON.stringify({ type: 'subscribe', channel: 'orderbook', symbol }));
  }

  // ==========================================
  // SERVER WEBSOCKET METHODS (RECOMMENDED)
  // ==========================================

  async connectServerWebSocket(): Promise<void> {
    try {
      await this.serverWs.connect();
    } catch (error) {
      console.error('Failed to connect to Server WebSocket:', error);
      throw error;
    }
  }

  subscribeToServerCandle(symbol: string, interval: string = '1m') {
    this.serverWs.subscribeToCandle(symbol, interval);
  }

  unsubscribeFromServerCandle(symbol: string, interval: string = '1m') {
    this.serverWs.unsubscribeFromCandle(symbol, interval);
  }

  onServerMessage(handler: (data: any) => void): () => void {
    return this.serverWs.onMessage(handler);
  }

  disconnectServerWebSocket() {
    this.serverWs.disconnect();
  }

  isServerConnected(): boolean {
    return this.serverWs.isConnected();
  }
}

// Export singleton instance
export const apiClient = new PercolatorAPIClient();

// Export class for custom instances
export default PercolatorAPIClient;
