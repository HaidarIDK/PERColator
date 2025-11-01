import WebSocket from "ws";

export interface HyperliquidCandle {
  t: number;
  T: number;
  s: string;
  i: string;
  o: number | string;
  c: number | string;
  h: number | string;
  l: number | string;
  v: number | string;
  n: number | string;
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
  private subscriptions: Map<string, string> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectDelay = 5000;
  private isConnected = false;
  private eventListeners: Map<string, ((data: ProcessedCandle) => void)[]> = new Map();
  private reconnectTimeout: NodeJS.Timeout | null = null;

  constructor(
    private url: string = "wss://api.hyperliquid.xyz/ws"
  ) {}

  public async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        if (this.ws?.readyState === WebSocket.OPEN) {
          resolve();
          return;
        }

        console.log(`Connecting to Hyperliquid WebSocket...`);
        
        this.ws = new WebSocket(this.url);

        this.ws.on("open", () => {
          console.log("Connected to Hyperliquid WebSocket");
          this.isConnected = true;
          this.reconnectAttempts = 0;
          this.resubscribeAll();
          resolve();
        });

        this.ws.on("message", (data: Buffer) => {
          this.handleMessage(data);
        });

        this.ws.on("error", (error) => {
          console.error("WebSocket error:", error);
          this.isConnected = false;
          if (this.reconnectAttempts === 0) {
            reject(error);
          }
        });

        this.ws.on("close", () => {
          console.warn(`Hyperliquid WebSocket closed`);
          this.isConnected = false;
          this.handleReconnect();
        });

      } catch (error) {
        console.error("Failed to create WebSocket:", error);
        reject(error);
      }
    });
  }

  public subscribeToCandles(coin: string, interval: string): string {
    const subscriptionId = `${coin}-${interval}`;
    
    if (this.subscriptions.has(subscriptionId)) {
      return subscriptionId;
    }

    this.subscriptions.set(subscriptionId, subscriptionId);

    if (this.isConnected && this.ws) {
      this.sendSubscription(coin, interval);
    }

    console.log(`Subscribed to ${coin} ${interval}`);
    return subscriptionId;
  }

  public unsubscribeFromCandles(subscriptionId: string): boolean {
    if (!this.subscriptions.has(subscriptionId)) {
      return false;
    }

    const [coin, interval] = subscriptionId.split('-');

    if (this.isConnected && this.ws) {
      this.sendUnsubscription(coin, interval);
    }

    this.subscriptions.delete(subscriptionId);
    this.eventListeners.delete(subscriptionId);
    return true;
  }

  public onCandleUpdate(subscriptionId: string, callback: (candle: ProcessedCandle) => void): void {
    if (!this.eventListeners.has(subscriptionId)) {
      this.eventListeners.set(subscriptionId, []);
    }
    this.eventListeners.get(subscriptionId)!.push(callback);
  }

  public getConnectionStatus(): boolean {
    return this.isConnected;
  }

  private sendSubscription(coin: string, interval: string): void {
    if (!this.ws) return;

    const message = {
      method: "subscribe",
      subscription: {
        type: "candle",
        coin: coin,
        interval: interval,
      },
    };

    this.ws.send(JSON.stringify(message));
  }

  private sendUnsubscription(coin: string, interval: string): void {
    if (!this.ws) return;

    const message = {
      method: "unsubscribe",
      subscription: {
        type: "candle",
        coin: coin,
        interval: interval,
      },
    };

    this.ws.send(JSON.stringify(message));
  }

  private handleMessage(data: Buffer): void {
    try {
      const message: any = JSON.parse(data.toString());

      if (message.channel === "subscriptionResponse") {
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
    
    const open = typeof candle.o === 'string' ? parseFloat(candle.o) : candle.o;
    const close = typeof candle.c === 'string' ? parseFloat(candle.c) : candle.c;
    const high = typeof candle.h === 'string' ? parseFloat(candle.h) : candle.h;
    const low = typeof candle.l === 'string' ? parseFloat(candle.l) : candle.l;
    const volume = typeof candle.v === 'string' ? parseFloat(candle.v) : candle.v;
    const trades = typeof candle.n === 'string' ? parseInt(candle.n) : candle.n;
    
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

    const listeners = this.eventListeners.get(subscriptionId);
    if (listeners) {
      listeners.forEach(callback => callback(processedCandle));
    }
  }

  private resubscribeAll(): void {
    console.log(`Resubscribing to ${this.subscriptions.size} feeds...`);
    this.subscriptions.forEach((subId) => {
      const [coin, interval] = subId.split('-');
      this.sendSubscription(coin, interval);
    });
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error(`Max reconnection attempts reached`);
      return;
    }

    this.reconnectAttempts++;
    console.log(`Reconnecting (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }

    this.reconnectTimeout = setTimeout(() => {
      this.connect().catch((error) => {
        console.error("Reconnection failed:", error);
      });
    }, this.reconnectDelay);
  }

  public disconnect(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
    }
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.isConnected = false;
  }
}

export const hyperliquidWS = new HyperliquidWebSocketService();

