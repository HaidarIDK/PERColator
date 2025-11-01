"use client"

import { useEffect, useRef, useState } from 'react';
import { createChart, ColorType, IChartApi, ISeriesApi, Time, CandlestickData, CandlestickSeries } from 'lightweight-charts';
import { hyperliquidClient, type Coin, type Timeframe } from '@/lib/hyperliquid-client';
import { TrendingUp, TrendingDown } from 'lucide-react';

interface TradingChartProps {
  coin: Coin;
  onPriceUpdate?: (price: number) => void;
}

export default function TradingChart({ coin, onPriceUpdate }: TradingChartProps) {
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

  useEffect(() => {
    if (!chartContainerRef.current) return;

    let isMounted = true;

    // Create chart
    const chart = createChart(chartContainerRef.current, {
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight,
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
        
        console.log(`Loading ${coin} ${selectedTimeframe} candles...`);
        
        const candles = await hyperliquidClient.getCandles(
          coin,
          selectedTimeframe,
          startTime,
          now
        );

        if (!isMounted || !candles || candles.length === 0) return;

        // Convert to chart format
        const chartData: CandlestickData[] = candles.map(c => ({
          time: c.time as Time,
          open: c.open,
          high: c.high,
          low: c.low,
          close: c.close,
        }));

        candlestickSeries.setData(chartData);

        // Update price display
        if (chartData.length > 0) {
          const lastCandle = chartData[chartData.length - 1];
          const firstCandle = chartData[0];
          
          setCurrentPrice(lastCandle.close);
          const change = lastCandle.close - firstCandle.close;
          const changePercent = (change / firstCandle.close) * 100;
          setPriceChange(changePercent);
          
          if (onPriceUpdate) {
            onPriceUpdate(lastCandle.close);
          }
        }

        console.log(`Loaded ${chartData.length} candles`);
        setLoading(false);

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
      }
    };

    loadHistoricalData();

    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current && chart) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      isMounted = false;
      window.removeEventListener('resize', handleResize);
      if (wsCleanupRef.current) {
        wsCleanupRef.current();
      }
      chart.remove();
    };
  }, [coin, selectedTimeframe, onPriceUpdate]);

  return (
    <div className="flex flex-col h-full bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
        <div className="flex items-center gap-4">
          <h3 className="text-xl font-bold text-white">{coin}/USDC</h3>
          <div className="flex items-center gap-2">
            <span className="text-2xl font-bold text-white">
              ${currentPrice.toFixed(2)}
            </span>
            {priceChange !== 0 && (
              <div className={`flex items-center gap-1 text-sm font-semibold ${
                priceChange >= 0 ? 'text-green-500' : 'text-red-500'
              }`}>
                {priceChange >= 0 ? (
                  <TrendingUp className="w-4 h-4" />
                ) : (
                  <TrendingDown className="w-4 h-4" />
                )}
                {Math.abs(priceChange).toFixed(2)}%
              </div>
            )}
          </div>
        </div>
        
        {/* Timeframe Selector */}
        <div className="flex gap-1 bg-gray-800 rounded-lg p-1">
          {timeframes.map((tf) => (
            <button
              key={tf.value}
              onClick={() => setSelectedTimeframe(tf.value)}
              className={`px-4 py-2 text-sm font-semibold rounded transition-colors ${
                selectedTimeframe === tf.value
                  ? 'bg-blue-600 text-white'
                  : 'text-gray-400 hover:text-white hover:bg-gray-700'
              }`}
            >
              {tf.label}
            </button>
          ))}
        </div>
      </div>

      {/* Chart Container */}
      <div className="flex-1 relative" ref={chartContainerRef}>
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-gray-900/80 z-10">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
              <p className="text-gray-400">Loading chart data...</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

