"use client"

import { useState, useEffect } from 'react'
import { useWallet } from '@solana/wallet-adapter-react'
import { cn } from '@/lib/utils'
import { TrendingUp, TrendingDown, ExternalLink, RefreshCw } from 'lucide-react'

interface PastTrade {
  id: string
  symbol: string
  side: 'buy' | 'sell'
  price: number
  quantity: number
  timestamp: number
  status: string
  fee?: number
  signature?: string
  solscanLink?: string
}

interface PastTradesProps {
  symbol?: string
  className?: string
}

export function PastTrades({ symbol = 'SOL-USD', className }: PastTradesProps) {
  const { publicKey, connected } = useWallet()
  const [trades, setTrades] = useState<PastTrade[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)

  const fetchPastTrades = async () => {
    if (!connected || !publicKey) {
      setLoading(false)
      setTrades([])
      return
    }

    try {
      setRefreshing(true)
      const walletAddress = publicKey.toBase58()
      
      // Fetch from API
      const response = await fetch(
        `http://localhost:5001/api/slab/transactions?limit=50&wallet=${walletAddress}`
      )
      const data = await response.json()
      
      if (data.success && data.transactions) {
        // Convert transactions to trade format
        const formattedTrades: PastTrade[] = data.transactions
          .filter((tx: any) => tx.signer === walletAddress)
          .map((tx: any, index: number) => ({
            id: tx.signature,
            symbol,
            side: Math.random() > 0.5 ? 'buy' : 'sell', // TODO: Parse from tx data
            price: 186 + Math.random() * 10, // TODO: Parse from tx data
            quantity: Math.random() * 2, // TODO: Parse from tx data
            timestamp: (tx.blockTime || 0) * 1000,
            status: tx.err ? 'failed' : 'filled',
            fee: 0.02,
            signature: tx.signature,
            solscanLink: tx.solscanLink
          }))
        
        setTrades(formattedTrades)
      } else {
        setTrades([])
      }
    } catch (error) {
      console.error('Failed to fetch past trades:', error)
      setTrades([])
    } finally {
      setLoading(false)
      setRefreshing(false)
    }
  }

  useEffect(() => {
    fetchPastTrades()
    
    // Refresh every 30 seconds
    const interval = setInterval(fetchPastTrades, 30000)
    return () => clearInterval(interval)
  }, [symbol, connected, publicKey])

  return (
    <div className={cn(
      "w-full bg-gradient-to-br from-black/40 via-black/30 to-black/20 backdrop-blur-xl rounded-2xl border border-[#181825] overflow-hidden",
      className
    )}>
      {/* Header */}
      <div className="h-12 flex items-center justify-between px-4 border-b border-[#181825]">
        <h3 className="text-white font-semibold text-sm">Trade History</h3>
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-400">
            {trades.length} trades
          </span>
          <button
            onClick={fetchPastTrades}
            disabled={refreshing}
            className="p-1 hover:bg-[#181825] rounded transition-colors disabled:opacity-50"
          >
            <RefreshCw className={cn(
              "w-3.5 h-3.5 text-gray-400",
              refreshing && "animate-spin"
            )} />
          </button>
        </div>
      </div>
      
      {/* Trades List */}
      <div className="p-3">
        {!connected ? (
          <div className="text-center py-12">
            <div className="text-gray-500 text-sm mb-2">Connect wallet to view trade history</div>
            <div className="text-gray-600 text-xs">
              Your past trades will appear here
            </div>
          </div>
        ) : loading ? (
          <div className="text-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[#B8B8FF] mx-auto mb-2"></div>
            <div className="text-gray-500 text-sm">Loading trades...</div>
          </div>
        ) : trades.length > 0 ? (
          <>
            {/* Column Headers */}
            <div className="grid grid-cols-5 gap-2 text-xs text-gray-400 mb-2 font-semibold">
              <span>Side</span>
              <span>Price</span>
              <span>Size</span>
              <span>Time</span>
              <span className="text-right">Status</span>
            </div>
            
            {/* Trades */}
            <div className="space-y-1 max-h-[500px] overflow-y-auto">
              {trades.map((trade) => (
                <div 
                  key={trade.id} 
                  className="grid grid-cols-5 gap-2 text-sm py-2 hover:bg-[#181825]/50 rounded px-2 group cursor-pointer transition-colors"
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
                  <span className="text-gray-400 text-xs" suppressHydrationWarning>
                    {new Date(trade.timestamp).toLocaleDateString([], {
                      month: 'short',
                      day: 'numeric',
                      hour: '2-digit',
                      minute: '2-digit'
                    })}
                  </span>
                  <div className="text-right flex items-center justify-end gap-1">
                    <span className={cn(
                      "text-xs font-medium",
                      trade.status === 'filled' ? "text-green-400" : "text-red-400"
                    )}>
                      {trade.status}
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
          <div className="text-center py-12">
            <div className="text-gray-500 text-sm mb-2">No trades yet</div>
            <div className="text-gray-600 text-xs">
              Place your first order to start trading!
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

