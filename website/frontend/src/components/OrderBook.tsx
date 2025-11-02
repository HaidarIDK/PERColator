"use client"

import { useState, useEffect } from 'react'
import { cn } from '@/lib/utils'
import { TrendingUp, TrendingDown, Copy, Check } from 'lucide-react'

interface OrderBookLevel {
  price: number
  quantity: number
  total?: number
  user?: string
}

interface OrderBookData {
  bids: OrderBookLevel[]
  asks: OrderBookLevel[]
  midPrice?: number
  spread?: number
  lastUpdate: number
}

interface OrderBookProps {
  symbol?: string
  walletAddress?: string
  className?: string
}

export function OrderBook({ symbol = 'SOL-USD', walletAddress, className }: OrderBookProps) {
  const [orderbook, setOrderbook] = useState<OrderBookData | null>(null)
  const [loading, setLoading] = useState(true)
  const [connected, setConnected] = useState(false)
  const [activeTab, setActiveTab] = useState<'orderbook' | 'trades'>('orderbook')
  const [slabInfo, setSlabInfo] = useState<{ coin: string; slabAccount: string; message: string } | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    const fetchOrderbook = async () => {
      try {
        // Extract coin from symbol (e.g., 'SOL-USD' -> 'SOL')
        const coin = symbol.split('-')[0];
        
        // Fetch from our API that reads the slab for this coin
        const response = await fetch(`http://localhost:5001/api/slab/orderbook?coin=${coin}`)
        const data = await response.json()
        
        if (data.success && data.orderbook) {
          setOrderbook({
            bids: data.orderbook.bids || [],
            asks: data.orderbook.asks || [],
            midPrice: data.orderbook.midPrice,
            spread: data.orderbook.spread,
            lastUpdate: Date.now()
          })
          setSlabInfo({
            coin: data.coin || coin,
            slabAccount: data.slabAccount || 'unknown',
            message: data.message || ''
          })
          setConnected(true)
        } else {
          // If no data, show empty orderbook
          setOrderbook({
            bids: [],
            asks: [],
            lastUpdate: Date.now()
          })
          setConnected(false)
        }
        setLoading(false)
      } catch (error) {
        console.error('Failed to fetch orderbook:', error)
        setLoading(false)
        setConnected(false)
        // Set empty orderbook on error
        setOrderbook({
          bids: [],
          asks: [],
          lastUpdate: Date.now()
        })
      }
    }

    fetchOrderbook()

    // Refresh every 5 seconds
    const interval = setInterval(fetchOrderbook, 5000)

    return () => clearInterval(interval)
  }, [symbol])

  const asks = orderbook?.asks.slice(0, 10) || []
  const bids = orderbook?.bids.slice(0, 10) || []
  const midPrice = orderbook?.midPrice || 
    (asks.length && bids.length 
      ? ((asks[asks.length - 1].price + bids[0].price) / 2)
      : 0)
  const spread = orderbook?.spread || 0

  const handleCopyAddress = async () => {
    if (slabInfo?.slabAccount) {
      try {
        await navigator.clipboard.writeText(slabInfo.slabAccount)
        setCopied(true)
        setTimeout(() => setCopied(false), 2000)
      } catch (err) {
        console.error('Failed to copy:', err)
      }
    }
  }

  return (
    <div className={cn(
      "w-full h-full bg-gradient-to-br from-black/40 via-black/30 to-black/20 backdrop-blur-xl rounded-2xl border border-[#181825] overflow-hidden flex flex-col",
      className
    )}>
      {/* Header */}
      <div className="h-12 flex items-center justify-between px-4 border-b border-[#181825] flex-shrink-0">
        <div className="flex items-center space-x-3">
          <span className="text-xs font-bold text-[#B8B8FF] bg-[#B8B8FF]/10 px-2 py-1 rounded">
            {symbol}
          </span>
          <button 
            onClick={() => setActiveTab('orderbook')}
            className={cn(
              "text-sm font-semibold transition-colors",
              activeTab === 'orderbook' ? "text-[#B8B8FF]" : "text-gray-400 hover:text-white"
            )}
          >
            Order Book
          </button>
          
          <button 
            onClick={() => setActiveTab('trades')}
            className={cn(
              "text-sm font-semibold transition-colors",
              activeTab === 'trades' ? "text-[#B8B8FF]" : "text-gray-400 hover:text-white"
            )}
          >
            Recent Trades
          </button>
        </div>
        
        {/* Connection Status */}
        <div className="flex items-center space-x-2">
          <div className={cn(
            "w-2 h-2 rounded-full",
            connected ? "bg-green-400 animate-pulse" : "bg-yellow-400"
          )}></div>
          <span className="text-xs text-gray-400 font-medium">
            {connected ? "Live" : "Connecting..."}
          </span>
        </div>
      </div>
      
      {/* Slab Info Banner */}
      {slabInfo && (
        <div className="px-4 py-2 bg-gradient-to-r from-[#B8B8FF]/5 to-transparent border-b border-[#181825]">
          <div className="flex items-center justify-between text-[10px]">
            <div className="flex items-center gap-2">
              <span className="text-gray-400">Slab:</span>
              <button
                onClick={handleCopyAddress}
                className="flex items-center gap-1.5 hover:bg-[#B8B8FF]/10 px-2 py-1 rounded transition-colors group"
                title="Click to copy full address"
              >
                <span className="font-mono text-[#B8B8FF] truncate max-w-[100px]">
                  {slabInfo.slabAccount.slice(0, 8)}...
                </span>
                {copied ? (
                  <Check className="w-3 h-3 text-green-400" />
                ) : (
                  <Copy className="w-3 h-3 text-gray-400 group-hover:text-[#B8B8FF] transition-colors" />
                )}
              </button>
            </div>
            <span className={cn(
              "text-[9px] px-2 py-0.5 rounded",
              slabInfo.message.includes('Real on-chain') || slabInfo.message.includes('✅') ? "bg-green-500/20 text-green-400" : "bg-blue-500/20 text-blue-400"
            )}>
              {slabInfo.message.includes('Real on-chain') || slabInfo.message.includes('✅') ? '🟢 On-Chain' : '🔵 Simulated'}
            </span>
          </div>
        </div>
      )}
      
      {/* Content */}
      <div className="flex-1 p-4 overflow-y-auto">
        {activeTab === 'orderbook' ? (
          <>
            {/* Column Headers */}
            <div className="grid grid-cols-3 gap-1 sm:gap-2 text-[10px] sm:text-xs text-gray-400 mb-2 font-semibold">
              <span>Price</span>
              <span>Size</span>
              <span className="text-right">Total</span>
            </div>
            
            {/* Asks (Sell Orders) */}
            <div className="space-y-0.5 mb-3">
              {asks.length > 0 ? (
                [...asks].reverse().map((ask, index) => {
                  const total = ask.price * ask.quantity
                  const maxTotal = Math.max(...asks.map(a => a.price * a.quantity), 1)
                  return (
                    <div key={`ask-${index}`} className="grid grid-cols-3 gap-1 sm:gap-2 text-[10px] sm:text-xs relative group hover:bg-red-500/5 cursor-pointer rounded px-1">
                      <div 
                        className="absolute inset-0 bg-red-500/10 origin-right rounded" 
                        style={{ width: `${(total / maxTotal) * 100}%` }}
                      />
                      <span className="text-red-400 relative z-10 font-mono">${ask.price.toFixed(2)}</span>
                      <span className="text-white relative z-10 font-mono">{ask.quantity.toFixed(4)}</span>
                      <span className="text-gray-400 relative z-10 text-right font-mono">${total.toFixed(2)}</span>
                    </div>
                  )
                })
              ) : (
                <div className="text-center text-gray-500 text-sm py-2">No asks</div>
              )}
            </div>
            
            {/* Mid Price */}
            <div className="flex items-center justify-center gap-2 my-3 py-2 bg-gradient-to-r from-transparent via-[#B8B8FF]/10 to-transparent border-y border-[#B8B8FF]/20">
              <TrendingUp className="w-4 h-4 text-[#B8B8FF]" />
              <span className="text-[#B8B8FF] font-bold text-lg font-mono">
                ${midPrice.toFixed(2)}
              </span>
              {spread > 0 && (
                <span className="text-xs text-gray-400">
                  (Spread: {spread.toFixed(2)}%)
                </span>
              )}
            </div>
            
            {/* Bids (Buy Orders) */}
            <div className="space-y-0.5">
              {bids.length > 0 ? (
                bids.map((bid, index) => {
                  const total = bid.price * bid.quantity
                  const maxTotal = Math.max(...bids.map(b => b.price * b.quantity), 1)
                  return (
                    <div key={`bid-${index}`} className="grid grid-cols-3 gap-1 sm:gap-2 text-[10px] sm:text-xs relative group hover:bg-green-500/5 cursor-pointer rounded px-1">
                      <div 
                        className="absolute inset-0 bg-green-500/10 origin-right rounded" 
                        style={{ width: `${(total / maxTotal) * 100}%` }}
                      />
                      <span className="text-green-400 relative z-10 font-mono">${bid.price.toFixed(2)}</span>
                      <span className="text-white relative z-10 font-mono">{bid.quantity.toFixed(4)}</span>
                      <span className="text-gray-400 relative z-10 text-right font-mono">${total.toFixed(2)}</span>
                    </div>
                  )
                })
              ) : (
                <div className="text-center text-gray-500 text-sm py-2">No bids</div>
              )}
            </div>

            {loading && (
              <div className="text-center text-gray-500 text-sm mt-4">
                Loading orderbook...
              </div>
            )}
          </>
        ) : (
          // Recent Trades Tab
          <div className="text-center text-gray-500 text-sm py-8">
            Recent trades will appear here
          </div>
        )}
      </div>
    </div>
  )
}

