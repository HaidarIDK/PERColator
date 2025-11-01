/**
 * Hyperliquid API Client for Chart Data
 */

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:5001/ws';

export interface CandlestickData {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export type Coin = 'SOL' | 'ETH' | 'BTC';
export type Timeframe = '1m' | '15m' | '1h' | '12h';

class HyperliquidClient {
  private ws: WebSocket | null = null;
  private subscriptions: Map<string, (candle: CandlestickData) => void> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;

  /**
   * Fetch historical candlestick data
   */
  async getCandles(
    coin: Coin,
    interval: Timeframe,
    startTime?: number,
    endTime?: number
  ): Promise<CandlestickData[]> {
    try {
      const params = new URLSearchParams({
        coin,
        interval,
      });

      if (startTime) params.append('startTime', startTime.toString());
      if (endTime) params.append('endTime', endTime.toString());

      const response = await fetch(`${API_URL}/api/hyperliquid/candles?${params}`);
      
      if (!response.ok) {
        throw new Error(`API error: ${response.status}`);
      }

      const data = await response.json();
      return data.candles || [];
    } catch (error) {
      console.error(`Failed to fetch candles for ${coin} ${interval}:`, error);
      throw error;
    }
  }

  /**
   * Get latest price
   */
  async getPrice(coin: Coin): Promise<number> {
    try {
      const response = await fetch(`${API_URL}/api/hyperliquid/price?coin=${coin}`);
      
      if (!response.ok) {
        throw new Error(`API error: ${response.status}`);
      }

      const data = await response.json();
      return data.price || 0;
    } catch (error) {
      console.error(`Failed to fetch price for ${coin}:`, error);
      throw error;
    }
  }

  /**
   * Connect to WebSocket for real-time updates
   */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        if (this.ws?.readyState === WebSocket.OPEN) {
          resolve();
          return;
        }

        console.log(`Connecting to WebSocket: ${WS_URL}`);
        this.ws = new WebSocket(WS_URL);

        this.ws.onopen = () => {
          console.log('WebSocket connected');
          this.reconnectAttempts = 0;
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data);
            
            if (message.type === 'candle' && message.data) {
              const subscriptionKey = `${message.data.symbol}-${message.data.timeframe}`;
              const callback = this.subscriptions.get(subscriptionKey);
              
              if (callback) {
                callback({
                  time: Math.floor(message.data.timestamp / 1000),
                  open: message.data.open,
                  high: message.data.high,
                  low: message.data.low,
                  close: message.data.close,
                  volume: message.data.volume,
                });
              }
            }
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
          }
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          reject(error);
        };

        this.ws.onclose = () => {
          console.warn('WebSocket closed');
          this.handleReconnect();
        };

      } catch (error) {
        console.error('Failed to create WebSocket:', error);
        reject(error);
      }
    });
  }

  /**
   * Subscribe to real-time candle updates
   */
  subscribe(
    coin: Coin,
    interval: Timeframe,
    callback: (candle: CandlestickData) => void
  ): () => void {
    const subscriptionKey = `${coin}-${interval}`;
    
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not connected, connecting...');
      this.connect().then(() => {
        this.doSubscribe(coin, interval, callback);
      });
      return () => this.unsubscribe(coin, interval);
    }

    this.doSubscribe(coin, interval, callback);
    return () => this.unsubscribe(coin, interval);
  }

  private doSubscribe(
    coin: Coin,
    interval: Timeframe,
    callback: (candle: CandlestickData) => void
  ): void {
    const subscriptionKey = `${coin}-${interval}`;
    this.subscriptions.set(subscriptionKey, callback);

    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'subscribe',
        symbol: coin,
        interval: interval,
      }));
      console.log(`Subscribed to ${coin} ${interval}`);
    }
  }

  private unsubscribe(coin: Coin, interval: Timeframe): void {
    const subscriptionKey = `${coin}-${interval}`;
    this.subscriptions.delete(subscriptionKey);

    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'unsubscribe',
        symbol: coin,
        interval: interval,
      }));
      console.log(`Unsubscribed from ${coin} ${interval}`);
    }
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached');
      return;
    }

    this.reconnectAttempts++;
      console.log(`Reconnecting (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);

    setTimeout(() => {
      this.connect().catch(console.error);
    }, 5000);
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscriptions.clear();
  }

  isConnected(): boolean {
    return !!this.ws && this.ws.readyState === WebSocket.OPEN;
  }
}

export const hyperliquidClient = new HyperliquidClient();

