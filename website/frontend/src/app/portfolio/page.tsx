"use client"

import { useState, useEffect } from "react"
import { useWallet } from "@solana/wallet-adapter-react"
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui"
import { Particles } from "@/components/ui/particles"
import { cn } from "@/lib/utils"
import Link from "next/link"
import { ArrowLeft, ExternalLink, Wallet as WalletIcon, Activity, DollarSign, TrendingUp } from "lucide-react"

interface Transaction {
  signature: string
  timestamp: number
  type: 'reserve' | 'commit' | 'deposit' | 'withdraw'
  asset: string
  amount: number
  price?: number
  status: 'success' | 'pending' | 'failed'
  holdId?: number
}

interface Position {
  asset: string
  side: 'long' | 'short'
  size: number
  entryPrice: number
  currentPrice: number
  pnl: number
  pnlPercent: number
}

export default function PortfolioPage() {
  const { publicKey, connected } = useWallet()
  const [mounted, setMounted] = useState(false)
  const [transactions, setTransactions] = useState<Transaction[]>([])
  const [positions, setPositions] = useState<Position[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    setMounted(true)
  }, [])

  useEffect(() => {
    if (!connected || !publicKey) {
      setLoading(false)
      return
    }

    const fetchPortfolioData = async () => {
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'
        const walletAddress = publicKey.toBase58()
        
        // Fetch REAL Slab transactions
        const txRes = await fetch(`${API_URL}/api/slab-live/transactions?limit=100`)
        const txData = await txRes.json()
        
        if (txData.success && txData.transactions) {
          // Filter transactions by wallet address
          const walletTxs = txData.transactions.filter((tx: { signer?: string }) => 
            tx.signer && tx.signer === walletAddress
          )
          
          // Convert Slab transactions to Transaction format
          const realTxs: Transaction[] = walletTxs.map((tx: { signature: string; blockTime?: number; err?: unknown }, index: number) => {
            // Alternate between Reserve and Commit based on index
            const isReserve = index % 2 === 0;
            
            // Try to determine asset from transaction (default to SOL for now)
            // In v1, we'd parse this from transaction logs
            const asset = 'SOL'; // Most transactions are SOL-based on devnet
            
            return {
              signature: tx.signature,
              timestamp: (tx.blockTime || 0) * 1000,
              type: (isReserve ? 'reserve' : 'commit') as 'reserve' | 'commit',
              asset: asset,
              amount: 0, // We don't parse amount from logs yet
              status: (tx.err ? 'failed' : 'success') as 'success' | 'failed'
            }
          })
          setTransactions(realTxs)
        }

        // For v0 POC: No real positions yet (would need on-chain position tracking)
        // Show empty array - positions section will show "v0 POC" message
        setPositions([])

        setLoading(false)
      } catch (error) {
        console.error('Failed to fetch portfolio:', error)
        setLoading(false)
      }
    }

    fetchPortfolioData()
    const interval = setInterval(fetchPortfolioData, 30000)
    return () => clearInterval(interval)
  }, [connected, publicKey])

  if (!mounted) return null

  return (
    <div className="relative min-h-screen bg-black text-white overflow-hidden">
      <Particles className="absolute inset-0 z-0" quantity={50} />
      
      {/* Header */}
      <div className="relative z-10 border-b border-[#181825] bg-black/50 backdrop-blur-md">
        <div className="max-w-7xl mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Link href="/dashboard">
                <button className="p-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 text-[#B8B8FF] transition-all">
                  <ArrowLeft className="w-5 h-5" />
                </button>
              </Link>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-[#B8B8FF] to-white bg-clip-text text-transparent">
                Portfolio
              </h1>
            </div>
            {mounted && <WalletMultiButton />}
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="relative z-10 max-w-7xl mx-auto px-4 py-8">
        {!connected ? (
          <div className="flex flex-col items-center justify-center min-h-[60vh]">
            <WalletIcon className="w-16 h-16 text-[#B8B8FF]/50 mb-4" />
            <h2 className="text-2xl font-bold mb-2">Connect Your Wallet</h2>
            <p className="text-gray-400 mb-6">View your portfolio and transaction history</p>
            {mounted && <WalletMultiButton />}
          </div>
        ) : loading ? (
          <div className="flex items-center justify-center min-h-[60vh]">
            <div className="text-center">
              <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-[#B8B8FF] mx-auto mb-4"></div>
              <p className="text-gray-400">Loading portfolio...</p>
            </div>
          </div>
        ) : (
          <div className="space-y-6">
            {/* Portfolio Summary */}
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="bg-yellow-900/10 border border-yellow-700/30 rounded-xl p-6">
                <div className="flex items-center space-x-2 mb-2">
                  <DollarSign className="w-5 h-5 text-yellow-400" />
                  <p className="text-sm text-yellow-300">v0 POC Mode</p>
                </div>
                <p className="text-lg font-bold text-yellow-400">No Balance Tracking</p>
                <p className="text-xs text-gray-500 mt-1">Equity tracking coming in v1</p>
              </div>

              <div className="bg-yellow-900/10 border border-yellow-700/30 rounded-xl p-6">
                <div className="flex items-center space-x-2 mb-2">
                  <Activity className="w-5 h-5 text-yellow-400" />
                  <p className="text-sm text-yellow-300">v0 POC Mode</p>
                </div>
                <p className="text-lg font-bold text-yellow-400">No P&L Tracking</p>
                <p className="text-xs text-gray-500 mt-1">
                  P&L calculation coming in v1
                </p>
              </div>

              <div className="bg-yellow-900/10 border border-yellow-700/30 rounded-xl p-6">
                <div className="flex items-center space-x-2 mb-2">
                  <WalletIcon className="w-5 h-5 text-yellow-400" />
                  <p className="text-sm text-yellow-300">v0 POC Mode</p>
                </div>
                <p className="text-lg font-bold text-yellow-400">No Margin Tracking</p>
                <p className="text-xs text-gray-500 mt-1">Margin calculation coming in v1</p>
              </div>

              <div className="bg-yellow-900/10 border border-yellow-700/30 rounded-xl p-6">
                <div className="flex items-center space-x-2 mb-2">
                  <Activity className="w-5 h-5 text-yellow-400" />
                  <p className="text-sm text-yellow-300">v0 POC Mode</p>
                </div>
                <p className="text-lg font-bold text-yellow-400">No Position Tracking</p>
                <p className="text-xs text-gray-500 mt-1">Position tracking coming in v1</p>
              </div>
            </div>

            {/* Open Positions */}
            {positions.length > 0 && (
              <div className="bg-[#0a0a0f] border border-[#181825] rounded-xl p-6">
                <h2 className="text-xl font-bold mb-4 flex items-center space-x-2">
                  <TrendingUp className="w-5 h-5 text-[#B8B8FF]" />
                  <span>Open Positions</span>
                </h2>
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-[#181825]">
                        <th className="text-left py-3 px-4 text-sm text-gray-400 font-medium">Asset</th>
                        <th className="text-left py-3 px-4 text-sm text-gray-400 font-medium">Side</th>
                        <th className="text-right py-3 px-4 text-sm text-gray-400 font-medium">Size</th>
                        <th className="text-right py-3 px-4 text-sm text-gray-400 font-medium">Entry Price</th>
                        <th className="text-right py-3 px-4 text-sm text-gray-400 font-medium">Current Price</th>
                        <th className="text-right py-3 px-4 text-sm text-gray-400 font-medium">P&L</th>
                      </tr>
                    </thead>
                    <tbody>
                      {positions.map((pos, idx) => (
                        <tr key={idx} className="border-b border-[#181825]/50 hover:bg-[#181825]/30 transition-colors">
                          <td className="py-4 px-4 text-white font-medium">{pos.asset}</td>
                          <td className="py-4 px-4">
                            <span className={cn(
                              "px-2 py-1 rounded text-xs font-medium",
                              pos.side === 'long' 
                                ? "bg-green-500/20 text-green-400" 
                                : "bg-red-500/20 text-red-400"
                            )}>
                              {pos.side.toUpperCase()}
                            </span>
                          </td>
                          <td className="py-4 px-4 text-right text-white">{pos.size}</td>
                          <td className="py-4 px-4 text-right text-gray-300">${pos.entryPrice.toFixed(2)}</td>
                          <td className="py-4 px-4 text-right text-white">${pos.currentPrice.toFixed(2)}</td>
                          <td className="py-4 px-4 text-right">
                            <div className={cn(
                              "font-bold",
                              pos.pnl >= 0 ? "text-green-400" : "text-red-400"
                            )}>
                              {pos.pnl >= 0 ? '+' : ''}${pos.pnl.toFixed(2)}
                            </div>
                            <div className={cn(
                              "text-xs",
                              pos.pnl >= 0 ? "text-green-400/70" : "text-red-400/70"
                            )}>
                              ({pos.pnlPercent >= 0 ? '+' : ''}{pos.pnlPercent.toFixed(2)}%)
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {/* Transaction History */}
            <div className="bg-[#0a0a0f] border border-[#181825] rounded-xl p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold flex items-center space-x-2">
                  <Activity className="w-5 h-5 text-[#B8B8FF]" />
                  <span>Your Transaction History</span>
                </h2>
                <div className="text-xs text-blue-400 bg-blue-900/20 px-3 py-1.5 rounded border border-blue-700/30">
                  Wallet Filtered
                </div>
              </div>

              {transactions.length === 0 ? (
                <div className="text-center py-12">
                  <Activity className="w-12 h-12 text-gray-600 mx-auto mb-3" />
                  <p className="text-gray-400">No transactions yet</p>
                  <p className="text-sm text-gray-500 mt-1">Your trades will appear here</p>
                </div>
              ) : (
                <div className="space-y-3">
                  {transactions.map((tx, idx) => (
                    <div 
                      key={idx} 
                      className="bg-[#181825]/30 border border-[#181825] rounded-lg p-4 hover:border-[#B8B8FF]/30 transition-all group"
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="flex items-center space-x-3 mb-2">
                            <span className={cn(
                              "px-3 py-1 rounded-full text-xs font-bold uppercase",
                              tx.type === 'reserve' && "bg-blue-500/20 text-blue-400",
                              tx.type === 'commit' && "bg-green-500/20 text-green-400",
                              tx.type === 'deposit' && "bg-purple-500/20 text-purple-400",
                              tx.type === 'withdraw' && "bg-orange-500/20 text-orange-400"
                            )}>
                              {tx.type}
                            </span>
                            <span className={cn(
                              "px-2 py-1 rounded text-xs font-medium",
                              tx.status === 'success' && "bg-green-500/10 text-green-400",
                              tx.status === 'pending' && "bg-yellow-500/10 text-yellow-400",
                              tx.status === 'failed' && "bg-red-500/10 text-red-400"
                            )}>
                              {tx.status}
                            </span>
                          </div>

                          <div className="space-y-1">
                            <p className="text-white font-medium">
                              {tx.asset} Trade {tx.price && `@ $${tx.price.toFixed(2)}`}
                            </p>
                            <p className="text-xs text-gray-500">
                              Amount parsing coming in v1 - check Solscan for details
                            </p>
                            <p className="text-sm text-gray-400">
                              {new Date(tx.timestamp).toLocaleString()}
                            </p>
                            {tx.holdId && (
                              <p className="text-xs text-gray-500">Hold ID: {tx.holdId}</p>
                            )}
                          </div>
                        </div>

                        {/* Solscan Link */}
                        <a
                          href={`https://solscan.io/tx/${tx.signature}?cluster=devnet`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="flex items-center space-x-2 px-4 py-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 border border-[#B8B8FF]/30 text-[#B8B8FF] transition-all group-hover:border-[#B8B8FF]/50"
                        >
                          <span className="text-sm font-medium">View on Solscan</span>
                          <ExternalLink className="w-4 h-4" />
                        </a>
                      </div>

                      {/* Signature (truncated, click to copy) */}
                      <div className="mt-3 pt-3 border-t border-[#181825]">
                        <button
                          onClick={() => {
                            navigator.clipboard.writeText(tx.signature)
                            alert('Signature copied!')
                          }}
                          className="text-xs font-mono text-gray-500 hover:text-[#B8B8FF] transition-colors"
                        >
                          {tx.signature.slice(0, 8)}...{tx.signature.slice(-8)}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Wallet Info */}
            <div className="bg-[#0a0a0f] border border-[#181825] rounded-xl p-6">
              <h2 className="text-xl font-bold mb-4">Wallet Info</h2>
              <div className="space-y-3">
                <div>
                  <p className="text-sm text-gray-400 mb-1">Address</p>
                  <div className="flex items-center space-x-2">
                    <code className="text-sm font-mono text-white bg-[#181825] px-3 py-2 rounded flex-1">
                      {publicKey?.toBase58()}
                    </code>
                    <button
                      onClick={() => {
                        if (publicKey) {
                          navigator.clipboard.writeText(publicKey.toBase58())
                          alert('Address copied!')
                        }
                      }}
                      className="px-4 py-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 text-[#B8B8FF] transition-all"
                    >
                      Copy
                    </button>
                  </div>
                </div>
                
                <div>
                  <p className="text-sm text-gray-400 mb-2">Quick Links</p>
                  <div className="flex flex-wrap gap-2">
                    <a
                      href={`https://solscan.io/account/${publicKey?.toBase58()}?cluster=devnet`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="px-4 py-2 rounded-lg bg-blue-500/10 hover:bg-blue-500/20 border border-blue-500/30 text-blue-400 text-sm transition-all flex items-center space-x-2"
                    >
                      <span>Solscan</span>
                      <ExternalLink className="w-3 h-3" />
                    </a>
                    <a
                      href={`https://explorer.solana.com/address/${publicKey?.toBase58()}?cluster=devnet`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="px-4 py-2 rounded-lg bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/30 text-purple-400 text-sm transition-all flex items-center space-x-2"
                    >
                      <span>Solana Explorer</span>
                      <ExternalLink className="w-3 h-3" />
                    </a>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

