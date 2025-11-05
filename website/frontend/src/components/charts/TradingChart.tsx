"use client"

import { useEffect, useRef, useState } from 'react';
import { createChart, ColorType, IChartApi, ISeriesApi, Time, CandlestickData, CandlestickSeries } from 'lightweight-charts';
import { hyperliquidClient, type Coin, type Timeframe } from '@/lib/hyperliquid-client';
import { TrendingUp, TrendingDown } from 'lucide-react';

interface TradingChartProps {
  coin: Coin;
  onPriceUpdate?: (price: number) => void;
  onCoinChange?: (coin: Coin) => void;
}

export default function TradingChart({ coin, onPriceUpdate, onCoinChange }: TradingChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null);
  const [selectedTimeframe, setSelectedTimeframe] = useState<Timeframe>('15m');
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [priceChange, setPriceChange] = useState<number>(0);
  const [loading, setLoading] = useState(true);
  const wsCleanupRef = useRef<(() => void) | null>(null);

  const timeframes: { value: Timeframe; label: string }[] = [
    { value: '1m', label: '1M' },
    { value: '15m', label: '15M' },
    { value: '1h', label: '1H' },
    { value: '12h', label: '12H' },
  ];

  const coins: { value: Coin; label: string }[] = [
    { value: 'SOL', label: 'SOL' },
    { value: 'ETH', label: 'ETH' },
    { value: 'BTC', label: 'BTC' },
  ];

  useEffect(() => {
    if (!chartContainerRef.current) {
      return;
    }

    // Use ResizeObserver to get accurate dimensions
    const updateChartSize = () => {
      if (!chartContainerRef.current || !chartRef.current) return;
      
      const containerWidth = chartContainerRef.current.clientWidth || chartContainerRef.current.offsetWidth;
      const containerHeight = chartContainerRef.current.clientHeight || chartContainerRef.current.offsetHeight;
      
      if (containerWidth > 0 && containerHeight > 0) {
        chartRef.current.applyOptions({
          width: containerWidth,
          height: containerHeight,
          timeScale: {
            visible: true,
            timeVisible: true,
          },
        });
      }
    };

    // Initial size calculation - use actual container dimensions
    // Use requestAnimationFrame to ensure layout is complete
    const getInitialSize = () => {
      if (chartContainerRef.current) {
        const rect = chartContainerRef.current.getBoundingClientRect();
        // Use actual container size, with minimums
        return {
          width: Math.max(rect.width || 0, 300),
          height: Math.max(rect.height || 0, 300)
        };
      }
      // Fallback: use viewport-based calculation for initial render
      const isMobile = typeof window !== 'undefined' && window.innerWidth < 1024;
      return {
        width: Math.max(isMobile ? window.innerWidth * 0.95 : window.innerWidth * 0.6, 300),
        height: Math.max(isMobile ? window.innerHeight * 0.4 : (window.innerHeight - 200) * 0.8, 300)
      };
    };

    // Get initial size, but will be updated by ResizeObserver
    const initialSize = getInitialSize();
    const containerWidth = initialSize.width;
    const containerHeight = initialSize.height;
    
    let isMounted = true;

    // Create chart with responsive sizing
    const chart = createChart(chartContainerRef.current, {
      width: Math.max(containerWidth, 300),
      height: Math.max(containerHeight, 300),
      layout: {
        background: { type: ColorType.Solid, color: '#0a0e1a' },
        textColor: '#9ca3af',
      },
      grid: {
        vertLines: { color: '#1f2937' },
        horzLines: { color: '#1f2937' },
      },
      crosshair: {
        mode: 0,
      },
      rightPriceScale: {
        borderColor: '#1f2937',
      },
      timeScale: {
        borderColor: '#1f2937',
        timeVisible: true,
        secondsVisible: false,
        rightOffset: 12,
        visible: true,
        fixLeftEdge: false,
        fixRightEdge: false,
        lockVisibleTimeRangeOnResize: false,
        allowShiftVisibleRangeOnWhitespaceReplacement: true,
        allowBoldLabels: true,
        shiftVisibleRangeOnNewBar: false,
        barSpacing: 3,
        minBarSpacing: 0.5,
      },
    });

    // Add candlestick series
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: '#10b981',
      downColor: '#ef4444',
      borderUpColor: '#10b981',
      borderDownColor: '#ef4444',
      wickUpColor: '#10b981',
      wickDownColor: '#ef4444',
    });

    chartRef.current = chart;
    seriesRef.current = candlestickSeries;

    // Load historical data
    const loadHistoricalData = async () => {
      try {
        setLoading(true);
        // Reset price while loading to prevent stale data
        setCurrentPrice(0);
        setPriceChange(0);
        
        // Calculate time range based on timeframe
        const now = Date.now();
        let daysBack = 7; // Default
        
        switch (selectedTimeframe) {
          case '1m':
            daysBack = 1; // 1 day for 1min candles
            break;
          case '15m':
            daysBack = 7; // 7 days for 15min candles
            break;
          case '1h':
            daysBack = 30; // 30 days for 1h candles
            break;
          case '12h':
            daysBack = 90; // 90 days for 12h candles
            break;
        }

        const startTime = now - (daysBack * 24 * 60 * 60 * 1000);
        
        console.log(`Loading ${coin} chart data for ${selectedTimeframe} timeframe...`);
        console.log(`API URL: ${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'}`);
        console.log(`Requesting: ${coin} with timeframe ${selectedTimeframe}`);
        
        const candles = await hyperliquidClient.getCandles(
          coin,
          selectedTimeframe,
          startTime,
          now
        );

        console.log(`Received ${candles?.length || 0} candles from API`);

        if (!isMounted) {
          console.log('Component unmounted, skipping data update');
          return;
        }

        if (!candles || candles.length === 0) {
          console.warn(`No candles received for ${coin} ${selectedTimeframe}`);
          setLoading(false);
          return;
        }

        console.log(`Processing ${candles.length} candles for ${coin}`);

        // Convert to chart format
        // Backend returns time in seconds (from Hyperliquid API)
        const chartData: CandlestickData[] = candles.map(c => {
          // Convert time from milliseconds to seconds if needed, or use as-is if already in correct format
          let timeValue: Time;
          if (typeof c.time === 'number') {
            // If time is in milliseconds, convert to seconds
            timeValue = (c.time > 1000000000000 ? c.time / 1000 : c.time) as Time;
          } else {
            timeValue = c.time as Time;
          }
          
          // Ensure prices are valid numbers
          const open = Number(c.open);
          const high = Number(c.high);
          const low = Number(c.low);
          const close = Number(c.close);
          
          // Validate price ranges (prevent obviously wrong data)
          if (isNaN(open) || isNaN(high) || isNaN(low) || isNaN(close)) {
            console.warn(`Invalid price data in candle:`, c);
            return null;
          }
          
          // Log if prices seem unusually high (might indicate format issue)
          // But don't filter - accept Hyperliquid's data format
          const expectedMaxPrice: Record<Coin, number> = {
            'SOL': 500,
            'ETH': 15000,
            'BTC': 250000
          };
          
          if (close > expectedMaxPrice[coin] * 100) {
            console.warn(`Price ${close} for ${coin} is very high - checking if conversion needed`);
          }
          
          return {
            time: timeValue,
            open,
            high,
            low,
            close,
          };
        }).filter((c): c is CandlestickData => {
          // Only filter out null/invalid entries, not based on price
          return c !== null;
        });

        if (chartData.length === 0) {
          console.error('No valid candles after filtering');
          setLoading(false);
          return;
        }

        console.log(`Setting ${chartData.length} candles on chart. Price range: $${Math.min(...chartData.map(c => c.low)).toFixed(2)} - $${Math.max(...chartData.map(c => c.high)).toFixed(2)}`);
        
        candlestickSeries.setData(chartData);
        console.log('Chart data set successfully');

        // Update price display
        if (chartData.length > 0) {
          const lastCandle = chartData[chartData.length - 1];
          const firstCandle = chartData[0];
          
          const lastPrice = lastCandle.close;
          const firstPrice = firstCandle.close;
          const change = lastPrice - firstPrice;
          const changePercent = (change / firstCandle.open) * 100;
          
          console.log(`${coin} price: $${lastPrice.toFixed(2)}, change: ${changePercent.toFixed(2)}%`);
          
          setCurrentPrice(lastPrice);
          setPriceChange(changePercent);
          
          if (onPriceUpdate) {
            onPriceUpdate(lastPrice);
          }
        }

        setLoading(false);
        console.log('Chart data loading complete');

        // Connect WebSocket for real-time updates
        if (!hyperliquidClient.isConnected()) {
          await hyperliquidClient.connect();
        }

        // Subscribe to real-time updates
        if (wsCleanupRef.current) {
          wsCleanupRef.current();
        }

        wsCleanupRef.current = hyperliquidClient.subscribe(
          coin,
          selectedTimeframe,
          (newCandle) => {
            if (!isMounted) return;
            
            candlestickSeries.update({
              time: newCandle.time as Time,
              open: newCandle.open,
              high: newCandle.high,
              low: newCandle.low,
              close: newCandle.close,
            });

            setCurrentPrice(newCandle.close);
            
            if (onPriceUpdate) {
              onPriceUpdate(newCandle.close);
            }
          }
        );

      } catch (error) {
        console.error('Failed to load chart data:', error);
        setLoading(false);
        // Show error message to user
        if (chartContainerRef.current) {
          const errorDiv = document.createElement('div');
          errorDiv.className = 'absolute inset-0 flex items-center justify-center bg-gray-900/80 z-10 text-red-400 text-sm';
          errorDiv.textContent = 'Failed to load chart data. Please check your connection.';
          chartContainerRef.current.appendChild(errorDiv);
        }
      }
    };

    loadHistoricalData();

    // Handle resize with debounce for better performance
    let resizeTimeout: NodeJS.Timeout;
    const handleResize = () => {
      clearTimeout(resizeTimeout);
      resizeTimeout = setTimeout(() => {
        if (chartContainerRef.current && chart && isMounted) {
          const rect = chartContainerRef.current.getBoundingClientRect();
          if (rect.width > 0 && rect.height > 0) {
            chart.applyOptions({
              width: Math.max(rect.width, 300),
              height: Math.max(rect.height, 300),
              timeScale: {
                visible: true,
                timeVisible: true,
              },
            });
          }
        }
      }, 100);
    };

    window.addEventListener('resize', handleResize);

    // Use ResizeObserver to detect container size changes (for panel resizing and viewport changes)
    const resizeObserver = new ResizeObserver((entries) => {
      if (!isMounted || !chart) return;
      
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          chart.applyOptions({ 
            width: Math.max(width, 300),
            height: Math.max(height, 300),
            timeScale: {
              visible: true,
              timeVisible: true,
            },
          });
        }
      }
    });

    if (chartContainerRef.current) {
      resizeObserver.observe(chartContainerRef.current);
    }

    // Also listen to window resize for better responsiveness
    const handleWindowResize = () => {
      updateChartSize();
    };
    
    window.addEventListener('resize', handleWindowResize);

    return () => {
      isMounted = false;
      clearTimeout(resizeTimeout);
      window.removeEventListener('resize', handleResize);
      window.removeEventListener('resize', handleWindowResize);
      resizeObserver.disconnect();
      if (wsCleanupRef.current) {
        wsCleanupRef.current();
        wsCleanupRef.current = null;
      }
      if (chart) {
        chart.remove();
        chartRef.current = null;
        seriesRef.current = null;
      }
    };
  }, [coin, selectedTimeframe]); // Removed onPriceUpdate from dependencies to prevent unnecessary re-renders

  return (
    <div className="flex flex-col h-full w-full bg-gray-900 rounded-lg border border-gray-800">
      {/* Header - Responsive Layout */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between px-3 sm:px-4 py-2.5 sm:py-3 border-b border-gray-800 gap-2 sm:gap-0">
        {/* Price Info */}
        <div className="flex items-center gap-2 sm:gap-3 min-w-0">
          <h3 className="text-sm sm:text-lg font-bold text-white whitespace-nowrap">{coin}/USDC</h3>
          <div className="flex items-center gap-1 sm:gap-1.5">
            <span className="text-base sm:text-xl font-bold text-white">
              ${currentPrice.toFixed(2)}
            </span>
            {priceChange !== 0 && (
              <div className={`flex items-center gap-0.5 text-[10px] sm:text-xs font-semibold ${
                priceChange >= 0 ? 'text-green-500' : 'text-red-500'
              }`}>
                {priceChange >= 0 ? (
                  <TrendingUp className="w-3 h-3" />
                ) : (
                  <TrendingDown className="w-3 h-3" />
                )}
                {Math.abs(priceChange).toFixed(2)}%
              </div>
            )}
          </div>
        </div>
        
        {/* Controls - Horizontal scroll on mobile */}
        <div className="flex items-center gap-2 overflow-x-auto scrollbar-hide -mx-3 px-3 sm:mx-0 sm:px-0 sm:overflow-visible">
          {/* Coin Selector */}
          <div className="flex gap-0.5 bg-gray-800 rounded-md p-0.5 shrink-0">
            {coins.map((c) => (
              <button
                key={c.value}
                onClick={() => onCoinChange?.(c.value)}
                className={`px-2.5 sm:px-3 py-1 sm:py-1.5 text-[11px] sm:text-xs font-medium rounded-sm transition-all touch-manipulation ${
                  coin === c.value
                    ? 'bg-blue-600 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50 active:bg-gray-700'
                }`}
              >
                {c.label}
              </button>
            ))}
          </div>

          {/* Timeframe Selector */}
          <div className="flex gap-0.5 bg-gray-800 rounded-md p-0.5 shrink-0">
            {timeframes.map((tf) => (
              <button
                key={tf.value}
                onClick={() => setSelectedTimeframe(tf.value)}
                className={`px-2.5 sm:px-3 py-1 sm:py-1.5 text-[11px] sm:text-xs font-medium rounded-sm transition-all touch-manipulation ${
                  selectedTimeframe === tf.value
                    ? 'bg-blue-600 text-white shadow-sm'
                    : 'text-gray-400 hover:text-white hover:bg-gray-700/50 active:bg-gray-700'
                }`}
              >
                {tf.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Chart Container - Responsive to viewport with proper padding for time scale */}
      <div 
        ref={chartContainerRef}
        style={{ 
          width: '100%',
          height: '100%',
          minHeight: '300px',
          position: 'relative',
        }}
      >
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-gray-900/80 z-10">
            <div className="text-center">
              <div className="animate-spin rounded-full h-8 w-8 sm:h-12 sm:w-12 border-b-2 border-blue-600 mx-auto mb-3 sm:mb-4"></div>
              <p className="text-xs sm:text-sm text-gray-400">Loading chart data...</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

