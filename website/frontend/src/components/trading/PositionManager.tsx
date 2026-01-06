"use client"

import { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { TrendingUp, TrendingDown, DollarSign, Percent, AlertTriangle } from 'lucide-react';

interface Position {
  coin: string;
  symbol: string;
  side: 'long' | 'short';
  size: number;
  entryPrice: number;
  currentPrice: number;
  leverage: number;
  liquidationPrice: number;
  margin: number;
  unrealizedPnl: number;
  unrealizedPnlPercent: number;
}

interface PositionManagerProps {
  prices: Record<string, number>;
}

export function PositionManager({ prices }: PositionManagerProps) {
  const { publicKey } = useWallet();
  const [positions, setPositions] = useState<Position[]>([]);
  const [totalPnl, setTotalPnl] = useState(0);
  const [totalMargin, setTotalMargin] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!publicKey) {
      setPositions([]);
      setLoading(false);
      return;
    }

    // Fetch positions from API
    const fetchPositions = async () => {
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
        const response = await fetch(`${API_URL}/api/user/${publicKey.toBase58()}/positions`);

        if (response.ok) {
          const data = await response.json();
          if (data.positions) {
            // Calculate P&L with current prices
            const positionsWithPnl = data.positions.map((pos: any) => {
              const currentPrice = prices[pos.symbol] || pos.currentPrice || pos.entryPrice;
              const priceDiff = pos.side === 'long'
                ? currentPrice - pos.entryPrice
                : pos.entryPrice - currentPrice;
              const unrealizedPnl = priceDiff * pos.size;
              const unrealizedPnlPercent = (priceDiff / pos.entryPrice) * 100 * pos.leverage;

              return {
                ...pos,
                currentPrice,
                unrealizedPnl,
                unrealizedPnlPercent,
              };
            });
            setPositions(positionsWithPnl);
          }
        }
      } catch (error) {
        console.error('Failed to fetch positions:', error);
        // Use mock data for demo
        setPositions([
          {
            coin: 'solana',
            symbol: 'SOL',
            side: 'long',
            size: 10,
            entryPrice: 135.50,
            currentPrice: prices['SOL'] || 138.87,
            leverage: 5,
            liquidationPrice: 108.40,
            margin: 271.00,
            unrealizedPnl: (prices['SOL'] || 138.87 - 135.50) * 10,
            unrealizedPnlPercent: ((prices['SOL'] || 138.87 - 135.50) / 135.50) * 100 * 5,
          },
          {
            coin: 'ethereum',
            symbol: 'ETH',
            side: 'short',
            size: 0.5,
            entryPrice: 3280.00,
            currentPrice: prices['ETH'] || 3234.65,
            leverage: 3,
            liquidationPrice: 4264.00,
            margin: 546.67,
            unrealizedPnl: (3280.00 - (prices['ETH'] || 3234.65)) * 0.5,
            unrealizedPnlPercent: ((3280.00 - (prices['ETH'] || 3234.65)) / 3280.00) * 100 * 3,
          },
        ]);
      } finally {
        setLoading(false);
      }
    };

    fetchPositions();
    const interval = setInterval(fetchPositions, 10000);
    return () => clearInterval(interval);
  }, [publicKey, prices]);

  // Calculate totals
  useEffect(() => {
    const pnl = positions.reduce((sum, pos) => sum + pos.unrealizedPnl, 0);
    const margin = positions.reduce((sum, pos) => sum + pos.margin, 0);
    setTotalPnl(pnl);
    setTotalMargin(margin);
  }, [positions]);

  if (!publicKey) {
    return (
      <div className="bg-zinc-900/50 rounded-xl p-4 border border-white/5">
        <h3 className="text-sm font-semibold text-zinc-300 mb-3">Positions</h3>
        <p className="text-xs text-zinc-500 text-center py-4">Connect wallet to view positions</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="bg-zinc-900/50 rounded-xl p-4 border border-white/5">
        <h3 className="text-sm font-semibold text-zinc-300 mb-3">Positions</h3>
        <div className="animate-pulse space-y-2">
          <div className="h-12 bg-zinc-800 rounded"></div>
          <div className="h-12 bg-zinc-800 rounded"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-zinc-900/50 rounded-xl p-4 border border-white/5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-zinc-300">Open Positions</h3>
        <div className="flex items-center gap-3">
          <div className="text-right">
            <p className="text-[10px] text-zinc-500 uppercase">Total P&L</p>
            <p className={`text-sm font-bold ${totalPnl >= 0 ? 'text-emerald-400' : 'text-red-400'}`}>
              {totalPnl >= 0 ? '+' : ''}{totalPnl.toFixed(2)} USDC
            </p>
          </div>
          <div className="text-right">
            <p className="text-[10px] text-zinc-500 uppercase">Margin Used</p>
            <p className="text-sm font-mono text-zinc-300">{totalMargin.toFixed(2)} USDC</p>
          </div>
        </div>
      </div>

      {positions.length === 0 ? (
        <p className="text-xs text-zinc-500 text-center py-4">No open positions</p>
      ) : (
        <div className="space-y-2">
          {positions.map((position, index) => (
            <div
              key={index}
              className="bg-zinc-950/50 rounded-lg p-3 border border-white/5 hover:border-white/10 transition-colors"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-bold text-zinc-200">{position.symbol}</span>
                  <span className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
                    position.side === 'long'
                      ? 'bg-emerald-500/20 text-emerald-400'
                      : 'bg-red-500/20 text-red-400'
                  }`}>
                    {position.side.toUpperCase()} {position.leverage}x
                  </span>
                </div>
                <div className={`flex items-center gap-1 ${
                  position.unrealizedPnl >= 0 ? 'text-emerald-400' : 'text-red-400'
                }`}>
                  {position.unrealizedPnl >= 0 ? (
                    <TrendingUp className="w-3 h-3" />
                  ) : (
                    <TrendingDown className="w-3 h-3" />
                  )}
                  <span className="text-sm font-bold">
                    {position.unrealizedPnl >= 0 ? '+' : ''}{position.unrealizedPnl.toFixed(2)}
                  </span>
                  <span className="text-[10px]">
                    ({position.unrealizedPnlPercent >= 0 ? '+' : ''}{position.unrealizedPnlPercent.toFixed(2)}%)
                  </span>
                </div>
              </div>

              <div className="grid grid-cols-4 gap-2 text-[10px]">
                <div>
                  <p className="text-zinc-500">Size</p>
                  <p className="text-zinc-300 font-mono">{position.size} {position.symbol}</p>
                </div>
                <div>
                  <p className="text-zinc-500">Entry</p>
                  <p className="text-zinc-300 font-mono">${position.entryPrice.toFixed(2)}</p>
                </div>
                <div>
                  <p className="text-zinc-500">Mark</p>
                  <p className="text-zinc-300 font-mono">${position.currentPrice.toFixed(2)}</p>
                </div>
                <div>
                  <p className="text-zinc-500 flex items-center gap-0.5">
                    Liq. <AlertTriangle className="w-2 h-2 text-amber-500" />
                  </p>
                  <p className="text-amber-400 font-mono">${position.liquidationPrice.toFixed(2)}</p>
                </div>
              </div>

              {/* Liquidation warning bar */}
              {Math.abs(position.currentPrice - position.liquidationPrice) / position.currentPrice < 0.15 && (
                <div className="mt-2 bg-amber-500/10 border border-amber-500/20 rounded px-2 py-1">
                  <p className="text-[10px] text-amber-400 flex items-center gap-1">
                    <AlertTriangle className="w-3 h-3" />
                    Position approaching liquidation price
                  </p>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
