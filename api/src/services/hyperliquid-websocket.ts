import WebSocket from "ws";

// TypeScript interfaces for Hyperliquid WebSocket data
export interface HyperliquidCandle {
  t: number; // open millis
  T: number; // close millis
  s: string; // coin
  i: string; // interval
  o: number | string; // open price (can be string from API)
  c: number | string; // close price (can be string from API)
  h: number | string; // high price (can be string from API)
  l: number | string; // low price (can be string from API)
  v: number | string; // volume (can be string from API)
  n: number | string; // number of trades (can be string from API)
}

export interface HyperliquidWebSocketMessage {
  channel: string;
  data: HyperliquidCandle | any;
  isSnapshot?: boolean;
}

export interface CandleSubscription {
  coin: string;
  interval: string;
  id: string; // unique identifier for this subscription
}

export interface ProcessedCandle {
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

export class HyperliquidWebSocketService {
  private ws: WebSocket | null = null;
  private subscriptions: Map<string, CandleSubscription> = new Map();
  private candleData: Map<string, ProcessedCandle[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 5000;
  private isConnected = false;
  private eventListeners: Map<string, ((data: ProcessedCandle) => void)[]> = new Map();

  constructor(
    private url: string = "wss://api.hyperliquid.xyz/ws",
    private testnet: boolean = false
  ) {
    if (testnet) {
      this.url = "wss://api.hyperliquid-testnet.xyz/ws";
    }
  }

  /**
   * Connect to Hyperliquid WebSocket
   */
  public async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        console.log(`🔌 Connecting to Hyperliquid WebSocket: ${this.url}`);
        
        this.ws = new WebSocket(this.url, {
          headers: {
            "Sec-WebSocket-Key": "ZEUXn+g5bJ4dsmej6wkcDA==",
          },
        });

        this.ws.on("open", () => {
          console.log("✅ Connected to Hyperliquid WebSocket");
          this.isConnected = true;
          this.reconnectAttempts = 0;
          
          // Resubscribe to existing subscriptions
          this.resubscribeAll();
          resolve();
        });

        this.ws.on("message", (data: Buffer) => {
          this.handleMessage(data);
        });

        this.ws.on("error", (error) => {
          console.error("❌ WebSocket error:", error);
          this.isConnected = false;
          reject(error);
        });

        this.ws.on("close", (code, reason) => {
          console.warn(`⚠️ WebSocket closed. Code: ${code}, Reason: ${reason}`);
          this.isConnected = false;
          this.handleReconnect();
        });

      } catch (error) {
        console.error("Failed to create WebSocket connection:", error);
        reject(error);
      }
    });
  }

  /**
   * Subscribe to candle data for a specific coin and timeframe
   */
  public subscribeToCandles(coin: string, interval: string): string {
    const subscriptionId = `${coin}-${interval}`;
    
    // Check if already subscribed
    if (this.subscriptions.has(subscriptionId)) {
      console.log(`Already subscribed to ${coin} ${interval}`);
      return subscriptionId;
    }

    const subscription: CandleSubscription = {
      coin,
      interval,
      id: subscriptionId,
    };

    this.subscriptions.set(subscriptionId, subscription);

    if (this.isConnected && this.ws) {
      this.sendSubscription(subscription);
    }

    console.log(`📊 Subscribed to ${coin} ${interval} candles`);
    return subscriptionId;
  }

  /**
   * Unsubscribe from candle data
   */
  public unsubscribeFromCandles(subscriptionId: string): boolean {
    const subscription = this.subscriptions.get(subscriptionId);
    if (!subscription) {
      console.log(`No subscription found for ${subscriptionId}`);
      return false;
    }

    if (this.isConnected && this.ws) {
      this.sendUnsubscription(subscription);
    }

    this.subscriptions.delete(subscriptionId);
    this.candleData.delete(subscriptionId);
    this.eventListeners.delete(subscriptionId);

    console.log(`📊 Unsubscribed from ${subscriptionId} candles`);
    return true;
  }

  /**
   * Get latest candle data for a subscription
   */
  public getLatestCandle(subscriptionId: string): ProcessedCandle | null {
    const candles = this.candleData.get(subscriptionId);
    return candles && candles.length > 0 ? candles[candles.length - 1] : null;
  }

  /**
   * Get historical candle data for a subscription
   */
  public getCandleHistory(subscriptionId: string, limit: number = 100): ProcessedCandle[] {
    const candles = this.candleData.get(subscriptionId) || [];
    return candles.slice(-limit);
  }

  /**
   * Add event listener for candle updates
   */
  public onCandleUpdate(subscriptionId: string, callback: (candle: ProcessedCandle) => void): void {
    if (!this.eventListeners.has(subscriptionId)) {
      this.eventListeners.set(subscriptionId, []);
    }
    this.eventListeners.get(subscriptionId)!.push(callback);
  }

  /**
   * Remove event listener
   */
  public offCandleUpdate(subscriptionId: string, callback: (candle: ProcessedCandle) => void): void {
    const listeners = this.eventListeners.get(subscriptionId);
    if (listeners) {
      const index = listeners.indexOf(callback);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    }
  }

  /**
   * Get all active subscriptions
   */
  public getActiveSubscriptions(): CandleSubscription[] {
    return Array.from(this.subscriptions.values());
  }

  /**
   * Disconnect from WebSocket
   */
  public disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.isConnected = false;
    console.log("🔌 Disconnected from Hyperliquid WebSocket");
  }

  /**
   * Check if WebSocket is connected
   */
  public getConnectionStatus(): boolean {
    return this.isConnected;
  }

  // Private methods

  private sendSubscription(subscription: CandleSubscription): void {
    if (!this.ws) return;

    const message = {
      method: "subscribe",
      subscription: {
        type: "candle",
        coin: subscription.coin,
        interval: subscription.interval,
      },
    };

    this.ws.send(JSON.stringify(message));
    console.log(`📤 Sent subscription: ${subscription.coin} ${subscription.interval}`);
  }

  private sendUnsubscription(subscription: CandleSubscription): void {
    if (!this.ws) return;

    const message = {
      method: "unsubscribe",
      subscription: {
        type: "candle",
        coin: subscription.coin,
        interval: subscription.interval,
      },
    };

    this.ws.send(JSON.stringify(message));
    console.log(`📤 Sent unsubscription: ${subscription.coin} ${subscription.interval}`);
  }

  private handleMessage(data: Buffer): void {
    try {
      const message: HyperliquidWebSocketMessage = JSON.parse(data.toString());

      if (message.channel === "subscriptionResponse") {
        console.log("✅ Subscription confirmed:", message.data);
        return;
      }

      if (message.channel === "candle" && message.data) {
        this.processCandleData(message.data as HyperliquidCandle);
      }
    } catch (error) {
      console.error("Failed to parse WebSocket message:", error);
    }
  }

  private processCandleData(candle: HyperliquidCandle): void {
    const subscriptionId = `${candle.s}-${candle.i}`;
    
    // Convert string values to numbers (Hyperliquid sends some values as strings)
    const open = typeof candle.o === 'string' ? parseFloat(candle.o) : candle.o;
    const close = typeof candle.c === 'string' ? parseFloat(candle.c) : candle.c;
    const high = typeof candle.h === 'string' ? parseFloat(candle.h) : candle.h;
    const low = typeof candle.l === 'string' ? parseFloat(candle.l) : candle.l;
    const volume = typeof candle.v === 'string' ? parseFloat(candle.v) : candle.v;
    const trades = typeof candle.n === 'string' ? parseInt(candle.n) : candle.n;
    
    // Calculate price change
    const priceChange = close - open;
    const priceChangePercent = open !== 0 ? (priceChange / open) * 100 : 0;

    const processedCandle: ProcessedCandle = {
      symbol: candle.s,
      timeframe: candle.i,
      timestamp: candle.t,
      open,
      high,
      low,
      close,
      volume,
      trades,
      priceChange,
      priceChangePercent,
    };

    // Store candle data
    if (!this.candleData.has(subscriptionId)) {
      this.candleData.set(subscriptionId, []);
    }

    const candles = this.candleData.get(subscriptionId)!;
    
    // Update or add candle (keep last 1000 candles)
    const existingIndex = candles.findIndex(c => c.timestamp === candle.t);
    if (existingIndex >= 0) {
      candles[existingIndex] = processedCandle;
    } else {
      candles.push(processedCandle);
      // Keep only last 1000 candles
      if (candles.length > 1000) {
        candles.shift();
      }
    }

    // Notify listeners
    const listeners = this.eventListeners.get(subscriptionId);
    if (listeners) {
      listeners.forEach(callback => callback(processedCandle));
    }

    console.log(
      `[${candle.s} ${candle.i}] $${close.toFixed(2)} ` +
      `(H:${high.toFixed(2)} L:${low.toFixed(2)} V:${volume.toFixed(0)}) ` +
      `Change: ${priceChangePercent.toFixed(2)}%`
    );
  }

  private resubscribeAll(): void {
    console.log(`🔄 Resubscribing to ${this.subscriptions.size} feeds...`);
    this.subscriptions.forEach(subscription => {
      this.sendSubscription(subscription);
    });
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error(`❌ Max reconnection attempts (${this.maxReconnectAttempts}) reached`);
      return;
    }

    this.reconnectAttempts++;
    console.log(`🔄 Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts}) in ${this.reconnectDelay}ms...`);

    setTimeout(() => {
      this.connect().catch(error => {
        console.error("Reconnection failed:", error);
      });
    }, this.reconnectDelay);
  }
}

// Export singleton instance
export const hyperliquidWS = new HyperliquidWebSocketService();
