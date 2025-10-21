"use client"

import { useState, useEffect } from "react"
import { motion } from "motion/react"
import { 
  Activity, 
  TrendingUp, 
  Users, 
  DollarSign,
  ArrowUpRight,
  ArrowDownRight,
  RefreshCw,
  Eye,
  Clock,
  CheckCircle,
  XCircle,
  AlertCircle
} from "lucide-react"
import Link from "next/link"

interface Transaction {
  id: number
  type: string
  timestamp: number
  user?: string
  action?: string
  amount?: number
  signature?: string
}

interface SlabState {
  address: string
  header: {
    current_batch_id: number
    imr: number
    mmr: number
    maker_fee: number
    taker_fee: number
    current_ts: number
  }
  stats: {
    total_accounts: number
    active_accounts: number
    total_orders: number
    total_positions: number
    memory_usage_kb?: number
    memory_usage_mb?: number
    estimated_rent_sol: number
  }
}

export default function MonitorPage() {
  const [transactions, setTransactions] = useState<Transaction[]>([])
  const [slabState, setSlabState] = useState<SlabState | null>(null)
  const [stats, setStats] = useState<any>(null)
  const [loading, setLoading] = useState(true)
  const [autoRefresh, setAutoRefresh] = useState(true)

  const fetchData = async () => {
    try {
      // Fetch only last 5 transactions
      const txRes = await fetch('http://localhost:3000/api/monitor/transactions?limit=5')
      const txData = await txRes.json()
      if (txData.success) {
        setTransactions(txData.transactions)
      }

      // Fetch slab state
      const slabRes = await fetch('http://localhost:3000/api/monitor/slab/11111111111111111111111111111111')
      const slabData = await slabRes.json()
      if (slabData.success) {
        setSlabState(slabData.slab)
      }

      // Fetch global stats
      const statsRes = await fetch('http://localhost:3000/api/monitor/stats')
      const statsData = await statsRes.json()
      if (statsData.success) {
        setStats(statsData.stats)
      }

      setLoading(false)
    } catch (error) {
      console.error('Error fetching monitor data:', error)
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchData()
    
    const interval = autoRefresh ? setInterval(fetchData, 5000) : null
    
    return () => {
      if (interval) clearInterval(interval)
    }
  }, [autoRefresh])

  const formatTime = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000)
    if (seconds < 60) return `${seconds}s ago`
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    return `${Math.floor(seconds / 3600)}h ago`
  }

  const getTypeColor = (type: string) => {
    const colors: Record<string, string> = {
      trade: 'text-green-400',
      position: 'text-blue-400',
      liquidation: 'text-red-400',
      funding: 'text-purple-400',
      reserve: 'text-yellow-400',
      commit: 'text-cyan-400',
    }
    return colors[type] || 'text-gray-400'
  }

  const getTypeIcon = (type: string) => {
    switch(type) {
      case 'trade': return <TrendingUp className="w-4 h-4" />
      case 'position': return <Activity className="w-4 h-4" />
      case 'liquidation': return <AlertCircle className="w-4 h-4" />
      case 'reserve': return <Clock className="w-4 h-4" />
      case 'commit': return <CheckCircle className="w-4 h-4" />
      default: return <Activity className="w-4 h-4" />
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-black flex items-center justify-center">
        <div className="text-white">Loading monitor data...</div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-black text-white p-6">
      {/* Header */}
      <div className="max-w-7xl mx-auto mb-8">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-4xl font-bold mb-2">Slab Monitor</h1>
            <p className="text-gray-400">Real-time transaction and slab state monitoring</p>
          </div>
          <div className="flex gap-4 items-center">
            <Link 
              href="/dashboard"
              className="px-4 py-2 bg-gray-800 rounded-lg hover:bg-gray-700 transition-colors"
            >
              Back to Dashboard
            </Link>
            <button
              onClick={() => setAutoRefresh(!autoRefresh)}
              className={`px-4 py-2 rounded-lg transition-colors flex items-center gap-2 ${
                autoRefresh ? 'bg-green-600 hover:bg-green-700' : 'bg-gray-800 hover:bg-gray-700'
              }`}
            >
              <RefreshCw className={`w-4 h-4 ${autoRefresh ? 'animate-spin' : ''}`} />
              {autoRefresh ? 'Auto-Refresh ON' : 'Auto-Refresh OFF'}
            </button>
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      {stats && (
        <div className="max-w-7xl mx-auto grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-gradient-to-br from-purple-900/30 to-purple-600/10 border border-purple-500/30 rounded-lg p-6"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="text-purple-400">
                <Users className="w-6 h-6" />
              </div>
              <div className="text-2xl font-bold">{stats.total_users}</div>
            </div>
            <div className="text-sm text-gray-400">Total Users</div>
            <div className="text-xs text-green-400 mt-1">
              {stats.active_users_24h} active (24h)
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="bg-gradient-to-br from-green-900/30 to-green-600/10 border border-green-500/30 rounded-lg p-6"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="text-green-400">
                <DollarSign className="w-6 h-6" />
              </div>
              <div className="text-2xl font-bold">${(stats.total_volume_24h / 1000000).toFixed(2)}M</div>
            </div>
            <div className="text-sm text-gray-400">24h Volume</div>
            <div className="text-xs text-green-400 mt-1">
              {stats.total_trades_24h} trades
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            className="bg-gradient-to-br from-blue-900/30 to-blue-600/10 border border-blue-500/30 rounded-lg p-6"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="text-blue-400">
                <TrendingUp className="w-6 h-6" />
              </div>
              <div className="text-2xl font-bold">${(stats.total_tvl / 1000000).toFixed(2)}M</div>
            </div>
            <div className="text-sm text-gray-400">Total TVL</div>
            <div className="text-xs text-blue-400 mt-1">
              ${(stats.total_open_interest / 1000000).toFixed(2)}M OI
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
            className="bg-gradient-to-br from-cyan-900/30 to-cyan-600/10 border border-cyan-500/30 rounded-lg p-6"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="text-cyan-400">
                <Activity className="w-6 h-6" />
              </div>
              <div className="text-2xl font-bold">{stats.total_slabs}</div>
            </div>
            <div className="text-sm text-gray-400">Active Slabs</div>
            <div className="text-xs text-green-400 mt-1">
              {stats.system_health} • {stats.uptime}% uptime
            </div>
          </motion.div>
        </div>
      )}

      <div className="max-w-7xl mx-auto grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Slab State */}
        {slabState && (
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            className="bg-gray-900/50 border border-gray-800 rounded-lg p-6"
          >
            <h2 className="text-2xl font-bold mb-4 flex items-center gap-2">
              <Eye className="w-6 h-6 text-cyan-400" />
              Slab State
            </h2>
            
            <div className="space-y-4">
              <div className="bg-black/30 rounded-lg p-4">
                <div className="text-sm text-gray-400 mb-2">Address</div>
                <div className="font-mono text-xs break-all">{slabState.address}</div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="bg-black/30 rounded-lg p-4">
                  <div className="text-sm text-gray-400 mb-1">Batch ID</div>
                  <div className="text-xl font-bold">{slabState.header.current_batch_id}</div>
                </div>
                <div className="bg-black/30 rounded-lg p-4">
                  <div className="text-sm text-gray-400 mb-1">Total Orders</div>
                  <div className="text-xl font-bold">{slabState.stats.total_orders}</div>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="bg-black/30 rounded-lg p-4">
                  <div className="text-sm text-gray-400 mb-1">IMR / MMR</div>
                  <div className="text-lg font-bold">
                    {(slabState.header.imr / 100).toFixed(2)}% / {(slabState.header.mmr / 100).toFixed(2)}%
                  </div>
                </div>
                <div className="bg-black/30 rounded-lg p-4">
                  <div className="text-sm text-gray-400 mb-1">Fees</div>
                  <div className="text-lg font-bold">
                    M: {(slabState.header.maker_fee / 100).toFixed(2)}% / T: {(slabState.header.taker_fee / 100).toFixed(2)}%
                  </div>
                </div>
              </div>

              <div className="bg-black/30 rounded-lg p-4">
                <div className="grid grid-cols-3 gap-4 text-center">
                  <div>
                    <div className="text-2xl font-bold text-purple-400">{slabState.stats.active_accounts}</div>
                    <div className="text-xs text-gray-400">Active Accounts</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold text-blue-400">{slabState.stats.total_positions}</div>
                    <div className="text-xs text-gray-400">Positions</div>
                  </div>
                  <div>
                    <div className="text-2xl font-bold text-green-400">
                      {slabState.stats.memory_usage_kb 
                        ? `${slabState.stats.memory_usage_kb} KB` 
                        : `${slabState.stats.memory_usage_mb?.toFixed(1)} MB`}
                    </div>
                    <div className="text-xs text-gray-400">Memory Used</div>
                  </div>
                </div>
              </div>

              <div className="bg-gradient-to-br from-cyan-900/30 to-green-900/20 border border-cyan-500/30 rounded-lg p-4">
                <div className="text-sm text-cyan-400 mb-1 flex items-center gap-2">
                  <span>Rent Required</span>
                  <span className="text-xs bg-green-500/20 text-green-400 px-2 py-0.5 rounded-full font-bold">ULTRA-CHEAP!</span>
                </div>
                <div className="text-3xl font-bold text-cyan-300 mb-1">
                  {slabState.stats.estimated_rent_sol.toFixed(2)} SOL
                </div>
                <div className="text-xs text-green-400 font-semibold mb-1">
                  Saved 72.5 SOL vs 10MB design! 🎉
                </div>
                <div className="text-xs text-gray-400">
                  ~${(slabState.stats.estimated_rent_sol * 150).toFixed(2)} @ $150/SOL
                </div>
              </div>
            </div>
          </motion.div>
        )}

        {/* Transaction Feed */}
        <motion.div
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          className="bg-gray-900/50 border border-gray-800 rounded-lg p-6"
        >
          <h2 className="text-2xl font-bold mb-4 flex items-center gap-2">
            <Activity className="w-6 h-6 text-green-400" />
            Last 5 Transactions
          </h2>

          <div className="space-y-3">
            {transactions.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <Activity className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <div className="font-medium">No transactions yet</div>
                <div className="text-sm">Waiting for activity...</div>
              </div>
            ) : (
              transactions.map((tx, index) => (
                <motion.div
                  key={tx.id}
                  initial={{ opacity: 0, scale: 0.95 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ delay: index * 0.05 }}
                  className="bg-gradient-to-br from-gray-900/80 to-gray-900/50 border border-gray-800 rounded-lg p-4 hover:border-gray-700 hover:shadow-lg hover:shadow-gray-900/50 transition-all duration-200"
                >
                  <div className="flex items-start justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <div className={`p-1.5 rounded-md ${getTypeColor(tx.type).replace('text-', 'bg-')}/20`}>
                        <span className={getTypeColor(tx.type)}>
                          {getTypeIcon(tx.type)}
                        </span>
                      </div>
                      <span className="font-bold capitalize text-white">{tx.type}</span>
                    </div>
                    <div className="flex flex-col items-end">
                      <span className="text-xs text-gray-500">{formatTime(tx.timestamp)}</span>
                      <span className="text-xs text-gray-600">#{index + 1}</span>
                    </div>
                  </div>

                  {tx.action && (
                    <div className="text-sm text-gray-300 mb-2 font-medium">{tx.action}</div>
                  )}

                  <div className="flex items-center justify-between">
                    {tx.user && (
                      <div className="text-xs text-gray-500">
                        <span className="font-mono bg-gray-800/50 px-2 py-1 rounded">{tx.user}</span>
                      </div>
                    )}

                    {tx.amount && (
                      <div className="text-sm font-bold text-green-400">
                        ${tx.amount.toLocaleString()}
                      </div>
                    )}
                  </div>

                  {tx.signature && (
                    <div className="text-xs text-gray-600 font-mono break-all mt-2 bg-black/30 px-2 py-1 rounded">
                      {tx.signature}
                    </div>
                  )}
                </motion.div>
              ))
            )}
          </div>
        </motion.div>
      </div>

    </div>
  )
}

