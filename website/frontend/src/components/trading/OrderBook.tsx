"use client"

import { useState, useEffect } from "react"
import { TrendingUp } from "lucide-react"
import { cn } from "@/lib/utils"
import { type Orderbook } from "@/lib/api-client"

interface OrderBookProps {
  symbol: string
  walletAddress?: string
}

export const OrderBook = ({ symbol, walletAddress }: OrderBookProps) => {
  const [orderbook, setOrderbook] = useState<Orderbook | null>(null)
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'orderbook' | 'trades'>('orderbook')
  const [recentTrades, setRecentTrades] = useState<Array<{ price: number; quantity: number; timestamp: number; side: string }>>([])
  const [transactions, setTransactions] = useState<Array<{ signature: string; blockTime?: number; err?: unknown; signer?: string; solscanLink?: string }>>([])
  const [walletTransactions, setWalletTransactions] = useState<Array<{ signature: string; blockTime?: number; err?: unknown; signer?: string; solscanLink?: string }>>([])
  const [wsConnected, setWsConnected] = useState(false)

  useEffect(() => {
    const fetchOrderbook = async () => {
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'
        const response = await fetch(`${API_URL}/api/slab-live/orderbook`)
        const data = await response.json()
        
        if (data.success) {
          setOrderbook({
            bids: data.orderbook.bids,
            asks: data.orderbook.asks,
            lastUpdate: Date.now()
          })
          
          if (data.recentTrades && data.recentTrades.length > 0) {
            setRecentTrades(data.recentTrades)
          }
          
          setLoading(false)
          setWsConnected(true)
        }
      } catch (error) {
        setLoading(false)
        setWsConnected(false)
      }
    }

    const fetchTransactions = async () => {
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'
        const response = await fetch(`${API_URL}/api/slab-live/transactions?limit=50`)
        const data = await response.json()
        
        if (data.success) {
          setTransactions(data.transactions)
          
          if (walletAddress) {
            const filteredTxs = data.transactions.filter((tx: { signer?: string }) => 
              tx.signer && tx.signer === walletAddress
            )
            setWalletTransactions(filteredTxs)
          } else {
            setWalletTransactions([])
          }
        }
      } catch (error) {
        // Silent error handling
      }
    }

    fetchOrderbook()
    fetchTransactions()

    const interval = setInterval(() => {
      fetchOrderbook()
      fetchTransactions()
    }, 5000)

    return () => {
      clearInterval(interval)
    }
  }, [symbol, walletAddress])

  const asks = orderbook?.asks.slice(0, 10) || []
  const bids = orderbook?.bids.slice(0, 10) || []
  const midPrice = asks.length && bids.length 
    ? ((asks[asks.length - 1].price + bids[0].price) / 2).toFixed(2)
    : '0.00'

  return (
    <div className="w-full h-full bg-zinc-900/50 rounded-xl border border-white/5 overflow-hidden flex flex-col">
      <div className="h-10 flex items-center justify-between px-3 border-b border-white/5 flex-shrink-0 bg-zinc-950/50">
        <div className="flex space-x-1 bg-zinc-900 rounded-lg p-0.5 border border-white/5">
          <button 
            onClick={() => setActiveTab('orderbook')}
            className={cn(
              "px-3 py-1 text-[10px] font-medium rounded-md transition-all",
              activeTab === 'orderbook' 
                ? "bg-zinc-800 text-white shadow-sm" 
                : "text-zinc-500 hover:text-zinc-300"
            )}
          >
            Order Book
          </button>
          
          <button 
            onClick={() => setActiveTab('trades')}
            className={cn(
              "px-3 py-1 text-[10px] font-medium rounded-md transition-all",
              activeTab === 'trades' 
                ? "bg-zinc-800 text-white shadow-sm" 
                : "text-zinc-500 hover:text-zinc-300"
            )}
          >
            Trades
          </button>
        </div>
        
        <div className="flex items-center space-x-2">
          <div className={cn(
            "w-1.5 h-1.5 rounded-full",
            wsConnected ? "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]" : "bg-amber-400"
          )}></div>
          <span className="text-[10px] text-zinc-500 font-medium uppercase tracking-wide">
            {wsConnected ? "Live" : "Connecting"}
          </span>
        </div>
      </div>
      
      <div className="flex-1 flex flex-col min-h-0">
        <div className="flex-1 p-0 overflow-y-auto border-b border-white/5 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent">
          {activeTab === 'orderbook' ? (
          <>
            <div className="grid grid-cols-3 px-3 py-2 text-[10px] font-medium text-zinc-500 border-b border-white/5 bg-zinc-950/30">
              <span className="truncate">Price (USDC)</span>
              <span className="truncate text-center">Size</span>
              <span className="truncate text-right">Total</span>
            </div>
            
            <div className="flex flex-col-reverse min-h-0">
              {/* Asks (Sells) - Red - Top part */}
              <div className="flex flex-col-reverse">
                {asks.length > 0 ? (
                  asks.slice(0, 15).map((ask, index) => {
                    const total = (ask.price || 0) * (ask.quantity || 0);
                    const maxTotal = Math.max(...asks.map(a => ((a.price || 0) * (a.quantity || 0))), 1);
                    return (
                      <div key={index} className="grid grid-cols-3 px-3 py-0.5 text-xs relative group hover:bg-white/5 transition-colors cursor-default">
                        <div 
                          className="absolute inset-y-0 right-0 bg-red-500/10 origin-right transition-all duration-300" 
                          style={{ width: `${(total / maxTotal) * 100}%` }}
                        />
                        <span className="text-red-400 font-mono relative z-10 truncate group-hover:text-red-300">{(ask.price || 0).toFixed(2)}</span>
                        <span className="text-zinc-300 font-mono relative z-10 truncate text-center">{(ask.quantity || 0).toFixed(4)}</span>
                        <span className="text-zinc-500 font-mono relative z-10 text-right truncate group-hover:text-zinc-400">{total.toFixed(2)}</span>
                      </div>
                    );
                  })
                ) : (
                  <div className="text-zinc-600 text-xs text-center py-8 italic">No asks available</div>
                )}
              </div>
            </div>
            
            {/* Mid Price */}
            <div className="flex items-center justify-between px-4 py-2 border-y border-white/5 bg-zinc-950/50 backdrop-blur-sm sticky top-0 bottom-0 z-20 my-1">
              <div className="flex items-center gap-2">
                <span className={`text-lg font-mono font-bold ${
                  parseFloat(midPrice) > 0 ? 'text-white' : 'text-zinc-500'
                }`}>
                  {midPrice}
                </span>
                <span className="text-[10px] text-zinc-500 font-medium">USDC</span>
              </div>
              {parseFloat(midPrice) > 0 && (
                <div className="flex items-center gap-1 text-emerald-500 bg-emerald-500/10 px-1.5 py-0.5 rounded text-[10px] font-medium">
                  <TrendingUp className="w-3 h-3" />
                  <span>Spread: {asks.length && bids.length ? (asks[asks.length-1].price - bids[0].price).toFixed(2) : '0.00'}</span>
                </div>
              )}
            </div>
            
            {/* Bids (Buys) - Green - Bottom part */}
            <div className="flex flex-col">
              {bids.length > 0 ? (
                bids.slice(0, 15).map((bid, index) => {
                  const total = (bid.price || 0) * (bid.quantity || 0);
                  const maxTotal = Math.max(...bids.map(b => ((b.price || 0) * (b.quantity || 0))), 1);
                  return (
                    <div key={index} className="grid grid-cols-3 px-3 py-0.5 text-xs relative group hover:bg-white/5 transition-colors cursor-default">
                      <div 
                        className="absolute inset-y-0 right-0 bg-emerald-500/10 origin-right transition-all duration-300" 
                        style={{ width: `${(total / maxTotal) * 100}%` }}
                      />
                      <span className="text-emerald-400 font-mono relative z-10 truncate group-hover:text-emerald-300">{(bid.price || 0).toFixed(2)}</span>
                      <span className="text-zinc-300 font-mono relative z-10 truncate text-center">{(bid.quantity || 0).toFixed(4)}</span>
                      <span className="text-zinc-500 font-mono relative z-10 text-right truncate group-hover:text-zinc-400">{total.toFixed(2)}</span>
                    </div>
                  );
                })
              ) : (
                <div className="text-zinc-600 text-xs text-center py-8 italic">No bids available</div>
              )}
            </div>
          </>
        ) : (
          <>
            <div className="p-3 space-y-2">
              <div className="flex items-center justify-between pb-2 border-b border-white/5">
                <h4 className="text-xs font-medium text-zinc-400">Your Activity</h4>
                {walletAddress && <span className="text-[10px] text-emerald-500 bg-emerald-500/10 px-2 py-0.5 rounded-full">Connected</span>}
              </div>
              
              {walletTransactions.length > 0 ? (
                <div className="space-y-2">
                  {walletTransactions.slice(0, 10).map((tx, i) => {
                    const isReserve = i % 2 === 0; // This is a rough heuristic, ideally we'd know the type
                    const typeColor = isReserve ? 'blue' : 'emerald';
                    
                    return (
                      <a
                        key={i}
                        href={tx.solscanLink}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="block p-2 bg-zinc-900/50 hover:bg-zinc-800 rounded-lg border border-white/5 transition-all group relative overflow-hidden"
                      >
                        <div className={`absolute left-0 top-0 bottom-0 w-0.5 bg-${typeColor}-500/50`} />
                        <div className="flex items-center justify-between mb-1">
                          <div className="flex items-center gap-2">
                            {tx.err ? (
                              <span className="text-red-400 text-[10px] font-bold">FAILED</span>
                            ) : (
                              <span className="text-emerald-400 text-[10px] font-bold">CONFIRMED</span>
                            )}
                            <span className="text-zinc-500 text-[9px] font-mono">
                              {tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleTimeString() : 'Pending'}
                            </span>
                          </div>
                        </div>
                        <code className="text-[10px] font-mono text-zinc-400 group-hover:text-zinc-300 block truncate">
                          {tx.signature}
                        </code>
                      </a>
                    );
                  })}
                </div>
              ) : (
                <div className="text-center py-12 border border-dashed border-white/10 rounded-lg bg-white/[0.02]">
                  {walletAddress ? (
                    <>
                      <div className="text-zinc-500 text-sm font-medium mb-1">No trades found</div>
                      <div className="text-zinc-700 text-xs">Your recent trading activity will appear here</div>
                    </>
                  ) : (
                    <>
                      <div className="text-zinc-500 text-sm font-medium mb-1">Wallet not connected</div>
                      <div className="text-zinc-700 text-xs">Connect your wallet to see your trades</div>
                    </>
                  )}
                </div>
              )}
            </div>

            <div className="p-3 border-t border-white/5">
              <div className="flex items-center justify-between mb-3">
                <h4 className="text-xs font-medium text-zinc-400">Recent Network Activity</h4>
                <div className="text-[10px] text-zinc-500 bg-zinc-900 px-2 py-0.5 rounded border border-white/5">
                  Live Feed
                </div>
              </div>
              <div className="space-y-2">
                {transactions.length === 0 ? (
                  <div className="text-center py-4 text-zinc-600 text-xs italic">
                    Waiting for transactions...
                  </div>
                ) : (
                  transactions.slice(0, 5).map((tx, i) => (
                    <a
                      key={i}
                      href={tx.solscanLink}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center justify-between p-2 hover:bg-white/5 rounded-md transition-colors group"
                    >
                      <div className="flex items-center gap-2">
                        <div className={`w-1.5 h-1.5 rounded-full ${tx.err ? 'bg-red-500' : 'bg-emerald-500'}`} />
                        <span className="text-[10px] font-mono text-zinc-500 group-hover:text-zinc-300 transition-colors">
                          {tx.signature.substring(0, 8)}...
                        </span>
                      </div>
                      <span className="text-[10px] text-zinc-600">
                        {tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleTimeString() : ''}
                      </span>
                    </a>
                  ))
                )}
              </div>
            </div>
          </>
        )}
        </div>
      </div>

      <div className="px-3 py-2 bg-zinc-950/30 border-t border-white/5 flex-shrink-0">
        <div className="flex items-center justify-between text-[10px]">
          <span className="text-zinc-500 font-medium">Slab Program</span>
          <button
            onClick={() => {
              navigator.clipboard.writeText('5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB');
            }}
            className="text-blue-400/80 hover:text-blue-400 font-mono transition-colors cursor-pointer flex items-center gap-1 group"
            title="Click to copy"
          >
            5Yd2...CLDB
            <span className="opacity-0 group-hover:opacity-100 transition-opacity">📋</span>
          </button>
        </div>
      </div>
    </div>
  )
}


