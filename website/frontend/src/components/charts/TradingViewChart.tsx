"use client"

import { useState, useEffect, useRef } from "react"
import { FaBitcoin, FaEthereum } from "react-icons/fa"
import { SiSolana } from "react-icons/si"
import { BarChart3, Zap } from "lucide-react"
import { cn } from "@/lib/utils"
import { 
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { apiClient } from "@/lib/api-client"
import { LightweightChartMemo } from "./LightweightChart"

interface TradingViewChartComponentProps {
  symbol?: string
  selectedCoin: "ethereum" | "bitcoin" | "solana"
  onCoinChange: (coin: "ethereum" | "bitcoin" | "solana") => void
  selectedTimeframe: "15" | "60" | "240" | "D"
  onTimeframeChange: (timeframe: "15" | "60" | "240" | "D") => void
  tradingMode: "simple" | "advanced"
  onTradingModeChange: (mode: "simple" | "advanced") => void
  onPriceUpdate?: (price: number) => void
}

export const TradingViewChartComponent = ({ 
  symbol = "ETH-PERP",
  selectedCoin,
  onCoinChange,
  selectedTimeframe,
  onTimeframeChange,
  tradingMode,
  onTradingModeChange,
  onPriceUpdate
}: TradingViewChartComponentProps) => {
  const [chartData, setChartData] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const [ohlcData, setOhlcData] = useState({ open: 0, high: 0, low: 0, close: 0, change: 0 })
  const wsCleanupRef = useRef<(() => void) | null>(null)

  const getBasePrice = (coinId: "ethereum" | "bitcoin" | "solana") => {
    switch(coinId) {
      case "ethereum": return 3882;
      case "bitcoin": return 97500;
      case "solana": return 185;
      default: return 3882;
    }
  };

  const generateMockPriceUpdate = (basePrice: number) => {
    const volatility = 0.02;
    const change = (Math.random() - 0.5) * volatility * basePrice;
    return basePrice + change;
  };

  const generateCoinSpecificMockData = (basePrice: number, count: number) => {
    const data: Array<{ time: number; open: number; high: number; low: number; close: number; volume: number }> = [];
    let price = basePrice;
    const baseTime = new Date('2024-01-01').getTime();

    for (let i = 0; i < count; i++) {
      const volatility = Math.random() * 0.05;
      const change = (Math.random() - 0.5) * volatility * price;
      const open = price;
      const close = price + change;
      const high = Math.max(open, close) + Math.random() * volatility * price * 0.5;
      const low = Math.min(open, close) - Math.random() * volatility * price * 0.5;

      data.push({
        time: (baseTime + i * 24 * 60 * 60 * 1000) / 1000,
        open: Number(open.toFixed(2)),
        high: Number(high.toFixed(2)),
        low: Number(low.toFixed(2)),
        close: Number(close.toFixed(2)),
        volume: Math.floor(Math.random() * 1000000) + 100000
      });

      price = close;
    }

    return data;
  };

  const connectPriceWebSocket = () => {
    if (wsCleanupRef.current) {
      wsCleanupRef.current();
    }

    const basePrice = getBasePrice(selectedCoin);
    
    const cleanup = apiClient.connectWebSocket((data: unknown) => {
      const typedData = data as { type?: string; symbol?: string; price?: number; change?: number };
      if (typedData.type === 'price_update' && typedData.symbol === symbol && typedData.price !== undefined) {
        const newPrice = typedData.price;
        const change = typedData.change || 0;
        
        setOhlcData(prev => ({
          ...prev,
          close: newPrice,
          high: Math.max(prev.high, newPrice),
          low: Math.min(prev.low, newPrice),
          change: change
        }));
      }
    });

    wsCleanupRef.current = cleanup;

    const mockInterval = setInterval(() => {
      const newPrice = generateMockPriceUpdate(basePrice);
      const change = ((newPrice - basePrice) / basePrice) * 100;
      
      setOhlcData(prev => ({
        ...prev,
        close: newPrice,
        high: Math.max(prev.high, newPrice),
        low: Math.min(prev.low, newPrice),
        change: change
      }));
    }, 3000);

    return () => {
      clearInterval(mockInterval);
      if (cleanup) cleanup();
    };
  };

  useEffect(() => {
    const fetchChartData = async () => {
      const timeframeMap: Record<string, string> = {
        "15": "15m",
        "60": "1h",
        "240": "4h",
        "D": "1d",
      };
      const apiTimeframe = timeframeMap[selectedTimeframe] || "15m";
      
      try {
        setLoading(true)
        const now = Date.now();
        const daysAgo = 90;
        const startTime = now - (daysAgo * 24 * 60 * 60 * 1000);
        const response = await apiClient.getChartData(symbol, apiTimeframe, 10000, startTime, now)
        
        let data;
        if (Array.isArray(response)) {
          data = response;
        } else if (typeof response === 'object' && response !== null) {
          data = Object.values(response);
        } else {
          throw new Error('Invalid API response format');
        }
        
        setChartData(data)
        
        if (data.length > 0) {
          const latest = data[data.length - 1] as { open: number; high: number; low: number; close: number }
          const first = data[0] as { open: number; high: number; low: number; close: number }
          const high = Math.max(...data.map((d: any) => d.high))
          const low = Math.min(...data.map((d: any) => d.low))
          const change = latest.close - first.open
          
          setOhlcData({
            open: first.open,
            high,
            low,
            close: latest.close,
            change
          })
        }
        setLoading(false)
      } catch (error) {
        const basePrice = getBasePrice(selectedCoin);
        const sampleData = generateCoinSpecificMockData(basePrice, 50);
        setChartData(sampleData)
        
        if (sampleData.length > 0) {
          const latest = sampleData[sampleData.length - 1]
          const first = sampleData[0]
          const high = Math.max(...sampleData.map(d => d.high))
          const low = Math.min(...sampleData.map(d => d.low))
          const change = latest.close - first.open
          
          setOhlcData({
            open: first.open,
            high,
            low,
            close: latest.close,
            change
          })
        }
        
        setLoading(false)
      }
    }

    fetchChartData()

    const priceWsCleanup = connectPriceWebSocket();

    const connectServerChartWebSocket = async () => {
      try {
        await apiClient.connectServerWebSocket();
        
        const interval = selectedTimeframe === '15' ? '15m' : 
                        selectedTimeframe === '60' ? '1h' : 
                        selectedTimeframe === '240' ? '4h' : '1m';
        
        apiClient.subscribeToServerCandle('SOL', interval);
        
        const cleanup = apiClient.onServerMessage((message: unknown) => {
          const typedMessage = message as { type?: string; symbol?: string; data?: unknown };
          if (typedMessage.type === 'candle' && typedMessage.symbol === 'SOL') {
            const candleData = typedMessage.data as {
              time: number;
              open: number;
              high: number;
              low: number;
              close: number;
              volume: number;
            };
            
            const chartCandle = {
              time: candleData.time,
              open: candleData.open,
              high: candleData.high,
              low: candleData.low,
              close: candleData.close,
              volume: candleData.volume
            };
            
            setChartData(prev => [...prev.slice(-99), chartCandle]);
            
            setOhlcData(prev => ({
              ...prev,
              close: candleData.close,
              high: Math.max(prev.high, candleData.high),
              low: Math.min(prev.low, candleData.low),
              change: candleData.close - prev.open
            }));
          }
        });
        
        return cleanup;
      } catch (error) {
        return () => {};
      }
    };
    
    let chartCleanup: (() => void) | null = null;
    connectServerChartWebSocket().then((cleanupFn) => {
      chartCleanup = cleanupFn;
    });

    const interval = setInterval(fetchChartData, 30000)

    return () => {
      priceWsCleanup();
      if (chartCleanup) chartCleanup();
      clearInterval(interval)
    }
  }, [symbol, selectedTimeframe])

  const handleTimeframeChange = (timeframe: "15" | "60" | "240" | "D") => {
    onTimeframeChange(timeframe)
  }

  return (
    <div className="bg-black/20 rounded-2xl border border-[#181825] overflow-hidden transition-all duration-300 w-full h-full">
      <div className="h-12 flex items-center justify-between px-4 border-b border-[#181825] overflow-x-auto [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
        <div className="flex items-center space-x-4 min-w-max">
          <div className="flex items-center space-x-2">
            <span className="text-white font-semibold text-sm">Price Charts</span>
          </div>
          
          <Select value={selectedCoin} onValueChange={(value: "ethereum" | "bitcoin" | "solana") => onCoinChange(value)}>
            <SelectTrigger className="w-[180px] h-10 bg-black/30 backdrop-blur-md border border-[#181825] hover:border-[#B8B8FF]/50 focus:border-[#B8B8FF]/50 focus:ring-[#B8B8FF]/20 text-white text-sm font-medium rounded-xl transition-all duration-500 hover:shadow-lg hover:shadow-[#B8B8FF]/10 px-3">
              <div className="flex items-center gap-2">
                {selectedCoin === "ethereum" && <FaEthereum className="w-4 h-4 text-blue-400" />}
                {selectedCoin === "bitcoin" && <FaBitcoin className="w-4 h-4 text-orange-400" />}
                {selectedCoin === "solana" && <SiSolana className="w-4 h-4 text-purple-400" />}
                <span className="text-sm font-medium">
                  {selectedCoin === "ethereum" && "ETH/USD"}
                  {selectedCoin === "bitcoin" && "BTC/USD"}
                  {selectedCoin === "solana" && "SOL/USD"}
                  {!selectedCoin && "Select asset"}
                </span>
              </div>
            </SelectTrigger>
            <SelectContent className="bg-black/30 backdrop-blur-md border border-[#181825] text-white rounded-xl shadow-2xl shadow-black/50 p-2">
              <SelectItem 
                value="ethereum" 
                className="text-white hover:text-white hover:bg-gradient-to-r hover:from-blue-500/20 hover:to-blue-400/10 focus:text-white focus:bg-gradient-to-r focus:from-blue-500/20 focus:to-blue-400/10 rounded-lg transition-all duration-300 cursor-pointer group data-[highlighted]:bg-gradient-to-r data-[highlighted]:from-blue-500/20 data-[highlighted]:to-blue-400/10 border-0 outline-none !border-none ring-0 focus:ring-0 focus:outline-none"
              >
                <div className="flex items-center gap-3 py-3 px-3">
                  <div className="flex items-center justify-center w-8 h-8 rounded-full bg-blue-500/10 group-hover:bg-blue-500/20 transition-all duration-300">
                    <FaEthereum className="w-5 h-5 text-blue-400 group-hover:text-blue-300 transition-colors duration-300" />
                  </div>
                  <div className="flex flex-col">
                    <span className="font-semibold text-sm text-white">Ethereum</span>
                    <span className="text-xs text-gray-400 font-medium">ETH/USD</span>
                  </div>
                </div>
              </SelectItem>
              <SelectItem 
                value="bitcoin" 
                className="text-white hover:text-white hover:bg-gradient-to-r hover:from-orange-500/20 hover:to-yellow-500/10 focus:text-white focus:bg-gradient-to-r focus:from-orange-500/20 focus:to-yellow-500/10 rounded-lg transition-all duration-300 cursor-pointer group data-[highlighted]:bg-gradient-to-r data-[highlighted]:from-orange-500/20 data-[highlighted]:to-yellow-500/10 border-0 outline-none !border-none ring-0 focus:ring-0 focus:outline-none"
              >
                <div className="flex items-center gap-3 py-3 px-3">
                  <div className="flex items-center justify-center w-8 h-8 rounded-full bg-orange-500/10 group-hover:bg-orange-500/20 transition-all duration-300">
                    <FaBitcoin className="w-5 h-5 text-orange-400 group-hover:text-orange-300 transition-colors duration-300" />
                  </div>
                  <div className="flex flex-col">
                    <span className="font-semibold text-sm text-white">Bitcoin</span>
                    <span className="text-xs text-gray-400 font-medium">BTC/USD</span>
                  </div>
                </div>
              </SelectItem>
              <SelectItem 
                value="solana" 
                className="text-white hover:text-white hover:bg-gradient-to-r hover:from-purple-500/20 hover:to-pink-500/10 focus:text-white focus:bg-gradient-to-r focus:from-purple-500/20 focus:to-pink-500/10 rounded-lg transition-all duration-300 cursor-pointer group data-[highlighted]:bg-gradient-to-r data-[highlighted]:from-purple-500/20 data-[highlighted]:to-pink-500/10 border-0 outline-none !border-none ring-0 focus:ring-0 focus:outline-none"
              >
                <div className="flex items-center gap-3 py-3 px-3">
                  <div className="flex items-center justify-center w-8 h-8 rounded-full bg-purple-500/10 group-hover:bg-purple-500/20 transition-all duration-300">
                    <SiSolana className="w-5 h-5 text-purple-400 group-hover:text-purple-300 transition-colors duration-300" />
                  </div>
                  <div className="flex flex-col">
                    <span className="font-semibold text-sm text-white">Solana</span>
                    <span className="text-xs text-gray-400 font-medium">SOL/USD</span>
                  </div>
                </div>
              </SelectItem>
            </SelectContent>
          </Select>

          <div className="flex items-center space-x-1">
            {(["15", "60", "240", "D"] as const).map((tf) => (
              <button
                key={tf}
                onClick={() => onTimeframeChange(tf)}
                className={cn(
                  "px-3 py-1 text-xs font-medium rounded-lg transition-all duration-300",
                  selectedTimeframe === tf
                    ? "bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/10 text-[#B8B8FF] border border-[#B8B8FF]/30"
                    : "bg-black/20 backdrop-blur-sm border border-[#181825] text-gray-400 hover:text-white hover:border-[#B8B8FF]/30 hover:bg-gradient-to-r hover:from-[#B8B8FF]/10 hover:to-transparent"
                )}
              >
                {tf === "D" ? "1D" : `${tf}m`}
              </button>
            ))}
          </div>

          <div className="flex items-center space-x-1 ml-4">
            <button
              onClick={() => onTradingModeChange("simple")}
              className={cn(
                "px-3 py-1 text-xs font-medium rounded-lg transition-all duration-300 flex items-center gap-1",
                tradingMode === "simple"
                  ? "bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/10 text-[#B8B8FF] border border-[#B8B8FF]/30"
                  : "bg-black/20 backdrop-blur-sm border border-[#181825] text-gray-400 hover:text-white hover:border-[#B8B8FF]/30 hover:bg-gradient-to-r hover:from-[#B8B8FF]/10 hover:to-transparent"
              )}
            >
              <BarChart3 className="w-3 h-3" />
              Simple
            </button>
            <button
              onClick={() => onTradingModeChange("advanced")}
              className={cn(
                "px-3 py-1 text-xs font-medium rounded-lg transition-all duration-300 flex items-center gap-1",
                tradingMode === "advanced"
                  ? "bg-gradient-to-r from-purple-500/20 to-pink-500/10 text-purple-300 border border-purple-500/30"
                  : "bg-black/20 backdrop-blur-sm border border-[#181825] text-gray-400 hover:text-white hover:border-purple-500/30 hover:bg-gradient-to-r hover:from-purple-500/10 hover:to-transparent"
              )}
            >
              <Zap className="w-3 h-3" />
              Router
            </button>
          </div>
        </div>
      </div>
      
      <div className="transition-all duration-300 h-[calc(100vh-200px)]">
        <LightweightChartMemo coinId={selectedCoin} timeframe={selectedTimeframe} onPriceUpdate={onPriceUpdate}/>
      </div>
    </div>
  )
}


