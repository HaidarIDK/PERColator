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
    <div className="w-full h-full bg-black/20 rounded-2xl border border-[#181825] overflow-hidden flex flex-col">
      <div className="h-12 flex items-center justify-between px-4 border-b border-[#181825] flex-shrink-0">
        <div className="flex space-x-4">
          <button 
            onClick={() => setActiveTab('orderbook')}
            className={cn(
              "font-medium transition-colors",
              activeTab === 'orderbook' ? "text-[#B8B8FF]" : "text-gray-400 hover:text-white"
            )}
          >
            Order book
          </button>
          
          <button 
            onClick={() => setActiveTab('trades')}
            className={cn(
              "font-medium transition-colors",
              activeTab === 'trades' ? "text-[#B8B8FF]" : "text-gray-400 hover:text-white"
            )}
          >
            Last trades
          </button>
          {loading && <span className="text-xs text-gray-500 ml-2">Loading...</span>}
        </div>
        
        <div className="flex items-center space-x-2">
          <div className={cn(
            "w-2 h-2 rounded-full",
            wsConnected ? "bg-green-400 animate-pulse" : "bg-yellow-400"
          )}></div>
          <span className="text-xs text-green-400 font-semibold">
            {wsConnected ? "Live from Slab" : "Connecting..."}
          </span>
        </div>
      </div>
      
      <div className="flex-1 flex flex-col min-h-0">
        <div className="flex-1 p-4 overflow-y-auto border-b border-[#181825] [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
          {activeTab === 'orderbook' ? (
          <>
            <div className="grid grid-cols-3 gap-1 sm:gap-2 text-[10px] sm:text-xs text-gray-400 mb-2">
              <span className="truncate">Price (USDC)</span>
              <span className="truncate">Qty</span>
              <span className="truncate text-right">Total</span>
            </div>
            
            <div className="space-y-1 mb-4">
              {asks.length > 0 ? (
                asks.reverse().map((ask, index) => {
                  const total = (ask.price || 0) * (ask.quantity || 0);
                  const maxTotal = Math.max(...asks.map(a => ((a.price || 0) * (a.quantity || 0))), 1);
                  return (
                    <div key={index} className="grid grid-cols-3 gap-1 sm:gap-2 text-xs sm:text-sm relative">
                      <div 
                        className="absolute inset-0 bg-red-500/5 origin-left" 
                        style={{ width: `${(total / maxTotal) * 100}%` }}
                      />
                      <span className="text-red-400 relative z-10 truncate">{(ask.price || 0).toFixed(2)}</span>
                      <span className="text-white relative z-10 truncate">{(ask.quantity || 0).toFixed(4)}</span>
                      <span className="text-gray-400 relative z-10 text-right truncate">{total.toFixed(4)}</span>
                    </div>
                  );
                })
              ) : (
                <div className="text-gray-500 text-sm text-center py-4">No asks</div>
              )}
            </div>
            
            <div className="flex items-center justify-center py-2 border-y border-[#181825] my-2">
              <span className="text-white font-semibold">{midPrice}</span>
              <TrendingUp className="w-3 h-3 text-green-400 ml-2" />
            </div>
            
            <div className="space-y-1">
              {bids.length > 0 ? (
                bids.map((bid, index) => {
                  const total = (bid.price || 0) * (bid.quantity || 0);
                  const maxTotal = Math.max(...bids.map(b => ((b.price || 0) * (b.quantity || 0))), 1);
                  return (
                    <div key={index} className="grid grid-cols-3 gap-1 sm:gap-2 text-xs sm:text-sm relative">
                      <div 
                        className="absolute inset-0 bg-green-500/5 origin-left" 
                        style={{ width: `${(total / maxTotal) * 100}%` }}
                      />
                      <span className="text-green-400 relative z-10 truncate">{(bid.price || 0).toFixed(2)}</span>
                      <span className="text-white relative z-10 truncate">{(bid.quantity || 0).toFixed(4)}</span>
                      <span className="text-gray-400 relative z-10 text-right truncate">{total.toFixed(4)}</span>
                    </div>
                  );
                })
              ) : (
                <div className="text-gray-500 text-sm text-center py-4">No bids</div>
              )}
            </div>
          </>
        ) : (
          <>
            <div className="space-y-2 max-h-[600px] overflow-y-auto [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
              {walletTransactions.length > 0 ? (
                walletTransactions.slice(0, 10).map((tx, i) => {
                  const isReserve = i % 2 === 0;
                  const typeColor = isReserve ? 'blue' : 'green';
                  const typeBg = isReserve ? 'bg-blue-900/10' : 'bg-green-900/10';
                  const typeBorder = isReserve ? 'border-blue-700/20' : 'border-green-700/20';
                  const typeLabel = isReserve ? 'RESERVE' : 'COMMIT';
                  
                  return (
                    <a
                      key={i}
                      href={tx.solscanLink}
                      target="_blank"
                      rel="noopener noreferrer"
                      className={`block p-2 ${typeBg} hover:bg-zinc-800/50 rounded border ${typeBorder} transition-all group`}
                    >
                      <div className="flex items-center justify-between mb-1">
                        <div className="flex items-center gap-2">
                          {tx.err ? (
                            <span className="text-red-400 text-[10px] font-semibold">UNSUCCESSFUL</span>
                          ) : (
                            <span className="text-green-400 text-[10px] font-semibold">SUCCESS</span>
                          )}
                          <span className={`text-${typeColor}-400 text-[9px] font-mono bg-${typeColor}-900/30 px-1.5 py-0.5 rounded border border-${typeColor}-700/50`}>
                            {typeLabel}
                          </span>
                        </div>
                        <span className="text-zinc-500 text-[9px]">
                          {tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleTimeString() : 'Pending'}
                        </span>
                      </div>
                      <code className={`text-[9px] font-mono text-${typeColor}-300 group-hover:text-${typeColor}-200 block truncate`}>
                        {tx.signature.substring(0, 24)}...
                      </code>
                    </a>
                  );
                })
              ) : (
                <div className="text-gray-500 text-sm text-center py-8">
                  {walletAddress ? (
                    <>
                      <div className="text-zinc-500 mb-1">No trades from your wallet yet</div>
                      <div className="text-zinc-700 text-xs">Make a trade to see it here!</div>
                    </>
                  ) : (
                    <>
                      <div className="text-zinc-500 mb-1">Connect wallet to see your trades</div>
                      <div className="text-zinc-700 text-xs">Your Reserve/Commit transactions will appear here</div>
                    </>
                  )}
                </div>
              )}
            </div>
          </>
        )}
        </div>

        <div className="flex-1 p-4 overflow-y-auto [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-xs font-semibold text-zinc-400">
              Slab Transactions
            </h4>
            <div className="text-[9px] text-zinc-500 bg-zinc-900/20 px-2 py-0.5 rounded border border-zinc-700/30">
              All Trades
            </div>
          </div>
          <div className="space-y-2">
            {transactions.length === 0 ? (
              <div className="text-center py-4 text-zinc-600 text-xs">
                No transactions yet
              </div>
            ) : (
              transactions.map((tx, i) => {
                const isReserve = i % 2 === 0;
                const typeColor = isReserve ? 'blue' : 'green';
                const typeBg = isReserve ? 'bg-blue-900/20' : 'bg-green-900/20';
                const typeBorder = isReserve ? 'border-blue-700/30' : 'border-green-700/30';
                const typeLabel = isReserve ? 'RESERVE' : 'COMMIT';
                
                return (
                  <a
                    key={i}
                    href={tx.solscanLink}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`block p-2 ${typeBg} hover:bg-zinc-800/50 rounded border ${typeBorder} hover:border-${typeColor}-600/50 transition-all group`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <div className="flex items-center gap-2">
                        {tx.err ? (
                          <span className="text-red-400 text-[10px] font-semibold">UNSUCCESSFUL</span>
                        ) : (
                          <span className="text-green-400 text-[10px] font-semibold">SUCCESS</span>
                        )}
                        <span className={`text-${typeColor}-400 text-[9px] font-mono bg-${typeColor}-900/30 px-1.5 py-0.5 rounded border border-${typeColor}-700/50`}>
                          {typeLabel}
                        </span>
                      </div>
                      <span className="text-zinc-500 text-[9px]">
                        {tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleTimeString() : 'Pending'}
                      </span>
                    </div>
                    <code className={`text-[9px] font-mono text-${typeColor}-300 group-hover:text-${typeColor}-200 block truncate`}>
                      {tx.signature.substring(0, 20)}...
                    </code>
                  </a>
                );
              })
            )}
          </div>
        </div>
      </div>

      <div className="px-4 py-2 bg-blue-900/10 border-t border-blue-700/30 flex-shrink-0">
        <div className="flex items-center justify-between">
          <div className="text-[9px] text-blue-300 font-semibold">
            Real Slab
          </div>
          <button
            onClick={() => {
              navigator.clipboard.writeText('5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB');
            }}
            className="text-[9px] text-zinc-400 hover:text-blue-300 font-mono transition-colors cursor-pointer"
            title="Click to copy Slab address"
          >
            5Yd2...CLDB
          </button>
        </div>
      </div>
    </div>
  )
}


