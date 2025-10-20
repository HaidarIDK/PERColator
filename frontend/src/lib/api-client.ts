/**
 * API Client for connecting to PERColator backend
 */

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';
const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3000/ws';

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

// ============================================
// API CLIENT CLASS
// ============================================

class PercolatorAPIClient {
  private baseUrl: string;
  private wsUrl: string;
  private ws: WebSocket | null = null;

  constructor() {
    this.baseUrl = API_URL;
    this.wsUrl = WS_URL;
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
    const res = await fetch(`${this.baseUrl}/api/market/${symbol}`);
    if (!res.ok) throw new Error(`Failed to fetch market data for ${symbol}`);
    return res.json();
  }

  async getOrderbook(symbol: string): Promise<Orderbook> {
    const res = await fetch(`${this.baseUrl}/api/market/${symbol}/orderbook`);
    if (!res.ok) throw new Error(`Failed to fetch orderbook for ${symbol}`);
    return res.json();
  }

  async getChartData(
    symbol: string,
    timeframe: string = '15',
    limit: number = 100
  ): Promise<CandlestickData[]> {
    const res = await fetch(
      `${this.baseUrl}/api/market/${symbol}/candles?timeframe=${timeframe}&limit=${limit}`
    );
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
      body: JSON.stringify({ walletAddress })
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

  connectWebSocket(
    onMessage: (data: unknown) => void,
    onError?: (error: Event) => void
  ): () => void {
    try {
      this.ws = new WebSocket(this.wsUrl);

      this.ws.onopen = () => {
        console.log('✅ WebSocket connected to backend');
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          onMessage(data);
        } catch (err) {
          console.error('Failed to parse WebSocket message:', err);
        }
      };

      this.ws.onerror = (error) => {
        console.error('❌ WebSocket error:', error);
        if (onError) onError(error);
      };

      this.ws.onclose = () => {
        console.log('🔌 WebSocket disconnected');
      };

      // Return cleanup function
      return () => {
        if (this.ws) {
          this.ws.close();
          this.ws = null;
        }
      };
    } catch (error) {
      console.error('Failed to connect WebSocket:', error);
      return () => {};
    }
  }

  subscribeToMarket(symbol: string) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        channel: 'market',
        symbol
      }));
    }
  }

  subscribeToOrderbook(symbol: string) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        channel: 'orderbook',
        symbol
      }));
    }
  }
}

// Export singleton instance
export const apiClient = new PercolatorAPIClient();

// Export class for custom instances
export default PercolatorAPIClient;

