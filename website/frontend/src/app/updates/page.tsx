"use client"

import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
import { 
  GitCommit, 
  Calendar, 
  ExternalLink, 
  Target,
  CheckCircle2,
  Circle,
  Clock
} from "lucide-react"
import { useState, useEffect } from "react"

interface Commit {
  sha: string
  commit: {
    message: string
    author: {
      name: string
      date: string
    }
  }
  html_url: string
}

interface RoadmapItem {
  title: string
  status: "completed" | "in-progress" | "planned"
  description: string
}

const roadmapItems: RoadmapItem[] = [
  {
    title: "Mobile Responsive UI",
    status: "completed",
    description: "Dashboard and homepage fully optimized for mobile devices"
  },
  {
    title: "Live Market Data Integration",
    status: "completed",
    description: "Real-time price feeds from Hyperliquid WebSocket"
  },
  {
    title: "Multi-Slab Router Program",
    status: "completed",
    description: "On-chain program deployed with cross-slab atomic swap logic and optimal routing"
  },
  {
    title: "Order Book (Slab) Program",
    status: "completed",
    description: "10MB unified order book with price-time priority matching and anti-toxicity protection"
  },
  {
    title: "AMM Program",
    status: "completed",
    description: "Constant product AMM (x·y=k) deployed on-chain with LP token support"
  },
  {
    title: "Oracle Program",
    status: "completed",
    description: "Price feed oracle deployed on-chain for mark price and funding rate calculations"
  },
  {
    title: "Liquidation Keeper Bot",
    status: "in-progress",
    description: "Off-chain keeper infrastructure built, pending full integration with live positions"
  },
  {
    title: "Portfolio Margin System",
    status: "in-progress",
    description: "Backend logic complete, integrating cross-market risk management into UI"
  },
  {
    title: "Trading Interface Integration",
    status: "in-progress",
    description: "Connecting frontend trading forms to on-chain programs for live order execution"
  },
  {
    title: "Advanced Analytics Dashboard",
    status: "planned",
    description: "Comprehensive trading metrics, PnL charts, and performance tracking"
  }
]

