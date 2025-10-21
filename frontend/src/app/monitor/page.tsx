"use client"

import { useState, useEffect } from "react"
import { motion } from "motion/react"
import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
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
      trade: 'text-[#B8B8FF]',
      position: 'text-[#B8B8FF]',
      liquidation: 'text-red-400',
      funding: 'text-[#B8B8FF]',
      reserve: 'text-yellow-400',
      commit: 'text-[#B8B8FF]',
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
      <div className="relative min-h-screen bg-black overflow-hidden">
        <Particles
          className="absolute inset-0 z-10"
          quantity={30}
          color="#B8B8FF"
          size={0.6}
          staticity={30}
          ease={80}
        />
        <div className="relative z-10 min-h-screen flex items-center justify-center">
          <div className="text-white">Loading monitor data...</div>
        </div>
      </div>
    )
  }

  return (
    <div className="relative min-h-screen bg-black overflow-hidden">
      <Particles
        className="absolute inset-0 z-10"
        quantity={30}
        color="#B8B8FF"
        size={0.6}
        staticity={30}
        ease={80}
      />
      
      <FloatingNavbar />
      
      <main className="relative z-10 pt-32 pb-20 px-4 text-white">
        {/* Header */}
        <div className="max-w-7xl mx-auto mb-8">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
            className="flex items-center justify-between"
          >
            <div>
              <h1 className="text-5xl md:text-6xl font-bold mb-4">
                <AuroraText
                  speed={0.8}
                  colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
                  className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
                >
                  Slab Monitor
                </AuroraText>
              </h1>
              <p className="text-xl text-gray-400">Real-time transaction and slab state monitoring</p>
            </div>
            <div className="flex gap-4 items-center">
              <Link 
                href="/dashboard"
                className="px-6 py-3 bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50 transition-all duration-300 backdrop-blur-sm text-white"
              >
                Back to Dashboard
              </Link>
              <button
                onClick={() => setAutoRefresh(!autoRefresh)}
                className={`px-6 py-3 rounded-xl transition-all duration-300 flex items-center gap-2 backdrop-blur-sm ${
                  autoRefresh 
                    ? 'bg-[#B8B8FF]/20 border border-[#B8B8FF]/50 text-white hover:bg-[#B8B8FF]/30' 
                    : 'bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 text-white hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50'
                }`}
              >
                <RefreshCw className={`w-4 h-4 ${autoRefresh ? 'animate-spin' : ''}`} />
                {autoRefresh ? 'Auto-Refresh ON' : 'Auto-Refresh OFF'}
              </button>
            </div>
          </motion.div>
        </div>

        {/* Stats Cards */}
        {stats && (
          <div className="max-w-7xl mx-auto grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="text-[#B8B8FF]">
                  <Users className="w-6 h-6" />
                </div>
                <div className="text-2xl font-bold text-white">{stats.total_users}</div>
              </div>
              <div className="text-sm text-gray-400">Total Users</div>
              <div className="text-xs text-[#B8B8FF] mt-1">
                {stats.active_users_24h} active (24h)
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
              className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="text-[#B8B8FF]">
                  <DollarSign className="w-6 h-6" />
                </div>
                <div className="text-2xl font-bold text-white">${(stats.total_volume_24h / 1000000).toFixed(2)}M</div>
              </div>
              <div className="text-sm text-gray-400">24h Volume</div>
              <div className="text-xs text-[#B8B8FF] mt-1">
                {stats.total_trades_24h} trades
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.2 }}
              className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="text-[#B8B8FF]">
                  <TrendingUp className="w-6 h-6" />
                </div>
                <div className="text-2xl font-bold text-white">${(stats.total_tvl / 1000000).toFixed(2)}M</div>
              </div>
              <div className="text-sm text-gray-400">Total TVL</div>
              <div className="text-xs text-[#B8B8FF] mt-1">
                ${(stats.total_open_interest / 1000000).toFixed(2)}M OI
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.3 }}
              className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="text-[#B8B8FF]">
                  <Activity className="w-6 h-6" />
                </div>
                <div className="text-2xl font-bold text-white">{stats.total_slabs}</div>
              </div>
              <div className="text-sm text-gray-400">Active Slabs</div>
              <div className="text-xs text-[#B8B8FF] mt-1">
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
              className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
            >
              <h2 className="text-2xl font-bold mb-4 flex items-center gap-2 text-white">
                <Eye className="w-6 h-6 text-[#B8B8FF]" />
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

              <div className="bg-gradient-to-br from-[#B8B8FF]/20 to-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-4">
                <div className="text-sm text-[#B8B8FF] mb-1 flex items-center gap-2">
                  <span>Rent Required</span>
                  <span className="text-xs bg-[#B8B8FF]/20 text-[#B8B8FF] px-2 py-0.5 rounded-full font-bold">ULTRA-CHEAP!</span>
                </div>
                <div className="text-3xl font-bold text-white mb-1">
                  {slabState.stats.estimated_rent_sol.toFixed(2)} SOL
                </div>
                <div className="text-xs text-[#B8B8FF] font-semibold mb-1">
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
            className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
          >
            <h2 className="text-2xl font-bold mb-4 flex items-center gap-2 text-white">
              <Activity className="w-6 h-6 text-[#B8B8FF]" />
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
                  className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/20 rounded-xl p-4 hover:border-[#B8B8FF]/40 hover:shadow-lg hover:shadow-[#B8B8FF]/10 transition-all duration-300"
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
                        <span className="font-mono bg-[#B8B8FF]/10 border border-[#B8B8FF]/20 px-2 py-1 rounded">{tx.user}</span>
                      </div>
                    )}

                    {tx.amount && (
                      <div className="text-sm font-bold text-[#B8B8FF]">
                        ${tx.amount.toLocaleString()}
                      </div>
                    )}
                  </div>

                  {tx.signature && (
                    <div className="text-xs text-gray-400 font-mono break-all mt-2 bg-[#B8B8FF]/5 border border-[#B8B8FF]/10 px-2 py-1 rounded">
                      {tx.signature}
                    </div>
                  )}
                </motion.div>
              ))
            )}
          </div>
        </motion.div>
        </div>
      </main>
    </div>
  )
}

