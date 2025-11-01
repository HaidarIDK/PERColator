"use client"

import { useState, useRef, useEffect } from 'react';
import TradingChart from '@/components/TradingChart';
import { PortfolioPanel } from '@/components/PortfolioPanel';
import { OrderForm } from '@/components/OrderForm';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

type Coin = 'SOL' | 'ETH' | 'BTC';

export default function DashboardPage() {
  const [selectedCoin, setSelectedCoin] = useState<Coin>('SOL');
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [rightPanelWidth, setRightPanelWidth] = useState(600); // Bigger default width
  const [isResizing, setIsResizing] = useState(false);
  const [isMounted, setIsMounted] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const coins: { value: Coin; label: string }[] = [
    { value: 'SOL', label: 'Solana' },
    { value: 'ETH', label: 'Ethereum' },
    { value: 'BTC', label: 'Bitcoin' },
  ];

  // Handle client-side mounting
  useEffect(() => {
    setIsMounted(true);
  }, []);

  // Handle resize
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing || !containerRef.current) return;
      
      const containerRect = containerRef.current.getBoundingClientRect();
      const newWidth = containerRect.right - e.clientX;
      
      // Clamp between 300px and 800px
      const clampedWidth = Math.max(300, Math.min(800, newWidth));
      setRightPanelWidth(clampedWidth);
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isResizing]);

  // Don't render until mounted (prevents hydration mismatch)
  if (!isMounted) {
    return (
      <div className="min-h-screen bg-gray-950 text-white flex items-center justify-center">
        <div className="text-gray-400">Loading...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-950 text-white flex flex-col">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-900 shrink-0">
        <div className="px-3 sm:px-4 py-2 sm:py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 sm:gap-3">
              <h1 className="text-base sm:text-xl font-bold bg-gradient-to-r from-blue-500 to-purple-600 bg-clip-text text-transparent">
                Percolator
              </h1>
              <span className="px-1.5 sm:px-2 py-0.5 sm:py-1 text-[10px] sm:text-xs font-semibold bg-blue-600 rounded">
                DEVNET
              </span>
            </div>
            <div className="flex items-center gap-2 sm:gap-3">
              <WalletMultiButton />
            </div>
          </div>
        </div>
      </header>

      {/* Main Content - Responsive Layout */}
      <div ref={containerRef} className="flex-1 flex flex-col lg:flex-row overflow-hidden">
        {/* Chart Area - Full width on mobile, flex on desktop */}
        <div 
          className="flex-1 p-2 sm:p-4 min-h-[400px] lg:min-h-0"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { width: `calc(100% - ${rightPanelWidth}px)` } 
            : undefined
          }
        >
          <TradingChart 
            coin={selectedCoin} 
            onPriceUpdate={setCurrentPrice}
            onCoinChange={setSelectedCoin}
          />
        </div>

        {/* Resizer Handle - Desktop Only */}
        <div
          onMouseDown={() => setIsResizing(true)}
          className="hidden lg:block w-1 bg-gray-800 hover:bg-blue-600 cursor-col-resize transition-colors relative group"
        >
          <div className="absolute inset-y-0 -left-1 -right-1" />
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-1 h-12 bg-gray-600 rounded group-hover:bg-blue-500 transition-colors" />
        </div>

        {/* Trading Panel - Full width on mobile, fixed width on desktop */}
        <div 
          className="w-full lg:w-auto border-t lg:border-t-0 lg:border-l border-gray-800 bg-gray-900 p-3 sm:p-4 overflow-y-auto max-h-[50vh] lg:max-h-none"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { width: `${rightPanelWidth}px` } 
            : undefined
          }
        >
          <div className="space-y-3 sm:space-y-4">
            {/* Trading Panel Header */}
            <div>
              <h2 className="text-base sm:text-lg font-bold text-white mb-1">Trade {selectedCoin}/USDC</h2>
              <p className="text-xs sm:text-sm text-gray-400">Current Price: ${currentPrice.toFixed(2)}</p>
            </div>

            {/* Portfolio Management */}
            <PortfolioPanel />

            {/* Order Form */}
            <OrderForm coin={selectedCoin} currentPrice={currentPrice} />

            {/* Order Book - Coming Soon */}
            <div className="bg-gray-800 rounded-lg p-3 sm:p-4 border border-gray-700">
              <h3 className="text-xs sm:text-sm font-semibold text-white mb-2">Order Book</h3>
              <p className="text-[10px] sm:text-xs text-gray-400 text-center py-3 sm:py-4">Coming soon</p>
            </div>

            {/* Recent Trades - Coming Soon */}
            <div className="bg-gray-800 rounded-lg p-3 sm:p-4 border border-gray-700">
              <h3 className="text-xs sm:text-sm font-semibold text-white mb-2">Recent Trades</h3>
              <p className="text-[10px] sm:text-xs text-gray-400 text-center py-3 sm:py-4">Coming soon</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

