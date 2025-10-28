"use client"

import { useState, useEffect } from "react"
import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { ShimmerButton } from "@/components/ui/shimmer-button"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
import { 
  Shield, 
  Zap, 
  Lock, 
  TrendingUp, 
  Layers, 
  Database, 
  GitBranch,
  Settings
} from "lucide-react"

const FeatureCard: React.FC<{ icon: React.ReactNode; title: string; description: string }> = ({
  icon,
  title,
  description,
}) => (
  <div className="group relative h-full">
    {/* Decorative corner elements */}
    <div className="absolute -top-2 -left-2 w-3 h-3 sm:w-4 sm:h-4 border-t-2 border-l-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    <div className="absolute -top-2 -right-2 w-3 h-3 sm:w-4 sm:h-4 border-t-2 border-r-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    <div className="absolute -bottom-2 -left-2 w-3 h-3 sm:w-4 sm:h-4 border-b-2 border-l-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    <div className="absolute -bottom-2 -right-2 w-3 h-3 sm:w-4 sm:h-4 border-b-2 border-r-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    
    {/* Main card container */}
    <div className="relative h-full rounded-2xl p-4 sm:p-8 border border-[#181825] hover:border-[#B8B8FF]/50 transition-all duration-500 backdrop-blur-sm overflow-hidden bg-black/20">
     
      
      <div className="absolute inset-0 bg-gradient-to-br from-[#B8B8FF]/0 via-[#B8B8FF]/5 to-[#B8B8FF]/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
      
      <div className="relative z-10">
        <div className="w-10 h-10 sm:w-14 sm:h-14 rounded-xl bg-gradient-to-br from-[#B8B8FF]/20 to-[#B8B8FF]/30 flex items-center justify-center mb-4 sm:mb-6 group-hover:scale-110 group-hover:from-[#B8B8FF]/30 group-hover:to-[#B8B8FF]/40 transition-all duration-300">
          <div className="text-[#B8B8FF] group-hover:text-white transition-colors">
            {icon}
          </div>
        </div>
        
        <h3 className="text-lg sm:text-xl font-bold mb-2 sm:mb-3 text-white group-hover:text-[#B8B8FF] transition-colors">
          {title}
        </h3>
        
        <p className="text-gray-400 text-sm sm:text-base leading-relaxed group-hover:text-gray-300 transition-colors">
          {description}
        </p>
      </div>
    </div>
  </div>
);

