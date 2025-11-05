"use client"

import { useState, useEffect } from "react"
import { useWallet } from "@solana/wallet-adapter-react"
import { cn } from "@/lib/utils"

interface PastTradesProps {
  symbol: string
}

export const PastTrades = ({ symbol }: PastTradesProps) => {
  const [pastTrades, setPastTrades] = useState<any[]>([])
  const [loading, setLoading] = useState(true)
  const wallet = useWallet();
  const { publicKey, connected } = wallet;

  useEffect(() => {
    if (!connected || !publicKey) {
      setLoading(false);
      return;
    }

    // Fetch user's past trades
    const fetchPastTrades = async () => {
      try {
        const walletAddress = publicKey.toBase58();
        // TODO: Replace with actual API endpoint when available
        // For now, show mock data
        const mockTrades = [
          {
            id: '1',
            symbol: symbol,
            side: 'buy',
            price: 184.50,
            quantity: 0.5,
            timestamp: Date.now() - 300000, // 5 minutes ago
            status: 'filled',
            fee: 0.02
          },
          {
            id: '2', 
            symbol: symbol,
            side: 'sell',
            price: 185.20,
            quantity: 0.3,
            timestamp: Date.now() - 600000, // 10 minutes ago
            status: 'filled',
            fee: 0.02
          },
          {
            id: '3',
            symbol: symbol,
            side: 'buy',
            price: 183.80,
            quantity: 1.0,
            timestamp: Date.now() - 900000, // 15 minutes ago
            status: 'filled',
            fee: 0.02
          }
        ];
        setPastTrades(mockTrades);
        setLoading(false);
      } catch (error) {
        setLoading(false);
      }
    };

    fetchPastTrades();
    
    // Refresh every 30 seconds
    const interval = setInterval(fetchPastTrades, 30000);
    return () => clearInterval(interval);
  }, [symbol, connected, publicKey]);

  return (
    <div className="w-full bg-black/20 rounded-2xl border border-[#181825] overflow-hidden">
      <div className="h-10 flex items-center px-3 border-b border-[#181825]">
        <h3 className="text-white font-medium text-sm">Past Trades</h3>
        {loading && <span className="text-xs text-gray-500 ml-2">Loading...</span>}
      </div>
      
      <div className="p-3">
        {!connected ? (
          <div className="text-center py-8">
            <div className="text-gray-500 text-sm">Connect wallet to view trades</div>
          </div>
        ) : pastTrades.length > 0 ? (
          <>
            <div className="grid grid-cols-4 gap-1 sm:gap-2 text-[10px] sm:text-xs text-gray-400 mb-2">
              <span className="truncate">Side</span>
              <span className="truncate">Price</span>
              <span className="truncate">Qty</span>
              <span className="truncate">Time</span>
            </div>
            
            <div className="space-y-1 max-h-[300px] overflow-y-auto [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
              {pastTrades.map((trade) => (
                <div key={trade.id} className="grid grid-cols-4 gap-1 sm:gap-2 text-xs sm:text-sm py-1 hover:bg-[#181825]/50 rounded px-1">
                  <span className={cn(
                    "text-xs font-medium",
                    trade.side === 'buy' ? "text-green-400" : "text-red-400"
                  )}>
                    {trade.side.toUpperCase()}
                  </span>
                  <span className="text-white">{trade.price.toFixed(2)}</span>
                  <span className="text-gray-300">{trade.quantity.toFixed(4)}</span>
                  <span className="text-gray-400 text-xs" suppressHydrationWarning>
                    {new Date(trade.timestamp).toLocaleTimeString()}
                  </span>
                </div>
              ))}
            </div>
            
            <div className="mt-3 pt-2 border-t border-[#181825]">
              <div className="text-xs text-gray-400">
                Total Trades: <span className="text-white">{pastTrades.length}</span>
              </div>
            </div>
          </>
        ) : (
          <div className="text-center py-8">
            <div className="text-gray-500 text-sm">No past trades found</div>
            <div className="text-xs text-gray-400 mt-1">Your trades will appear here</div>
          </div>
        )}
      </div>
    </div>
  )
}


