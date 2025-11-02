"use client"

import { useState, useRef, useEffect } from 'react';
import TradingChart from '@/components/TradingChart';
import { PortfolioPanel } from '@/components/PortfolioPanel';
import { OrderForm } from '@/components/OrderForm';
import { OrderBook } from '@/components/OrderBook';
import { RecentTrades } from '@/components/RecentTrades';
import { PastTrades } from '@/components/PastTrades';
import { TestnetBanner } from '@/components/TestnetBanner';
import { StatusFooter } from '@/components/StatusFooter';
import { AMMInterface } from '@/components/AMMInterface';
import { LPPanel } from '@/components/LPPanel';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useWallet } from '@solana/wallet-adapter-react';
import { Home, Plus } from 'lucide-react';
import Link from 'next/link';

type Coin = 'SOL' | 'ETH' | 'BTC';

export default function DashboardPage() {
  const { publicKey } = useWallet();
  const [selectedCoin, setSelectedCoin] = useState<Coin>('SOL');
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [rightPanelWidth, setRightPanelWidth] = useState(600); // Bigger default width
  const [isResizing, setIsResizing] = useState(false);
  const [isMounted, setIsMounted] = useState(false);
  const [tradingMode, setTradingMode] = useState<'orderbook' | 'amm' | 'lp'>('orderbook');
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
      
      // Clamp between 320px and 800px (matches CSS constraints)
      const clampedWidth = Math.max(320, Math.min(800, newWidth));
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
    <div className="min-h-screen bg-black text-white flex flex-col">
      {/* Testnet Warning Banner */}
      <TestnetBanner />

      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-900 shrink-0">
        <div className="px-3 sm:px-4 py-2 sm:py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 sm:gap-3">
              <Link href="/">
                <button className="px-2 sm:px-3 py-1.5 sm:py-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 border border-[#B8B8FF]/30 hover:border-[#B8B8FF]/50 text-[#B8B8FF] text-xs sm:text-sm font-bold transition-all flex items-center gap-1.5">
                  <Home className="w-4 h-4" />
                  <span className="hidden sm:inline">Home</span>
                </button>
              </Link>
              <Link href="/create-slab">
                <button className="px-2 sm:px-3 py-1.5 sm:py-2 rounded-lg bg-green-600/20 hover:bg-green-600/30 border border-green-500/30 hover:border-green-500/50 text-green-400 text-xs sm:text-sm font-bold transition-all flex items-center gap-1.5">
                  <Plus className="w-4 h-4" />
                  <span className="hidden md:inline">Create Slab</span>
                </button>
              </Link>
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
      <div ref={containerRef} className="flex-1 flex flex-col lg:flex-row overflow-y-auto lg:overflow-hidden">
        {/* Chart Area - Full viewport on mobile, flex on desktop */}
        <div 
          className="w-full p-2 sm:p-4 lg:flex-1 lg:min-h-0"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { width: `calc(100% - ${rightPanelWidth}px)`, height: '100%' } 
            : { height: 'calc(100vh - 180px)', minHeight: '1000px' }
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

        {/* Trading Panel - Full width below chart on mobile, fixed width on desktop */}
        <div 
          className="w-full lg:w-auto border-t lg:border-t-0 lg:border-l border-gray-800 bg-gray-900 p-3 sm:p-4 overflow-y-auto lg:max-h-none min-h-screen lg:min-h-0 lg:min-w-[320px] lg:max-w-[800px]"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { width: `${rightPanelWidth}px` } 
            : undefined
          }
        >
          <div className="space-y-3 sm:space-y-4">
            {/* Trading Panel Header */}
            <div>
              <h2 className="text-sm sm:text-base lg:text-lg font-bold text-white mb-1 truncate">Trade {selectedCoin}/USDC</h2>
              <p className="text-xs text-gray-400">Price: ${currentPrice.toFixed(2)}</p>
            </div>

            {/* Trading Mode Toggle */}
            <div className="flex items-center gap-1 bg-black/40 p-1 rounded-lg">
              <button
                onClick={() => setTradingMode('orderbook')}
                className={`flex-1 py-2 px-1 rounded-lg text-[9px] sm:text-[10px] lg:text-xs font-bold transition-all whitespace-nowrap ${
                  tradingMode === 'orderbook'
                    ? 'bg-gradient-to-r from-[#B8B8FF]/30 to-purple-500/20 text-white border border-[#B8B8FF]/50'
                    : 'bg-transparent text-gray-400 hover:text-white'
                }`}
              >
                Order Book
              </button>
              <button
                onClick={() => setTradingMode('amm')}
                className={`flex-1 py-2 px-1 rounded-lg text-[9px] sm:text-[10px] lg:text-xs font-bold transition-all whitespace-nowrap ${
                  tradingMode === 'amm'
                    ? 'bg-gradient-to-r from-[#B8B8FF]/30 to-purple-500/20 text-white border border-[#B8B8FF]/50'
                    : 'bg-transparent text-gray-400 hover:text-white'
                }`}
              >
                AMM
              </button>
              <button
                onClick={() => setTradingMode('lp')}
                className={`flex-1 py-2 px-1 rounded-lg text-[9px] sm:text-[10px] lg:text-xs font-bold transition-all whitespace-nowrap ${
                  tradingMode === 'lp'
                    ? 'bg-gradient-to-r from-purple-500/30 to-blue-500/20 text-white border border-purple-500/50'
                    : 'bg-transparent text-gray-400 hover:text-white'
                }`}
              >
                LP Ops
              </button>
            </div>

            {/* Portfolio Management */}
            <PortfolioPanel />

            {/* Trading Interface based on mode */}
            {tradingMode === 'orderbook' ? (
              <>
                {/* Order Form */}
                <OrderForm coin={selectedCoin} currentPrice={currentPrice} />

                {/* Order Book */}
                <OrderBook symbol={`${selectedCoin}-USD`} walletAddress={publicKey?.toBase58()} />

                {/* Recent Trades */}
                <RecentTrades symbol={`${selectedCoin}-USD`} walletAddress={publicKey?.toBase58()} />
              </>
            ) : tradingMode === 'amm' ? (
              <>
                {/* AMM Interface */}
                <AMMInterface coin={selectedCoin} />
              </>
            ) : (
              <>
                {/* LP Operations */}
                <LPPanel coin={selectedCoin} />
              </>
            )}

            {/* Past Trades History */}
            <PastTrades symbol={`${selectedCoin}-USD`} />
          </div>
        </div>
      </div>

      {/* Status Footer */}
      <StatusFooter />
    </div>
  );
}

