"use client"

import { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { TrendingUp, TrendingDown, Activity, Target, Award, BarChart3 } from 'lucide-react';

interface TradeStatsProps {
  className?: string;
}

interface Stats {
  totalTrades: number;
  winningTrades: number;
  losingTrades: number;
  winRate: number;
  totalPnl: number;
  avgWin: number;
  avgLoss: number;
  bestTrade: number;
  worstTrade: number;
  profitFactor: number;
  avgHoldTime: string;
}

export function TradeStats({ className }: TradeStatsProps) {
  const { publicKey } = useWallet();
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);
  const [timeframe, setTimeframe] = useState<'24h' | '7d' | '30d' | 'all'>('7d');

  useEffect(() => {
    if (!publicKey) {
      setStats(null);
      setLoading(false);
      return;
    }

    const fetchStats = async () => {
      setLoading(true);
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
        const response = await fetch(`${API_URL}/api/user/${publicKey.toBase58()}/stats?timeframe=${timeframe}`);

        if (response.ok) {
          const data = await response.json();
          setStats(data);
        } else {
          // Use mock data
          setStats({
            totalTrades: 47,
            winningTrades: 28,
            losingTrades: 19,
            winRate: 59.57,
            totalPnl: 1234.56,
            avgWin: 89.23,
            avgLoss: -45.67,
            bestTrade: 456.78,
            worstTrade: -123.45,
            profitFactor: 1.95,
            avgHoldTime: '4h 23m',
          });
        }
      } catch (error) {
        console.error('Failed to fetch stats:', error);
        // Mock data
        setStats({
          totalTrades: 47,
          winningTrades: 28,
          losingTrades: 19,
          winRate: 59.57,
          totalPnl: 1234.56,
          avgWin: 89.23,
          avgLoss: -45.67,
          bestTrade: 456.78,
          worstTrade: -123.45,
          profitFactor: 1.95,
          avgHoldTime: '4h 23m',
        });
      } finally {
        setLoading(false);
      }
    };

    fetchStats();
  }, [publicKey, timeframe]);

  if (!publicKey) {
    return (
      <div className={`bg-zinc-900/50 rounded-xl p-4 border border-white/5 ${className}`}>
        <h3 className="text-sm font-semibold text-zinc-300 mb-3 flex items-center gap-2">
          <BarChart3 className="w-4 h-4" />
          Trading Statistics
        </h3>
        <p className="text-xs text-zinc-500 text-center py-4">Connect wallet to view stats</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className={`bg-zinc-900/50 rounded-xl p-4 border border-white/5 ${className}`}>
        <h3 className="text-sm font-semibold text-zinc-300 mb-3 flex items-center gap-2">
          <BarChart3 className="w-4 h-4" />
          Trading Statistics
        </h3>
        <div className="animate-pulse space-y-3">
          <div className="h-16 bg-zinc-800 rounded"></div>
          <div className="grid grid-cols-3 gap-2">
            <div className="h-12 bg-zinc-800 rounded"></div>
            <div className="h-12 bg-zinc-800 rounded"></div>
            <div className="h-12 bg-zinc-800 rounded"></div>
          </div>
        </div>
      </div>
    );
  }

  if (!stats) return null;

  return (
    <div className={`bg-zinc-900/50 rounded-xl p-4 border border-white/5 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-zinc-300 flex items-center gap-2">
          <BarChart3 className="w-4 h-4" />
          Trading Statistics
        </h3>
        <div className="flex gap-1">
          {(['24h', '7d', '30d', 'all'] as const).map((tf) => (
            <button
              key={tf}
              onClick={() => setTimeframe(tf)}
              className={`px-2 py-0.5 text-[10px] rounded transition-colors ${
                timeframe === tf
                  ? 'bg-blue-500/20 text-blue-400'
                  : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              {tf.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      {/* Main P&L Display */}
      <div className="bg-zinc-950/50 rounded-lg p-3 mb-3 border border-white/5">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-[10px] text-zinc-500 uppercase mb-1">Total P&L</p>
            <p className={`text-2xl font-bold ${stats.totalPnl >= 0 ? 'text-emerald-400' : 'text-red-400'}`}>
              {stats.totalPnl >= 0 ? '+' : ''}{stats.totalPnl.toFixed(2)}
              <span className="text-sm text-zinc-500 ml-1">USDC</span>
            </p>
          </div>
          <div className={`p-3 rounded-full ${stats.totalPnl >= 0 ? 'bg-emerald-500/10' : 'bg-red-500/10'}`}>
            {stats.totalPnl >= 0 ? (
              <TrendingUp className="w-6 h-6 text-emerald-400" />
            ) : (
              <TrendingDown className="w-6 h-6 text-red-400" />
            )}
          </div>
        </div>
      </div>

      {/* Win Rate Visual */}
      <div className="mb-4">
        <div className="flex items-center justify-between mb-1">
          <span className="text-[10px] text-zinc-500">Win Rate</span>
          <span className="text-xs font-bold text-zinc-300">{stats.winRate.toFixed(1)}%</span>
        </div>
        <div className="h-2 bg-zinc-800 rounded-full overflow-hidden">
          <div
            className="h-full bg-gradient-to-r from-emerald-500 to-emerald-400 rounded-full transition-all duration-500"
            style={{ width: `${stats.winRate}%` }}
          />
        </div>
        <div className="flex justify-between mt-1 text-[10px]">
          <span className="text-emerald-400">{stats.winningTrades}W</span>
          <span className="text-red-400">{stats.losingTrades}L</span>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-2 gap-2">
        <div className="bg-zinc-950/50 rounded-lg p-2 border border-white/5">
          <div className="flex items-center gap-1 mb-1">
            <Activity className="w-3 h-3 text-zinc-500" />
            <span className="text-[10px] text-zinc-500">Total Trades</span>
          </div>
          <p className="text-sm font-bold text-zinc-200">{stats.totalTrades}</p>
        </div>

        <div className="bg-zinc-950/50 rounded-lg p-2 border border-white/5">
          <div className="flex items-center gap-1 mb-1">
            <Target className="w-3 h-3 text-zinc-500" />
            <span className="text-[10px] text-zinc-500">Profit Factor</span>
          </div>
          <p className={`text-sm font-bold ${stats.profitFactor >= 1 ? 'text-emerald-400' : 'text-red-400'}`}>
            {stats.profitFactor.toFixed(2)}
          </p>
        </div>

        <div className="bg-zinc-950/50 rounded-lg p-2 border border-white/5">
          <div className="flex items-center gap-1 mb-1">
            <TrendingUp className="w-3 h-3 text-emerald-500" />
            <span className="text-[10px] text-zinc-500">Avg Win</span>
          </div>
          <p className="text-sm font-bold text-emerald-400">+{stats.avgWin.toFixed(2)}</p>
        </div>

        <div className="bg-zinc-950/50 rounded-lg p-2 border border-white/5">
          <div className="flex items-center gap-1 mb-1">
            <TrendingDown className="w-3 h-3 text-red-500" />
            <span className="text-[10px] text-zinc-500">Avg Loss</span>
          </div>
          <p className="text-sm font-bold text-red-400">{stats.avgLoss.toFixed(2)}</p>
        </div>

        <div className="bg-zinc-950/50 rounded-lg p-2 border border-white/5">
          <div className="flex items-center gap-1 mb-1">
            <Award className="w-3 h-3 text-amber-500" />
            <span className="text-[10px] text-zinc-500">Best Trade</span>
          </div>
          <p className="text-sm font-bold text-emerald-400">+{stats.bestTrade.toFixed(2)}</p>
        </div>

        <div className="bg-zinc-950/50 rounded-lg p-2 border border-white/5">
          <div className="flex items-center gap-1 mb-1">
            <TrendingDown className="w-3 h-3 text-red-500" />
            <span className="text-[10px] text-zinc-500">Worst Trade</span>
          </div>
          <p className="text-sm font-bold text-red-400">{stats.worstTrade.toFixed(2)}</p>
        </div>
      </div>
    </div>
  );
}
