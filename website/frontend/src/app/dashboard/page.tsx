"use client"

import { useState } from 'react';
import TradingChart from '@/components/TradingChart';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

type Coin = 'SOL' | 'ETH' | 'BTC';

export default function DashboardPage() {
  const [selectedCoin, setSelectedCoin] = useState<Coin>('SOL');
  const [currentPrice, setCurrentPrice] = useState<number>(0);

  const coins: { value: Coin; label: string }[] = [
    { value: 'SOL', label: 'Solana' },
    { value: 'ETH', label: 'Ethereum' },
    { value: 'BTC', label: 'Bitcoin' },
  ];

  return (
    <div className="min-h-screen bg-gray-950 text-white">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-900">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <h1 className="text-2xl font-bold bg-gradient-to-r from-blue-500 to-purple-600 bg-clip-text text-transparent">
                Percolator
              </h1>
              <span className="px-3 py-1 text-xs font-semibold bg-blue-600 rounded">
                DEVNET
              </span>
            </div>
            <div className="flex items-center gap-4">
              <WalletMultiButton />
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="container mx-auto px-6 py-6">
        {/* Coin Selector */}
        <div className="mb-4 flex gap-2">
          {coins.map((coin) => (
            <button
              key={coin.value}
              onClick={() => setSelectedCoin(coin.value)}
              className={`px-6 py-3 rounded-lg font-semibold transition-colors ${
                selectedCoin === coin.value
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-white'
              }`}
            >
              {coin.label}
            </button>
          ))}
        </div>

        {/* Chart Area */}
        <div className="h-[calc(100vh-200px)] mb-6">
          <TradingChart 
            coin={selectedCoin} 
            onPriceUpdate={setCurrentPrice}
          />
        </div>

        {/* Trading Panel - Coming Soon */}
        <div className="bg-gray-900 rounded-lg border border-gray-800 p-6">
          <div className="text-center text-gray-400">
            <p className="text-lg font-semibold mb-2">Trading Panel</p>
            <p className="text-sm">Coming soon - Will connect to Solana programs</p>
          </div>
        </div>
      </div>
    </div>
  );
}

