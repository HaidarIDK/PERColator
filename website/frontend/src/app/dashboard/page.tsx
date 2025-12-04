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
import { type Coin } from '@/lib/hyperliquid-client';

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
  const getCoinString = (coin: Coin): "ethereum" | "bitcoin" | "solana" | "jupiter" | "bonk" | "dogwifhat" => {
    switch(coin) {
      case 'ETH': return 'ethereum';
      case 'BTC': return 'bitcoin';
      case 'SOL': return 'solana';
      case 'JUP': return 'jupiter';
      case 'BONK': return 'bonk';
      case 'WIF': return 'dogwifhat';
    }
  };

  // Map Coin type for symbol
  const getSymbol = (coin: Coin): string => {
    switch(coin) {
      case 'ETH': return 'ETH-USDC';
      case 'BTC': return 'BTC-USDC';
      case 'SOL': return 'SOL-USDC';
      case 'JUP': return 'JUP-USDC';
      case 'BONK': return 'BONK-USDC';
      case 'WIF': return 'WIF-USDC';
    }
  };

  return (
    <div className="h-screen bg-zinc-950 text-zinc-100 flex flex-col overflow-hidden">
      {/* Testnet Warning Banner */}
      <TestnetBanner />

      {/* Header */}
      <header className="border-b border-white/5 bg-zinc-950/80 backdrop-blur-md shrink-0 z-50">
        <div className="px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link href="/" className="group">
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 hover:bg-zinc-800 transition-all">
                  <Home className="w-4 h-4 text-zinc-400 group-hover:text-white" />
                  <span className="text-sm font-medium text-zinc-400 group-hover:text-white hidden sm:inline">Home</span>
                </div>
              </Link>
              
              <div className="flex items-center gap-2">
                <div className="h-8 w-[1px] bg-white/5 mx-2 hidden sm:block" />
                <h1 className="text-lg font-bold tracking-tight">
                  <span className="bg-gradient-to-r from-blue-400 to-violet-500 bg-clip-text text-transparent">Percolator</span>
                </h1>
                <span className="px-2 py-0.5 text-[10px] font-bold tracking-wide bg-blue-500/10 text-blue-400 border border-blue-500/20 rounded-full uppercase">
                  Devnet
                </span>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <Link href="/cli" className="group">
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 hover:bg-emerald-500/10 hover:border-emerald-500/20 transition-all">
                  <Terminal className="w-4 h-4 text-zinc-400 group-hover:text-emerald-400" />
                  <span className="text-sm font-medium text-zinc-400 group-hover:text-emerald-400 hidden sm:inline">CLI</span>
                </div>
              </Link>
              
              <Link href="/portfolio" className="group">
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 hover:bg-violet-500/10 hover:border-violet-500/20 transition-all">
                  <Wallet className="w-4 h-4 text-zinc-400 group-hover:text-violet-400" />
                  <span className="text-sm font-medium text-zinc-400 group-hover:text-violet-400 hidden sm:inline">Portfolio</span>
                </div>
              </Link>
              
              <div className="h-8 w-[1px] bg-white/5 mx-1" />
              <WalletMultiButton className="!bg-zinc-900 !border !border-white/10 !rounded-full !h-9 !px-4 !text-sm !font-medium hover:!bg-zinc-800 !transition-all" />
            </div>
          </div>
        </div>
      </header>

      {/* Main Content - Responsive Layout */}
      <div 
        ref={containerRef} 
        className="flex-1 flex flex-col lg:flex-row overflow-hidden relative"
        style={{ 
          height: 'calc(100vh - 130px)',
          minHeight: 0
        }}
      >
        {/* Chart Area */}
        <div 
          className="w-full lg:flex-1 flex flex-col shrink-0 bg-zinc-950"
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
                marginBottom: '0'
              }
          }
        >
          <div className="w-full h-full p-1">
            <div className="w-full h-full rounded-xl border border-white/5 overflow-hidden bg-zinc-900/50">
              <TradingChart 
                coin={selectedCoin} 
                onPriceUpdate={setCurrentPrice}
                onCoinChange={setSelectedCoin}
              />
            </div>
          </div>
        </div>

        {/* Resizer Handle - Desktop Only */}
        <div
          onMouseDown={() => setIsResizing(true)}
          className="hidden lg:flex w-1 items-center justify-center cursor-col-resize group z-10 hover:w-1.5 transition-all duration-300 -ml-[2px]"
        >
          <div className="w-[1px] h-full bg-white/5 group-hover:bg-blue-500/50 transition-colors" />
        </div>

        {/* Trading Panel */}
        <div 
          className="w-full lg:w-auto bg-zinc-950 border-t lg:border-t-0 border-white/5 p-2 overflow-y-auto shrink-0"
          style={isMounted && typeof window !== 'undefined' && window.innerWidth >= 1024 
            ? { 
                width: `${rightPanelWidth}px`,
                height: '100%',
                maxHeight: '100%'
              } 
            : { 
                height: '50%',
                minHeight: '350px',
                maxHeight: '50%',
              }
          }
        >
          <div className="flex flex-col h-full gap-3">
            {/* Trading Panel Header */}
            <div className="flex items-center justify-between px-2 pt-1">
              <div>
                <h2 className="text-sm font-medium text-zinc-100">Trade {selectedCoin}/USDC</h2>
                <p className="text-xs text-zinc-500 font-mono mt-0.5">${currentPrice.toFixed(2)}</p>
              </div>
              
              {/* Trading Mode Toggle */}
              <div className="flex items-center bg-zinc-900 border border-white/5 p-1 rounded-lg">
                <button
                  onClick={() => setTradingMode('orderbook')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-all ${
                    tradingMode === 'orderbook'
                      ? 'bg-zinc-800 text-white shadow-sm'
                      : 'text-zinc-500 hover:text-zinc-300'
                  }`}
                >
                  Order Book
                </button>
                <button
                  onClick={() => setTradingMode('amm')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-all ${
                    tradingMode === 'amm'
                      ? 'bg-zinc-800 text-white shadow-sm'
                      : 'text-zinc-500 hover:text-zinc-300'
                  }`}
                >
                  AMM
                </button>
              </div>
            </div>

            {/* Trading Interface */}
            <div className="flex-1 flex flex-col min-h-0 gap-3">
              {tradingMode === 'orderbook' ? (
                <>
                  <OrderForm 
                    coin={getCoinString(selectedCoin)} 
                    currentPrice={currentPrice} 
                  />
                  <div className="flex-1 min-h-0">
                    <OrderBook symbol={getSymbol(selectedCoin)} walletAddress={publicKey?.toBase58()} />
                  </div>
                </>
              ) : (
                <div className="flex-1">
                  <AMMInterface 
                    selectedCoin={getCoinString(selectedCoin)} 
                    mode="swap"
                    showToast={() => {}}
                    chartCurrentPrice={currentPrice}
                  />
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
      
      {/* Status Footer */}
      <div className="hidden lg:block shrink-0 border-t border-white/5 bg-zinc-950">
        <StatusFooter />
      </div>
    </div>
  );
}
