"use client"

import { useState, useEffect } from 'react'
import { cn } from '@/lib/utils'
import { TrendingUp, TrendingDown, ExternalLink } from 'lucide-react'

interface Trade {
  price: number
  quantity: number
  timestamp: number
  side: string
  signature?: string
  solscanLink?: string
}

interface RecentTradesProps {
  symbol?: string
  walletAddress?: string
  className?: string
  maxTrades?: number
}

export function RecentTrades({ 
  symbol = 'SOL-USD', 
  walletAddress, 
  className,
  maxTrades = 20
}: RecentTradesProps) {
  const [trades, setTrades] = useState<Trade[]>([])
  const [loading, setLoading] = useState(true)
  const [connected, setConnected] = useState(false)

  useEffect(() => {
    const fetchTrades = async () => {
      try {
        const response = await fetch('http://localhost:5001/api/slab/orderbook')
        const data = await response.json()
        
        if (data.success && data.recentTrades) {
          setTrades(data.recentTrades.slice(0, maxTrades))
          setConnected(true)
        } else {
          setTrades([])
          setConnected(false)
        }
        setLoading(false)
      } catch (error) {
        console.error('Failed to fetch recent trades:', error)
        setLoading(false)
        setConnected(false)
        setTrades([])
      }
    }

    fetchTrades()

    // Refresh every 5 seconds
    const interval = setInterval(fetchTrades, 5000)

    return () => clearInterval(interval)
  }, [symbol, maxTrades])

  // Filter by wallet if provided
  const displayTrades = walletAddress 
    ? trades.filter(t => t.signature?.includes(walletAddress)) 
    : trades

  return (
    <div className={cn(
      "w-full bg-gradient-to-br from-black/40 via-black/30 to-black/20 backdrop-blur-xl rounded-2xl border border-[#181825] overflow-hidden",
      className
    )}>
      {/* Header */}
      <div className="h-12 flex items-center justify-between px-4 border-b border-[#181825]">
        <h3 className="text-white font-semibold text-sm">
          {walletAddress ? 'Your Trades' : 'Recent Trades'}
        </h3>
        <div className="flex items-center space-x-2">
          <div className={cn(
            "w-2 h-2 rounded-full",
            connected ? "bg-green-400 animate-pulse" : "bg-gray-400"
          )}></div>
          <span className="text-xs text-gray-400">
            {displayTrades.length} trades
          </span>
        </div>
      </div>
      
      {/* Trades List */}
      <div className="p-3">
        {loading ? (
          <div className="text-center py-8 text-gray-500 text-sm">
            Loading trades...
          </div>
        ) : displayTrades.length > 0 ? (
          <>
            {/* Column Headers */}
            <div className="grid grid-cols-4 gap-2 text-xs text-gray-400 mb-2 font-semibold">
              <span>Side</span>
              <span>Price</span>
              <span>Size</span>
              <span className="text-right">Time</span>
            </div>
            
            {/* Trades */}
            <div className="space-y-1 max-h-[400px] overflow-y-auto">
              {displayTrades.map((trade, index) => (
                <div 
                  key={index} 
                  className="grid grid-cols-4 gap-2 text-sm py-1.5 hover:bg-[#181825]/50 rounded px-1.5 group cursor-pointer transition-colors"
                  onClick={() => {
                    if (trade.solscanLink) {
                      window.open(trade.solscanLink, '_blank')
                    }
                  }}
                >
                  <div className="flex items-center gap-1.5">
                    {trade.side === 'buy' ? (
                      <>
                        <TrendingUp className="w-3 h-3 text-green-400" />
                        <span className="text-green-400 font-semibold text-xs">BUY</span>
                      </>
                    ) : (
                      <>
                        <TrendingDown className="w-3 h-3 text-red-400" />
                        <span className="text-red-400 font-semibold text-xs">SELL</span>
                      </>
                    )}
                  </div>
                  <span className="text-white font-mono">${trade.price.toFixed(2)}</span>
                  <span className="text-gray-300 font-mono">{trade.quantity.toFixed(4)}</span>
                  <div className="text-right flex items-center justify-end gap-1">
                    <span className="text-gray-400 text-xs" suppressHydrationWarning>
                      {new Date(trade.timestamp).toLocaleTimeString([], { 
                        hour: '2-digit', 
                        minute: '2-digit',
                        second: '2-digit'
                      })}
                    </span>
                    {trade.solscanLink && (
                      <ExternalLink className="w-3 h-3 text-gray-500 opacity-0 group-hover:opacity-100 transition-opacity" />
                    )}
                  </div>
                </div>
              ))}
            </div>
          </>
        ) : (
          <div className="text-center py-8">
            <div className="text-gray-500 text-sm">
              {walletAddress 
                ? 'No trades from your wallet yet' 
                : 'No recent trades'}
            </div>
            <div className="text-gray-600 text-xs mt-1">
              Place an order to start trading!
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

