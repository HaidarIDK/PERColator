"use client"

import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { ShimmerButton } from "@/components/ui/shimmer-button"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
import { cn } from "@/lib/utils"
import { 
  Router, 
  Wallet, 
  User, 
  Shield, 
  Clock, 
  Zap, 
  Lock, 
  TrendingUp, 
  Layers, 
  Database, 
  Settings,
  Code2,
  Package,
  Terminal
} from "lucide-react"

const FeatureCard: React.FC<{ icon: React.ReactNode; title: string; description: string }> = ({
  icon,
  title,
  description,
}) => (
  <div className="group relative">
    {/* Decorative corner elements */}
    <div className="absolute -top-2 -left-2 w-4 h-4 border-t-2 border-l-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    <div className="absolute -top-2 -right-2 w-4 h-4 border-t-2 border-r-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    <div className="absolute -bottom-2 -left-2 w-4 h-4 border-b-2 border-l-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    <div className="absolute -bottom-2 -right-2 w-4 h-4 border-b-2 border-r-2 border-[#B8B8FF] opacity-60 group-hover:opacity-100 transition-opacity duration-300"></div>
    
    {/* Main card container */}
    <div className="relative rounded-2xl p-8 border border-[#181825] hover:border-[#B8B8FF]/50 transition-all duration-500 backdrop-blur-sm overflow-hidden bg-black/20">
     
      
      <div className="absolute inset-0 bg-gradient-to-br from-[#B8B8FF]/0 via-[#B8B8FF]/5 to-[#B8B8FF]/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
      
      <div className="relative z-10">
        <div className="w-14 h-14 rounded-xl bg-gradient-to-br from-[#B8B8FF]/20 to-[#B8B8FF]/30 flex items-center justify-center mb-6 group-hover:scale-110 group-hover:from-[#B8B8FF]/30 group-hover:to-[#B8B8FF]/40 transition-all duration-300">
          <div className="text-[#B8B8FF] group-hover:text-white transition-colors">
            {icon}
          </div>
        </div>
        
        <h3 className="text-xl font-bold mb-3 text-white group-hover:text-[#B8B8FF] transition-colors">
          {title}
        </h3>
        
        <p className="text-gray-400 text-base leading-relaxed group-hover:text-gray-300 transition-colors">
          {description}
        </p>
      </div>
    </div>
  </div>
);

export default function Home() {
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
      
      <main id="home" className="relative z-10 flex flex-col items-center justify-center min-h-screen px-4 text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8 }}
          className="max-w-6xl mx-auto"
        >
          {/* Main Title */}
          <div className="mb-8">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.2 }}
              className="text-6xl md:text-8xl lg:text-9xl font-bold mb-4"
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
            className="text-lg md:text-xl text-gray-400 mb-12 max-w-4xl mx-auto leading-relaxed"
          >
            a perp DEX program that just uses one slab of memory in an account for everything with its own LP/risk/matching engine
          </motion.p>

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

        </motion.div>
      </main>

      {/* Features Section */}
      <section id="features" className="relative z-10 py-32 px-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 1.8 }}
          className="max-w-7xl mx-auto"
        >
          <h2 className="text-5xl md:text-6xl font-bold text-center mb-24">
            <AuroraText
              speed={0.8}
              colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
              className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
            >
              Core Features
            </AuroraText>
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
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
              description="Router and Slab programs work together seamlessly with clear separation of concerns and responsibilities."
            />
          </div>
        </motion.div>
      </section>

      {/* Technical Architecture */}
      <section id="architecture" className="relative z-10 py-32 px-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 2.0 }}
          className="max-w-7xl mx-auto"
        >
          <h2 className="text-5xl md:text-6xl font-bold text-center mb-24">
            <AuroraText
              speed={0.8}
              colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
              className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
            >
              Technical Architecture
            </AuroraText>
          </h2>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
            {/* Architecture Diagram */}
            <div className="relative">
              <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-8">
                <h3 className="text-2xl font-bold text-[#B8B8FF] mb-6 text-center">Solana Program Architecture</h3>
                
                {/* Router Program */}
                <div className="mb-8">
                  <div className="bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-4 mb-4">
                    <h4 className="text-lg font-bold text-white mb-2">Router Program</h4>
                    <code className="text-xs text-gray-400">RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr</code>
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Vault - Collateral custody per asset</span>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Escrow - Per-user pledges with anti-replay</span>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Cap - Time-limited debit tokens (2min TTL)</span>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Portfolio - Cross-margin tracking</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Connection Arrow */}
                <div className="flex justify-center mb-6">
                  <div className="w-8 h-8 border-2 border-[#B8B8FF] rounded-full flex items-center justify-center">
                    <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                  </div>
                </div>

                {/* Slab Program */}
                <div className="mb-8">
                  <div className="bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-xl p-4">
                    <h4 className="text-lg font-bold text-white mb-2">Slab Program</h4>
                    <code className="text-xs text-gray-400">SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk</code>
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>SlabState - 10MB account with 5 memory pools</span>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Order Book - Price-time priority matching</span>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Risk Engine - IM/MM calculations</span>
                      </div>
                      <div className="flex items-center gap-2 text-sm text-gray-300">
                        <div className="w-2 h-2 bg-[#B8B8FF] rounded-full"></div>
                        <span>Anti-Toxicity - Kill Band, JIT Penalty, ARG</span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Memory Layout */}
                <div className="bg-gradient-to-r from-[#B8B8FF]/5 to-[#B8B8FF]/10 border border-[#B8B8FF]/20 rounded-xl p-4">
                  <h4 className="text-lg font-bold text-white mb-3">Memory Layout (10MB)</h4>
                  <div className="grid grid-cols-5 gap-2">
                    <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                      <div className="text-xs text-gray-400">Header</div>
                      <div className="text-xs text-white">64KB</div>
                    </div>
                    <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                      <div className="text-xs text-gray-400">Orders</div>
                      <div className="text-xs text-white">2MB</div>
                    </div>
                    <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                      <div className="text-xs text-gray-400">Reservations</div>
                      <div className="text-xs text-white">2MB</div>
                    </div>
                    <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                      <div className="text-xs text-gray-400">Positions</div>
                      <div className="text-xs text-white">2MB</div>
                    </div>
                    <div className="bg-[#B8B8FF]/20 rounded p-2 text-center">
                      <div className="text-xs text-gray-400">Free Lists</div>
                      <div className="text-xs text-white">2MB</div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Architecture Details */}
            <div className="space-y-8">
              <div>
                <h3 className="text-3xl font-bold text-white mb-6">Key Design Principles</h3>
                <div className="space-y-4">
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Shield className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-lg font-semibold text-white mb-1">Safety First</h4>
                      <p className="text-gray-400">Slabs cannot access Router vaults directly. All debits require unexpired, correctly scoped Caps.</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Zap className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-lg font-semibold text-white mb-1">Ultra-Fast Matching</h4>
                      <p className="text-gray-400">Price-time priority strictly maintained. O(1) freelist operations for maximum performance.</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Database className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-lg font-semibold text-white mb-1">Memory Efficiency</h4>
                      <p className="text-gray-400">Single 10MB account per market. No cross-contamination between users or slabs.</p>
                    </div>
                  </div>
                  
                  <div className="flex items-start gap-3">
                    <div className="w-6 h-6 rounded-full bg-[#B8B8FF]/20 flex items-center justify-center flex-shrink-0 mt-1">
                      <Settings className="w-3 h-3 text-[#B8B8FF]" />
                    </div>
                    <div>
                      <h4 className="text-lg font-semibold text-white mb-1">Anti-Toxicity</h4>
                      <p className="text-gray-400">Kill Band, JIT Penalty, and ARG mechanisms protect against toxic order flow.</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6">
                <h4 className="text-xl font-bold text-[#B8B8FF] mb-4">Technology Stack</h4>
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
        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.8, delay: 2.8 }}
        >
          © 2024 PERColator. Powered by Solana.
        </motion.p>
      </footer>
    </div>
  )
}