export default function UpdatesPage() {
  const [commits, setCommits] = useState<Commit[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    async function fetchCommits() {
      try {
        const response = await fetch('https://api.github.com/repos/HaidarIDK/PERColator/commits?per_page=20')
        if (!response.ok) {
          throw new Error('Failed to fetch commits')
        }
        const data = await response.json()
        setCommits(data)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load commits')
      } finally {
        setLoading(false)
      }
    }

    fetchCommits()
  }, [])

  const formatDate = (dateString: string) => {
    const date = new Date(dateString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)
    const diffHours = Math.floor(diffMs / 3600000)
    const diffDays = Math.floor(diffMs / 86400000)

    if (diffMins < 60) return `${diffMins} minutes ago`
    if (diffHours < 24) return `${diffHours} hours ago`
    if (diffDays < 7) return `${diffDays} days ago`
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="w-5 h-5 text-green-400" />
      case "in-progress":
        return <Clock className="w-5 h-5 text-yellow-400" />
      case "planned":
        return <Circle className="w-5 h-5 text-gray-400" />
      default:
        return <Circle className="w-5 h-5 text-gray-400" />
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "from-green-500/20 to-emerald-500/20 border-green-500/30"
      case "in-progress":
        return "from-yellow-500/20 to-orange-500/20 border-yellow-500/30"
      case "planned":
        return "from-gray-500/20 to-gray-600/20 border-gray-500/30"
      default:
        return "from-gray-500/20 to-gray-600/20 border-gray-500/30"
    }
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
      
      <main className="relative z-10 pt-32 pb-20 px-3 sm:px-4">
        <div className="max-w-6xl mx-auto">
          {/* Header */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
            className="text-center mb-12 sm:mb-16"
          >
            <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold mb-4 sm:mb-6">
              <AuroraText
                speed={0.8}
                colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
                className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
              >
                Updates & Roadmap
              </AuroraText>
            </h1>
            <p className="text-base sm:text-xl text-gray-400 max-w-3xl mx-auto px-2">
              Track development progress and upcoming features
            </p>
          </motion.div>

          {/* Roadmap Section */}
          <motion.section
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className="mb-12 sm:mb-16"
          >
            <div className="flex items-center gap-3 mb-6 sm:mb-8">
              <Target className="w-6 h-6 sm:w-8 sm:h-8 text-[#B8B8FF]" />
              <h2 className="text-2xl sm:text-3xl font-bold text-white">Development Roadmap</h2>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 sm:gap-6">
              {roadmapItems.map((item, index) => (
                <motion.div
                  key={item.title}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.6, delay: 0.3 + index * 0.1 }}
                  className={`bg-gradient-to-br ${getStatusColor(item.status)} border rounded-xl p-4 sm:p-6 backdrop-blur-sm hover:scale-105 transition-transform duration-300`}
                >
                  <div className="flex items-start gap-3 mb-3">
                    {getStatusIcon(item.status)}
                    <div className="flex-1">
                      <h3 className="text-lg sm:text-xl font-bold text-white mb-1">{item.title}</h3>
                      <span className={`text-xs font-semibold uppercase tracking-wider ${
                        item.status === "completed" ? "text-green-400" :
                        item.status === "in-progress" ? "text-yellow-400" :
                        "text-gray-400"
                      }`}>
                        {item.status.replace("-", " ")}
                      </span>
                    </div>
                  </div>
                  <p className="text-sm text-gray-400 leading-relaxed">{item.description}</p>
                </motion.div>
              ))}
            </div>
          </motion.section>

          {/* Recent Commits Section */}
          <motion.section
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.4 }}
          >
            <div className="flex items-center gap-3 mb-6 sm:mb-8">
              <GitCommit className="w-6 h-6 sm:w-8 sm:h-8 text-[#B8B8FF]" />
              <h2 className="text-2xl sm:text-3xl font-bold text-white">Recent Updates</h2>
            </div>

            {loading ? (
              <div className="text-center py-12">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-[#B8B8FF] mx-auto mb-4"></div>
                <p className="text-gray-400">Loading commits...</p>
              </div>
            ) : error ? (
              <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-6 text-center">
                <p className="text-red-400">{error}</p>
              </div>
            ) : (
              <div className="space-y-3 sm:space-y-4">
                {commits.map((commit, index) => (
                  <motion.a
                    key={commit.sha}
                    href={commit.html_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ duration: 0.4, delay: 0.5 + index * 0.05 }}
                    className="block bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-4 sm:p-6 hover:border-[#B8B8FF]/50 hover:bg-[#B8B8FF]/5 transition-all duration-300 group"
                  >
                    <div className="flex items-start gap-3 sm:gap-4">
                      <div className="flex-shrink-0 w-10 h-10 sm:w-12 sm:h-12 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center text-[#B8B8FF] group-hover:bg-[#B8B8FF]/30 transition-colors">
                        <GitCommit className="w-5 h-5 sm:w-6 sm:h-6" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-start justify-between gap-2 mb-2">
                          <h3 className="text-sm sm:text-base font-semibold text-white group-hover:text-[#B8B8FF] transition-colors line-clamp-2">
                            {commit.commit.message.split('\n')[0]}
                          </h3>
                          <ExternalLink className="w-4 h-4 text-gray-400 group-hover:text-[#B8B8FF] transition-colors flex-shrink-0" />
                        </div>
                        <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-3 text-xs sm:text-sm text-gray-400">
                          <span className="flex items-center gap-1.5">
                            <Calendar className="w-3 h-3 sm:w-4 sm:h-4" />
                            {formatDate(commit.commit.author.date)}
                          </span>
                          <span className="hidden sm:inline text-gray-600">•</span>
                          <span className="font-mono text-[10px] sm:text-xs text-gray-500 truncate">
                            {commit.sha.substring(0, 7)}
                          </span>
                        </div>
                      </div>
                    </div>
                  </motion.a>
                ))}
              </div>
            )}

            {/* GitHub Link */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.6 }}
              className="mt-8 sm:mt-12 text-center"
            >
              <a
                href="https://github.com/HaidarIDK/PERColator"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/30 text-white font-semibold hover:from-[#B8B8FF]/30 hover:to-[#B8B8FF]/40 transition-all duration-300 shadow-[0_0_15px_rgba(184,184,255,0.3)] hover:shadow-[0_0_20px_rgba(184,184,255,0.5)]"
              >
                <span className="text-sm sm:text-base">View Full History on GitHub</span>
                <ExternalLink className="w-4 h-4 sm:w-5 sm:h-5" />
              </a>
            </motion.div>
          </motion.section>
        </div>
      </main>
    </div>
  )
}