export default function Home() {
  const [commitsAhead, setCommitsAhead] = useState<number | null>(null)
  const [showCopiedToast, setShowCopiedToast] = useState(false)

  useEffect(() => {
    async function fetchCommitsAhead() {
      try {
        const response = await fetch('https://api.github.com/repos/HaidarIDK/PERColator/compare/aeyakovenko:master...master')
        if (!response.ok) return
        const data = await response.json()
        setCommitsAhead(data.ahead_by || 0)
      } catch (error) {
        console.error('Failed to fetch commit count:', error)
      }
    }
    fetchCommitsAhead()
  }, [])

  const handleCopyAddress = () => {
    navigator.clipboard.writeText('CXobgfkQT6wCysehb3abkuimkmx5chS62fZew9NBpump')
    setShowCopiedToast(true)
    setTimeout(() => setShowCopiedToast(false), 3000)
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
      
      {/* Floating Navbar */}
      <FloatingNavbar />
      
      {/* X/Twitter Button - Top Right */}
      <a 
        href="https://x.com/DexPERColator" 
        target="_blank" 
        rel="noopener noreferrer"
        className="fixed top-6 right-6 z-50 w-12 h-12 rounded-full bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/30 border border-[#B8B8FF]/40 flex items-center justify-center hover:from-[#B8B8FF]/30 hover:to-[#B8B8FF]/40 hover:border-[#B8B8FF]/60 transition-all duration-300 hover:shadow-[0_0_20px_rgba(184,184,255,0.5)] backdrop-blur-md"
      >
        <svg className="w-6 h-6 text-white" viewBox="0 0 24 24" fill="currentColor">
          <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
        </svg>
      </a>
      
      <main id="home" className="relative z-10 flex flex-col items-center justify-center min-h-screen px-3 sm:px-4 text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8 }}
          className="max-w-6xl mx-auto"
        >
          {/* Main Title */}
          <div className="mb-6 sm:mb-8">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.2 }}
              className="text-4xl sm:text-6xl md:text-8xl lg:text-9xl font-bold mb-4"
            >
              <AuroraText
                speed={0.8}
                colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
                className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
              >
                PERColator
              </AuroraText>
            </motion.div>
          </div>

          <motion.p
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.2 }}
            className="text-sm sm:text-lg md:text-xl text-gray-400 mb-6 max-w-4xl mx-auto leading-relaxed px-2"
          >
            A perpetual DEX with multiple slabs - each liquidity provider can have their own 10MB slab with dedicated matching, risk, and LP management.
          </motion.p>

          {/* Contract Address */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.3 }}
            className="mb-6 max-w-3xl mx-auto px-2"
          >
            <div className="bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/10 border border-[#B8B8FF]/40 rounded-xl p-3 sm:p-4 backdrop-blur-sm">
              <div className="flex flex-col items-center gap-2">
                <span className="text-xs font-semibold text-[#B8B8FF] uppercase tracking-wider">Contract Address</span>
                <div className="flex items-center justify-center gap-1 sm:gap-2 bg-black/30 px-3 sm:px-4 py-2 rounded-lg border border-[#B8B8FF]/20 mx-auto">
                  <code className="text-[10px] sm:text-sm md:text-base text-white font-mono text-center">
                    CXobgfkQT6wCysehb3abkuimkmx5chS62fZew9NBpump
                  </code>
                  <button
                    onClick={handleCopyAddress}
                    className="p-1.5 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 text-[#B8B8FF] transition-all duration-200 flex-shrink-0"
                    title="Copy to clipboard"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </motion.div>

          {/* Fork Attribution */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.4 }}
            className="mb-8 sm:mb-12 max-w-2xl mx-auto px-2"
          >
            <div className="bg-gradient-to-r from-[#B8B8FF]/10 to-[#B8B8FF]/5 border border-[#B8B8FF]/30 rounded-xl p-3 sm:p-4 backdrop-blur-sm">
              <div className="flex flex-col sm:flex-row items-center justify-center gap-1.5 sm:gap-2 text-xs sm:text-sm text-gray-300 text-center">
                <GitBranch className="w-4 h-4 text-[#B8B8FF] flex-shrink-0" />
                <span className="flex flex-wrap items-center justify-center gap-1">
                  <span>Forked from</span>
                  <a 
                    href="https://github.com/aeyakovenko/percolator" 
                    target="_blank" 
                    rel="noopener noreferrer"
                    className="text-[#B8B8FF] hover:text-white transition-colors underline"
                  >
                    Anatoly Yakovenko&apos;s Percolator
                  </a>
                </span>
                <span className="text-gray-500 hidden sm:inline">|</span>
                <span className="text-[#B8B8FF] font-semibold">
                  {commitsAhead === null ? '...' : commitsAhead === 0 ? 'Up to date' : `${commitsAhead} commit${commitsAhead !== 1 ? 's' : ''} ahead`}
                </span>
              </div>
              <p className="text-[10px] sm:text-xs text-gray-500 text-center mt-2">
                This is an enhanced fork with production-ready infrastructure, not a copy of the original work.
              </p>
            </div>
          </motion.div>

          {/* CTA Buttons */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.6 }}
            className="flex justify-center items-center mb-16"
          >
             <a href="/info">
               <ShimmerButton className="shadow-2xl">
                 <span className="text-center text-sm leading-none font-medium tracking-tight whitespace-pre-wrap text-white lg:text-lg dark:from-white dark:to-slate-900/10">
                   Read More
                 </span>
               </ShimmerButton>
             </a>
          </motion.div>

          {/* Scroll Down Indicator */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 1, delay: 2.0 }}
            className="flex justify-center mt-8"
          >
            <motion.div
              animate={{ y: [0, 10, 0] }}
              transition={{ duration: 1.5, repeat: Infinity, ease: "easeInOut" }}
              className="flex flex-col items-center gap-2 cursor-pointer"
              onClick={() => {
                window.scrollTo({
                  top: document.documentElement.scrollHeight,
                  behavior: 'smooth'
                });
              }}
            >
              <span className="text-sm text-gray-400">Scroll to explore</span>
              <svg
                className="w-6 h-6 text-[#B8B8FF]"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M19 14l-7 7m0 0l-7-7m7 7V3"
                />
              </svg>
            </motion.div>
          </motion.div>

        </motion.div>
      </main>

      {/* Key Concepts Section - Moved to Top */}
      <section className="relative z-10 py-12 sm:py-20 px-3 sm:px-4 bg-gradient-to-b from-black to-black/50">
        <div className="max-w-7xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
            viewport={{ once: true }}
            className="text-center mb-10 sm:mb-16"
          >
            <h2 className="text-3xl sm:text-4xl md:text-5xl font-bold text-white mb-3 sm:mb-4">
              Key Concepts
            </h2>
            <p className="text-base sm:text-xl text-gray-400 max-w-3xl mx-auto px-2">
              Essential terms to understand how PERColator works
            </p>
          </motion.div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-6">
            {/* Slab */}
            <FeatureCard
              title="Slab"
              description="A 10MB memory block containing an entire market - order book, positions, and all trading data in one contiguous space for maximum speed."
              icon={<Database className="w-6 h-6" />}
            />

            {/* Reserve-Commit */}
            <FeatureCard
              title="Reserve-Commit"
              description="Two-phase trading: First 'reserve' locks liquidity at current prices, then 'commit' executes the trade - preventing price manipulation between steps."
              icon={<Lock className="w-6 h-6" />}
            />

            {/* Slice */}
            <FeatureCard
              title="Slice"
              description="A piece of reserved liquidity from a specific order. Your reservation might grab slices from multiple orders to fill your size."
              icon={<Layers className="w-6 h-6" />}
            />

            {/* DLP */}
            <FeatureCard
              title="DLP"
              description="Designated Liquidity Provider - VIP traders who can post orders immediately and trade during freeze windows. Think of them as market makers with special access."
              icon={<Shield className="w-6 h-6" />}
            />

            {/* Kill Band */}
            <FeatureCard
              title="Kill Band"
              description="Safety check that rejects trades if the price moved too much (default 1%) since you reserved. Protects you from stale prices."
              icon={<Zap className="w-6 h-6" />}
            />

            {/* ARG */}
            <FeatureCard
              title="ARG Tax"
              description="Aggressor Roundtrip Guard - detects if you buy AND sell in the same batch (sandwich attempt) and charges you extra. Anti-manipulation."
              icon={<Shield className="w-6 h-6" />}
            />

            {/* Portfolio */}
            <FeatureCard
              title="Portfolio"
              description="Your cross-market account tracking total exposure and risk across all slabs. One portfolio, many markets."
              icon={<TrendingUp className="w-6 h-6" />}
            />

            {/* Capability Token */}
            <FeatureCard
              title="Capability Token"
              description="Time-limited (2min max) authorization pass that lets specific markets access your funds. Scoped security - no blanket permissions."
              icon={<Lock className="w-6 h-6" />}
            />

            {/* Funding Rate */}
            <FeatureCard
              title="Funding Rate"
              description="Hourly payment between longs and shorts that keeps perpetual prices anchored to spot. If perp > spot, longs pay shorts (and vice versa)."
              icon={<TrendingUp className="w-6 h-6" />}
            />
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="relative z-10 py-16 sm:py-32 px-3 sm:px-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 1.8 }}
          className="max-w-7xl mx-auto"
        >
          <h2 className="text-3xl sm:text-5xl md:text-6xl font-bold text-center mb-12 sm:mb-24">
            <AuroraText
              speed={0.8}
              colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
              className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
            >
              Core Features
            </AuroraText>
          </h2>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 sm:gap-8">
            <FeatureCard
              icon={<Database className="w-8 h-8" />}
              title="Unified Memory Architecture"
              description="Single 10MB account per market containing order book, positions, reservations, and free lists in one cohesive structure."
            />
            <FeatureCard
              icon={<Zap className="w-8 h-8" />}
              title="Ultra-Low Latency"
              description="Sub-second execution times with O(1) operations and optimized memory pools for maximum trading performance."
            />
            <FeatureCard
              icon={<Shield className="w-8 h-8" />}
              title="Advanced Security"
              description="Capability-based access control with time-limited tokens and anti-replay protection for secure operations."
            />
            <FeatureCard
              icon={<TrendingUp className="w-8 h-8" />}
              title="Cross-Margin Trading"
              description="Portfolio-level risk management with cross-slab margin calculations and unified collateral system."
            />
            <FeatureCard
              icon={<Lock className="w-8 h-8" />}
              title="Anti-Toxicity Mechanisms"
              description="Kill Band, JIT Penalty, and ARG systems protect against toxic order flow and ensure fair trading."
            />
            <FeatureCard
              icon={<Layers className="w-8 h-8" />}
              title="Modular Design"
              description="Four specialized programs (Router, Slab, AMM, Oracle) work together seamlessly with clear separation of concerns."
            />
          </div>
        </motion.div>
      </section>

      {/* Technical Architecture */}
      <section id="architecture" className="relative z-10 py-16 sm:py-32 px-3 sm:px-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 2.0 }}
          className="max-w-7xl mx-auto"
        >
          <h2 className="text-3xl sm:text-5xl md:text-6xl font-bold text-center mb-12 sm:mb-24">
            <AuroraText
              speed={0.8}
              colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
              className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
            >
              Technical Architecture
            </AuroraText>
          </h2>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 sm:gap-12 items-center">
            {/* Architecture Diagram */}
            <div className="relative">
              <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 sm:p-8">
                <h3 className="text-xl sm:text-2xl font-bold text-[#B8B8FF] mb-6 text-center">Solana Program Architecture</h3>
                
                {/* Router Program */}
                <div className="mb-6">
                  <div className="bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-3 sm:p-4 mb-4">
                    <h4 className="text-base sm:text-lg font-bold text-white mb-2">Router Program</h4>
                    <code className="text-[10px] sm:text-xs text-gray-400 break-all">RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr</code>
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Cross-slab routing & portfolio margin</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Vault custody & collateral management</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Liquidation engine & risk monitoring</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Slab Program */}
                <div className="mb-6">
                  <div className="bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-3 sm:p-4">
                    <h4 className="text-base sm:text-lg font-bold text-white mb-2">Slab Program</h4>
                    <code className="text-[10px] sm:text-xs text-gray-400 break-all">SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep</code>
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>10MB unified order book & matching</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Price-time priority & fill receipts</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Anti-toxicity protection (Kill Band, JIT)</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* AMM Program */}
                <div className="mb-6">
                  <div className="bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-3 sm:p-4">
                    <h4 className="text-base sm:text-lg font-bold text-white mb-2">AMM Program</h4>
                    <code className="text-[10px] sm:text-xs text-gray-400 break-all">AMMjkEeFdasQ8fs9a9HQyJdciPHtDHVEat8yxiXrTP6p</code>
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Automated market maker pools (x·y=k)</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>LP token minting & liquidity rewards</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Dynamic fee optimization</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Oracle Program */}
                <div className="mb-6">
                  <div className="bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-3 sm:p-4">
                    <h4 className="text-base sm:text-lg font-bold text-white mb-2">Oracle Program</h4>
                    <code className="text-[10px] sm:text-xs text-gray-400 break-all">oracpooXY8Nnpx2JTLkrLiJsDaMefERUFFRktkAZ3ki</code>
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Real-time price feed aggregation</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Multi-source oracle verification</span>
                      </div>
                      <div className="flex items-center gap-2 text-xs sm:text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full flex-shrink-0"></div>
                        <span>Mark price calculation & funding rates</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Memory Layout - v0 and v1 */}
                <div className="space-y-4">
                  {/* v0 Layout - Current */}
                  <div className="bg-gradient-to-r from-green-500/5 to-green-500/10 border border-green-500/30 rounded-xl p-3 sm:p-4">
                    <div className="flex items-center justify-between mb-2">
                      <h4 className="text-base sm:text-lg font-bold text-white">Slab v0 Layout</h4>
                      <span className="text-[10px] sm:text-xs px-2 py-1 rounded-full bg-green-500/20 text-green-400 font-semibold">CURRENT</span>
                    </div>
                    <div className="text-[10px] sm:text-xs text-gray-400 mb-3 text-center">Minimal ~4KB Account</div>
                    <div className="grid grid-cols-3 gap-2 text-[10px] sm:text-xs">
                      <div className="bg-green-500/20 rounded p-2 text-center">
                        <div className="text-gray-300">Header</div>
                        <div className="text-white font-semibold">256B</div>
                      </div>
                      <div className="bg-green-500/20 rounded p-2 text-center">
                        <div className="text-gray-300">QuoteCache</div>
                        <div className="text-white font-semibold">256B</div>
                      </div>
                      <div className="bg-green-500/20 rounded p-2 text-center">
                        <div className="text-gray-300">BookArea</div>
                        <div className="text-white font-semibold">3KB</div>
                      </div>
                    </div>
                    <div className="mt-3 pt-2 border-t border-green-500/20 text-center">
                      <div className="text-xs sm:text-sm text-green-400 font-bold">Total: ~3.5KB</div>
                      <div className="text-[10px] text-gray-500 mt-1">Proves core routing concept</div>
                    </div>
                  </div>

                  {/* v1 Layout - Future */}
                  <div className="bg-gradient-to-r from-[#B8B8FF]/5 to-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-3 sm:p-4">
                    <div className="flex items-center justify-between mb-2">
                      <h4 className="text-base sm:text-lg font-bold text-white">Slab v1 Layout</h4>
                      <span className="text-[10px] sm:text-xs px-2 py-1 rounded-full bg-[#B8B8FF]/20 text-[#B8B8FF] font-semibold">PLANNED</span>
                    </div>
                    <div className="text-[10px] sm:text-xs text-gray-400 mb-3 text-center">Full 10MB Unified Account</div>
                    <div className="grid grid-cols-2 gap-2 text-[10px] sm:text-xs">
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Header</div>
                        <div className="text-white font-semibold">200B</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Accounts</div>
                        <div className="text-white font-semibold">320KB</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Orders</div>
                        <div className="text-white font-semibold">2.4MB</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Positions</div>
                        <div className="text-white font-semibold">1.4MB</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Reservations</div>
                        <div className="text-white font-semibold">480KB</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Slices</div>
                        <div className="text-white font-semibold">512KB</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Trades</div>
                        <div className="text-white font-semibold">800KB</div>
                      </div>
                      <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                        <div className="text-gray-300">Aggressor</div>
                        <div className="text-white font-semibold">192KB</div>
                      </div>
                    </div>
                    <div className="mt-3 pt-2 border-t border-[#B8B8FF]/20 text-center">
                      <div className="text-xs sm:text-sm text-[#B8B8FF] font-bold">Total: ~6.1MB</div>
                      <div className="text-[10px] text-gray-500 mt-1">3.9MB available for expansion</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Architecture Details */}
            <div className="space-y-8">
              <div>
                <h3 className="text-2xl sm:text-3xl font-bold text-white mb-6">Four Program System</h3>
                <div className="space-y-4">
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <GitBranch className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-base sm:text-lg font-semibold text-white mb-1">Router - Multi-Slab Routing</h4>
                      <p className="text-sm sm:text-base text-gray-400">Orchestrates cross-slab atomic swaps, manages collateral vaults, and handles portfolio margin with liquidation monitoring.</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Layers className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-base sm:text-lg font-semibold text-white mb-1">Slab - Order Book Engine</h4>
                      <p className="text-sm sm:text-base text-gray-400">10MB unified memory with price-time priority matching, fill receipts, and anti-toxicity protection (Kill Band, JIT Penalty, ARG).</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <TrendingUp className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-base sm:text-lg font-semibold text-white mb-1">AMM - Liquidity Pools</h4>
                      <p className="text-sm sm:text-base text-gray-400">Constant product market maker (x·y=k) with LP token rewards, dynamic fees, and deep liquidity for popular pairs.</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Database className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-base sm:text-lg font-semibold text-white mb-1">Oracle - Price Feeds</h4>
                      <p className="text-sm sm:text-base text-gray-400">Real-time multi-source price aggregation with mark price calculation, funding rates, and oracle verification for accurate liquidations.</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Shield className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-base sm:text-lg font-semibold text-white mb-1">Security & Safety</h4>
                      <p className="text-sm sm:text-base text-gray-400">Programs cannot access vaults directly. All operations require time-limited capability tokens with strict scope validation.</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6">
                <h4 className="text-xl font-bold text-[#B8B8FF] mb-4 text-center">Technology Stack</h4>
                <div className="grid grid-cols-2 gap-4">
                  <div className="text-center">
                    <div className="text-2xl font-bold text-white">Pinocchio</div>
                    <div className="text-sm text-gray-400">Zero-dependency Solana SDK</div>
                  </div>
                  <div className="text-center">
                    <div className="text-2xl font-bold text-white">Rust</div>
                    <div className="text-sm text-gray-400">no_std, zero allocations</div>
                  </div>
                  <div className="text-center">
                    <div className="text-2xl font-bold text-white">Surfpool</div>
                    <div className="text-sm text-gray-400">Local test validator</div>
                  </div>
                  <div className="text-center">
                    <div className="text-2xl font-bold text-white">140+ Tests</div>
                    <div className="text-sm text-gray-400">Comprehensive coverage</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </motion.div>
      </section>

      {/* Footer */}
      <footer id="about" className="relative z-10 text-center py-8 text-gray-500">
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.8, delay: 2.8 }}
          className="flex flex-col items-center gap-4"
        >
          <a 
            href="https://x.com/DexPERColator" 
            target="_blank" 
            rel="noopener noreferrer"
            className="w-10 h-10 rounded-full bg-gradient-to-r from-[#B8B8FF]/10 to-[#B8B8FF]/20 border border-[#B8B8FF]/30 flex items-center justify-center hover:from-[#B8B8FF]/20 hover:to-[#B8B8FF]/30 hover:border-[#B8B8FF]/50 transition-all duration-300 group"
          >
            <svg className="w-5 h-5 text-gray-400 group-hover:text-white transition-colors" viewBox="0 0 24 24" fill="currentColor">
              <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
            </svg>
          </a>
          <p>© 2025 PERColator. Powered by Solana.</p>
        </motion.div>
      </footer>

      {/* Copy Toast Notification */}
      {showCopiedToast && (
        <motion.div
          initial={{ opacity: 0, y: 50 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 50 }}
          className="fixed bottom-8 left-1/2 transform -translate-x-1/2 z-[9999] pointer-events-none"
        >
          <div className="bg-gradient-to-r from-green-500/90 to-emerald-500/90 backdrop-blur-lg px-6 py-3 rounded-xl border border-green-400/50 shadow-lg shadow-green-500/20 flex items-center gap-3">
            <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
            <span className="text-white font-semibold text-sm">Contract address copied to clipboard!</span>
          </div>
        </motion.div>
      )}
    </div>
  )
}

// Glossary Card Component
function GlossaryCard({ term, definition, icon }: { term: string; definition: string; icon: React.ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6 }}
      viewport={{ once: true }}
      className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300 group"
    >
      <div className="flex items-start gap-4 mb-3">
        <div className="p-2 rounded-lg bg-[#B8B8FF]/20 text-[#B8B8FF] group-hover:bg-[#B8B8FF]/30 transition-colors flex-shrink-0">
          {icon}
        </div>
        <h3 className="text-lg font-bold text-white group-hover:text-[#B8B8FF] transition-colors">
          {term}
        </h3>
      </div>
      <p className="text-sm text-gray-400 leading-relaxed group-hover:text-gray-300 transition-colors">
        {definition}
      </p>
    </motion.div>
  )
}