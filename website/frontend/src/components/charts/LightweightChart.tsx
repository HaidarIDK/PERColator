"use client"

import { useState, useEffect, useRef, memo } from "react"
import { TrendingUp, TrendingDown, Activity } from "lucide-react"
import { createChart, ColorType, IChartApi, ISeriesApi, CandlestickSeries, Time, CandlestickData, IRange } from 'lightweight-charts'
import { apiClient } from "@/lib/api-client"

interface LightweightChartProps {
  coinId: "ethereum" | "bitcoin" | "solana"
  timeframe: "15" | "60" | "240" | "D"
  onPriceUpdate?: (price: number) => void
}

function LightweightChart({ coinId, timeframe, onPriceUpdate }: LightweightChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [priceChange, setPriceChange] = useState<number>(0);
  const wsCleanupRef = useRef<(() => void) | null>(null);
  const [oldestTimestamp, setOldestTimestamp] = useState<number | null>(null);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [loadedChunks, setLoadedChunks] = useState<Set<string>>(new Set());
  const [dataLoadingState, setDataLoadingState] = useState<'idle' | 'loading' | 'error'>('idle');

  const getServerSymbolFromCoinId = (coinId: "ethereum" | "bitcoin" | "solana") => {
    switch(coinId) {
      case "ethereum": return "ETH";
      case "bitcoin": return "BTC";
      case "solana": return "SOL";
      default: return "SOL";
    }
  };

  const getServerIntervalFromTimeframe = (timeframe: "15" | "60" | "240" | "D") => {
    switch(timeframe) {
      case "15": return "15m";
      case "60": return "1h";
      case "240": return "4h";
      case "D": return "1d";
      default: return "15m";
    }
  };

  const getChunkSizeMs = (timeframe: string): number => {
    const chunkSizes: Record<string, number> = {
      "15": 14 * 24 * 60 * 60 * 1000,
      "60": 30 * 24 * 60 * 60 * 1000,
      "240": 90 * 24 * 60 * 60 * 1000,
      "D": 365 * 24 * 60 * 60 * 1000,
    };
    return chunkSizes[timeframe] || 14 * 24 * 60 * 60 * 1000;
  };

  const getChunkKey = (startTime: number, endTime: number): string => {
    return `${startTime}-${endTime}`;
  };

  const shouldLoadMoreData = (_visibleRange: IRange<Time> | null, _oldestTimestamp: number | null): boolean => {
    return false;
  };

  const getBasePrice = (coinId: "ethereum" | "bitcoin" | "solana") => {
    switch(coinId) {
      case "ethereum": return 3882;
      case "bitcoin": return 97500;
      case "solana": return 185;
      default: return 3882;
    }
  };

  useEffect(() => {
    if (!chartContainerRef.current) return;
  
    let isComponentMounted = true;
    let cleanupFunctions: (() => void)[] = [];
  
    const timeoutId = setTimeout(async () => {
      if (!chartContainerRef.current || !isComponentMounted) return;
  
      const chart = createChart(chartContainerRef.current, {
        layout: {
          background: { type: ColorType.Solid, color: "transparent" },
          textColor: "#ffffff",
        },
        grid: {
          vertLines: { color: "#181825" },
          horzLines: { color: "#181825" },
        },
        crosshair: {
          mode: 3,
          vertLine: { color: "#B8B8FF", width: 1, style: 2 },
          horzLine: { color: "#B8B8FF", width: 1, style: 2 },
        },
        rightPriceScale: {
          borderColor: "#181825",
          textColor: "#ffffff",
        },
        timeScale: {
          borderColor: "#181825",
          timeVisible: true,
          secondsVisible: false,
        },
        width: chartContainerRef.current?.clientWidth || 1090,
        height: chartContainerRef.current?.clientHeight || 725,
      });

      const candlestickSeries = chart.addSeries(CandlestickSeries, {
        upColor: "#22c55e",
        borderUpColor: "#22c55e",
        wickUpColor: "#22c55e",
        downColor: "#ef4444",
        borderDownColor: "#ef4444",
        wickDownColor: "#ef4444",
      });

      chartRef.current = chart;
      seriesRef.current = candlestickSeries;

      const symbol = getServerSymbolFromCoinId(coinId);

      const loadMoreHistoricalData = async (_endTime: number) => { return; };

      const loadInitialData = async () => {
        try {
          setDataLoadingState('loading');
          const symbol = getServerSymbolFromCoinId(coinId);
          const apiTimeframeMap: Record<string, string> = {
            "15": "15",
            "60": "60",
            "240": "240",
            "D": "1440",
          };
          const apiTimeframe = apiTimeframeMap[timeframe] || "15";
          
          const now = Date.now();
          const daysAgo = 90;
          const startTime = now - (daysAgo * 24 * 60 * 60 * 1000);
          
          const data = await apiClient.getChartData(symbol, apiTimeframe, 10000, startTime, now);
          
          if (data && data.length > 0) {
            const chartData: CandlestickData<Time>[] = data.map((candle: { time: number; open: number; high: number; low: number; close: number }) => ({
              time: candle.time as Time,
              open: candle.open,
              high: candle.high,
              low: candle.low,
              close: candle.close,
            }));
            
            candlestickSeries.setData(chartData);
            
            if (chartData.length > 0) {
              const oldestCandle = chartData[0];
              setOldestTimestamp(oldestCandle.time as number);
              const lastCandle = chartData[chartData.length - 1];
              setCurrentPrice(lastCandle.close);
              
              if (onPriceUpdate) {
                onPriceUpdate(lastCandle.close);
              }
            }
            
            setDataLoadingState('idle');
          }
        } catch (error) {
          setDataLoadingState('error');
        }
      };

      loadInitialData();
  
      const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:5001/ws';
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws; 
      
      const subscribeToTimeframe = (currentTimeframe: string) => {
        const intervalMap: Record<string, string> = {
          "15": "15m",
          "60": "1h",
          "240": "4h",
          "D": "1d",
        };
        const interval = intervalMap[currentTimeframe] || "15m";
  
        ws.send(
          JSON.stringify({
            type: "subscribe",
            symbol: symbol.toUpperCase(),
            interval: interval,
          })
        );
      };
      
      ws.onopen = () => {
        subscribeToTimeframe(timeframe);
        
        const intervalMap: Record<string, string> = {
          "15": "15m",
          "60": "1h",
          "240": "4h",
          "D": "1d",
        };
        const interval = intervalMap[timeframe] || "15m";
        currentSubscriptionRef.current = { 
          symbol: symbol.toUpperCase(), 
          interval: interval 
        };
      };
  
      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          
          if (msg.type === "candle" && msg.data) {
            const candleData = msg.data;
            
            const currentSymbol = getServerSymbolFromCoinId(coinId);
            const intervalMap: Record<string, string> = {
              "15": "15m",
              "60": "1h",
              "240": "4h",
              "D": "1d",
            };
            const currentInterval = intervalMap[timeframe] || "15m";
                        
            if (candleData.symbol === currentSymbol && candleData.timeframe === currentInterval) {
              const timestamp = Math.floor(candleData.timestamp / 1000);
              
              const candle: CandlestickData<Time> = {
                time: timestamp as Time,
                open: candleData.open,
                high: candleData.high,
                low: candleData.low,
                close: candleData.close,
              };
                            
              candlestickSeries.update(candle);
              setCurrentPrice(candleData.close);
              setPriceChange(candleData.priceChangePercent);
              
              if (onPriceUpdate) {
                onPriceUpdate(candleData.close);
              }
            }
          }
        } catch (err) {
          // Silent error handling
        }
      };
  
      ws.onerror = () => {
        // Silent error handling
      };
      
      ws.onclose = () => {
        // Silent cleanup
      };
  
      wsCleanupRef.current = () => {
        if (currentSubscriptionRef.current) {
          ws.send(
            JSON.stringify({
              type: "unsubscribe",
              symbol: currentSubscriptionRef.current.symbol,
              interval: currentSubscriptionRef.current.interval,
            })
          );
        }
        ws.close();
      };
  
      const handleResize = () => {
        if (chartContainerRef.current && isComponentMounted) {
          try {
            chart.applyOptions({
              width: chartContainerRef.current.clientWidth,
              height: chartContainerRef.current.clientHeight,
            });
          } catch (e) {
            // Chart might be disposed, ignore
          }
        }
      };
      window.addEventListener("resize", handleResize);
      cleanupFunctions.push(() => window.removeEventListener("resize", handleResize));

      const handleVisibleRangeChange = (timeRange: IRange<Time> | null) => {
        if (!shouldLoadMoreData(timeRange, oldestTimestamp) || !isComponentMounted) return;
        
        if (oldestTimestamp) {
          loadMoreHistoricalData(oldestTimestamp * 1000);
        }
      };
      
      chart.timeScale().subscribeVisibleTimeRangeChange(handleVisibleRangeChange);
      cleanupFunctions.push(() => {
        try {
          chart.timeScale().unsubscribeVisibleTimeRangeChange(handleVisibleRangeChange);
        } catch (e) {
          // Chart might be disposed, ignore
        }
      });
      
      cleanupFunctions.push(() => ws.close());
    }, 100);
  
    return () => {
      isComponentMounted = false;
      clearTimeout(timeoutId);
      
      cleanupFunctions.forEach(cleanup => cleanup());
      
      if (wsCleanupRef.current) {
        try {
          wsCleanupRef.current();
        } catch (e) {
          // Ignore cleanup errors
        }
      }
      
      if (chartRef.current) {
        try {
          chartRef.current.remove();
        } catch (e) {
          // Chart might already be disposed, ignore
        }
        chartRef.current = null;
        seriesRef.current = null;
      }
    };
  }, [coinId, timeframe]);

  useEffect(() => {
    setLoadedChunks(new Set());
    setDataLoadingState('idle');
  }, [coinId, timeframe]);

  const wsRef = useRef<WebSocket | null>(null);
  const currentSubscriptionRef = useRef<{symbol: string, interval: string} | null>(null);
  
  useEffect(() => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    
    const intervalMap: Record<string, string> = {
      "15": "15m",
      "60": "1h",
      "240": "4h",
      "D": "1d",
    };
    const interval = intervalMap[timeframe] || "15m";
    const symbol = getServerSymbolFromCoinId(coinId).toUpperCase();
    
    if (currentSubscriptionRef.current) {
      wsRef.current.send(
        JSON.stringify({
          type: "unsubscribe",
          symbol: currentSubscriptionRef.current.symbol,
          interval: currentSubscriptionRef.current.interval,
        })
      );
    }
    
    wsRef.current.send(
      JSON.stringify({
        type: "subscribe",
        symbol: symbol,
        interval: interval,
      })
    );
    
    currentSubscriptionRef.current = { symbol, interval };
  }, [timeframe, coinId]);

  const generateMockPriceUpdate = (basePrice: number) => {
    const volatility = 0.02;
    const change = (Math.random() - 0.5) * volatility * basePrice;
    return basePrice + change;
  };

  return (
    <div className="relative h-full w-full">
      <div className="absolute top-4 left-4 z-20 bg-black/40 backdrop-blur-sm rounded-lg p-3 border border-[#181825]">
        <div className="flex items-center space-x-3">
          <div>
            <div className="text-white text-lg font-bold">
              ${currentPrice.toFixed(2)}
            </div>
            <div className={`text-sm font-medium ${priceChange >= 0 ? 'text-green-400' : 'text-red-400'}`}>
              {priceChange >= 0 ? '+' : ''}{priceChange.toFixed(2)}%
            </div>
          </div>
          <div className="flex items-center space-x-1">
            {priceChange >= 0 ? (
              <TrendingUp className="w-4 h-4 text-green-400" />
            ) : (
              <TrendingDown className="w-4 h-4 text-red-400" />
            )}
          </div>
        </div>
      </div>

      {error && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/20 backdrop-blur-sm z-10">
          <div className="text-red-400 text-sm">{error}</div>
        </div>
      )}
      
      {isLoadingMore && (
        <div className="absolute top-4 right-4 z-20 bg-black/40 backdrop-blur-sm rounded-lg p-2 border border-[#181825]">
          <div className="flex items-center space-x-2">
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-[#B8B8FF]"></div>
            <div className="text-white text-sm">Loading more data...</div>
          </div>
        </div>
      )}
      
      <div className="relative w-full h-full">
        <div ref={chartContainerRef} style={{ height: "100%", width: "100%"}} />
        
        {dataLoadingState === 'loading' && (
          <div className="absolute top-4 right-4 bg-black/80 text-white px-3 py-2 rounded-lg text-sm flex items-center gap-2">
            <div className="animate-spin w-4 h-4 border-2 border-white border-t-transparent rounded-full"></div>
            Loading data...
          </div>
        )}
        
        {dataLoadingState === 'error' && (
          <div className="absolute top-4 right-4 bg-red-900/80 text-white px-3 py-2 rounded-lg text-sm flex items-center gap-2">
            <Activity className="w-4 h-4" />
            Failed to load data
          </div>
        )}
      </div>
    </div>
  );
}

export const LightweightChartMemo = memo(LightweightChart);


