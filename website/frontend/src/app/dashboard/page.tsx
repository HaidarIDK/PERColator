"use client"

import { useState, useRef, useEffect } from 'react';
import TradingChart from '@/components/charts/TradingChart';
import { OrderForm } from '@/components/trading/OrderForm';
import { OrderBook } from '@/components/trading/OrderBook';
import { PastTrades } from '@/components/trading/PastTrades';
import { TestnetBanner } from '@/components/TestnetBanner';
import { StatusFooter } from '@/components/StatusFooter';
import { AMMInterface } from '@/components/trading/AMMInterface';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useWallet } from '@solana/wallet-adapter-react';
import { Home, Plus, Terminal, Wallet } from 'lucide-react';
import Link from 'next/link';

type Coin = 'SOL' | 'ETH' | 'BTC';

export default function DashboardPage() {
  const { publicKey } = useWallet();
  const [selectedCoin, setSelectedCoin] = useState<Coin>('SOL');
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [rightPanelWidth, setRightPanelWidth] = useState(420); // Narrower default width for more chart space
  const [isResizing, setIsResizing] = useState(false);
  const [isMounted, setIsMounted] = useState(false);
  const [tradingMode, setTradingMode] = useState<'orderbook' | 'amm' | 'lp'>('orderbook');
  const containerRef = useRef<HTMLDivElement>(null);

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
      
      // Clamp between 280px and 700px (narrower range for more compact panel)
      const clampedWidth = Math.max(280, Math.min(700, newWidth));
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

  // Map Coin type to string for OrderForm
  const getCoinString = (coin: Coin): "ethereum" | "bitcoin" | "solana" => {
    switch(coin) {
      case 'ETH': return 'ethereum';
      case 'BTC': return 'bitcoin';
      case 'SOL': return 'solana';
    }
  };

  // Map Coin type for symbol
  const getSymbol = (coin: Coin): string => {
    switch(coin) {
      case 'ETH': return 'ETH-USDC';
      case 'BTC': return 'BTC-USDC';
      case 'SOL': return 'SOL-USDC';
    }
  };

  return (
    <div className="h-screen bg-black text-white flex flex-col overflow-hidden [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
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
              <h1 className="text-base sm:text-xl font-bold bg-gradient-to-r from-blue-500 to-purple-600 bg-clip-text text-transparent">
                Percolator
              </h1>
              <span className="px-1.5 sm:px-2 py-0.5 sm:py-1 text-[10px] sm:text-xs font-semibold bg-blue-600 rounded">
                DEVNET
          </span>
        </div>
            <div className="flex items-center gap-2 sm:gap-3">
              <Link href="/portfolio">
                <button className="px-2 sm:px-3 py-1.5 sm:py-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 border border-[#B8B8FF]/30 hover:border-[#B8B8FF]/50 text-[#B8B8FF] text-xs sm:text-sm font-bold transition-all flex items-center gap-1.5">
                  <Wallet className="w-4 h-4" />
                  <span className="hidden sm:inline">Portfolio</span>
                </button>
              </Link>
              <WalletMultiButton />
      </div>
            </div>
                    </div>
      </header>

      {/* Main Content - Responsive Layout */}
      <div 
        ref={containerRef} 
        className="flex-1 flex flex-col lg:flex-row overflow-hidden [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]"
        style={{ 
          height: 'calc(100vh - 140px)',
          minHeight: 0
        }}
      >
        {/* Chart Area - Fixed height on mobile, flex on desktop */}
        <div 
          className="w-full lg:flex-1 flex flex-col shrink-0"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { 
                width: `calc(100% - ${rightPanelWidth}px)`, 
                height: '100%',
                minHeight: 0
              } 
            : { 
                height: '50%',
                minHeight: '350px',
                maxHeight: '50%',
                marginBottom: '28px'
              }
          }
        >
          <div className="w-full h-full p-2 sm:p-3" style={{ paddingBottom: '0px' }}>
            <TradingChart 
              coin={selectedCoin} 
              onPriceUpdate={setCurrentPrice}
              onCoinChange={setSelectedCoin}
            />
              </div>
            </div>

        {/* Resizer Handle - Desktop Only */}
        <div
          onMouseDown={() => setIsResizing(true)}
          className="hidden lg:block w-1 bg-gray-800 hover:bg-blue-600 cursor-col-resize transition-colors relative group shrink-0"
        >
          <div className="absolute inset-y-0 -left-1 -right-1" />
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-1 h-12 bg-gray-600 rounded group-hover:bg-blue-500 transition-colors" />
                </div>

        {/* Trading Panel - Below chart on mobile, side panel on desktop */}
        <div 
          className="w-full lg:w-auto border-t lg:border-t-0 lg:border-l border-gray-800 bg-gray-900 p-2 sm:p-3 overflow-y-auto shrink-0 lg:min-w-[280px] lg:max-w-[700px] [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { 
                width: `${rightPanelWidth}px`,
                height: '100%',
                maxHeight: '100%'
              } 
            : { 
                height: 'calc(50% - 28px)',
                minHeight: '350px',
                maxHeight: 'calc(50% - 28px)',
                marginTop: '28px'
              }
          }
        >
          <div className="space-y-2 sm:space-y-3">
            {/* Trading Panel Header */}
                    <div>
              <h2 className="text-xs sm:text-sm lg:text-base font-bold text-white mb-0.5 truncate">Trade {selectedCoin}/USDC</h2>
              <p className="text-[10px] sm:text-xs text-gray-400">Price: ${currentPrice.toFixed(2)}</p>
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
      </div>
      
            {/* Trading Interface based on mode */}
            {tradingMode === 'orderbook' ? (
              <>
                {/* Order Form */}
                <OrderForm 
                  coin={getCoinString(selectedCoin)} 
                  currentPrice={currentPrice} 
                />

                {/* Order Book */}
                <OrderBook symbol={getSymbol(selectedCoin)} walletAddress={publicKey?.toBase58()} />
                    </>
                  ) : (
                    <>
                {/* AMM Interface */}
                <AMMInterface 
                  selectedCoin={getCoinString(selectedCoin)} 
                  mode="swap"
                  showToast={() => {}}
                  chartCurrentPrice={currentPrice}
                />
              </>
            )}
          </div>
          </div>
        </div>
        
      {/* Status Footer - Hidden on mobile, shown on desktop */}
      <div className="hidden lg:block shrink-0">
        <StatusFooter />
    </div>
    </div>
  );
}
