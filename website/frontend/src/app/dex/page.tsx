"use client"

import Link from "next/link"
import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { Zap, Clock, Code2 } from "lucide-react"

export default function ComingSoonPage() {
  return (
    <div className="relative min-h-screen bg-black overflow-hidden flex items-center justify-center">
      <Particles
        className="absolute inset-0 z-10"
        quantity={50}
        color="#B8B8FF"
        size={0.8}
        staticity={30}
        ease={80}
      />
      
      <main className="relative z-10 px-4 text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8 }}
          className="max-w-4xl mx-auto"
        >
          {/* Coming Soon Badge */}
          <motion.div
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 mb-8"
          >
            <Clock className="w-4 h-4 text-[#B8B8FF]" />
            <span className="text-sm text-[#B8B8FF] font-medium">Coming Soon</span>
          </motion.div>

          {/* Main Title */}
          <div className="mb-8">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.8, delay: 0.4 }}
              className="text-6xl md:text-8xl lg:text-9xl font-bold mb-4"
            >
              <AuroraText
                speed={0.8}
                colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
                className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
              >
                PerpDEX
              </AuroraText>
            </motion.div>
          </div>

          {/* Description */}
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.6 }}
            className="text-xl md:text-2xl text-gray-400 mb-12 max-w-3xl mx-auto leading-relaxed"
          >
            The trading interface for PERColator is currently under development. Get ready for the fastest perpetual DEX on Solana.
          </motion.p>

          {/* Features Preview */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.8 }}
            className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12"
          >
            <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6">
              <Zap className="w-8 h-8 text-[#B8B8FF] mb-3 mx-auto" />
              <h3 className="text-lg font-bold text-white mb-2">Ultra-Fast Trading</h3>
              <p className="text-sm text-gray-400">Sub-second execution with unified memory architecture</p>
            </div>

            <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6">
              <Code2 className="w-8 h-8 text-[#B8B8FF] mb-3 mx-auto" />
              <h3 className="text-lg font-bold text-white mb-2">Advanced Features</h3>
              <p className="text-sm text-gray-400">Reserve-commit trading with anti-toxicity protection</p>
            </div>

            <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6">
              <svg className="w-8 h-8 text-[#B8B8FF] mb-3 mx-auto" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 2L2 7v10c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V7l-10-5z"/>
              </svg>
              <h3 className="text-lg font-bold text-white mb-2">Cross-Margin Safety</h3>
              <p className="text-sm text-gray-400">Portfolio-level risk management across all markets</p>
            </div>
          </motion.div>

          {/* Back to Home Link */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.0 }}
          >
            <Link
              href="/"
              className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 text-white hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50 transition-all duration-300 backdrop-blur-sm"
            >
              &larr; Back to Home
            </Link>
          </motion.div>

          {/* Status Message */}
          <motion.p
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.8, delay: 1.2 }}
            className="mt-12 text-sm text-gray-500"
          >
            We&apos;re working hard to bring you the best trading experience on Solana.
          </motion.p>
        </motion.div>
      </main>
    </div>
  )
}

