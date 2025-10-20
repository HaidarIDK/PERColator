"use client"

import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { ShimmerButton } from "@/components/ui/shimmer-button"
import { AnimatedBeam } from "@/components/ui/animated-beam"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
import { cn } from "@/lib/utils"
import { useRef } from "react"
import { Router, Wallet, User, Shield, Clock, Zap, Lock, TrendingUp, Layers, Database, Settings } from "lucide-react"

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
  const containerRef = useRef<HTMLDivElement>(null)
  const routerRef = useRef<HTMLDivElement>(null)
  const vaultRef = useRef<HTMLDivElement>(null)
  const portfolioRef = useRef<HTMLDivElement>(null)
  const escrowRef = useRef<HTMLDivElement>(null)
  const capRef = useRef<HTMLDivElement>(null)

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
                Per-colator
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
            className="flex flex-col sm:flex-row gap-6 justify-center items-center mb-16"
          >
             <ShimmerButton className="shadow-2xl">
              <span className="text-center text-sm leading-none font-medium tracking-tight whitespace-pre-wrap text-white lg:text-lg dark:from-white dark:to-slate-900/10">
                Shimmer Button
              </span>
            </ShimmerButton>
            
            <a 
              href="/dashboard"
              className="px-8 py-3 rounded-xl bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 text-white hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50 transition-all duration-300 backdrop-blur-sm"
            >
              View Dashboard
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
              icon={<Zap className="w-7 h-7" />}
              title="Lightning Fast"
              description="Ultra-low latency trading with sub-second execution times powered by our optimized memory architecture."
            />
            <FeatureCard
              icon={<Lock className="w-7 h-7" />}
              title="Secure Escrow"
              description="Advanced escrow system ensures your assets are protected with time-limited access controls and scope restrictions."
            />
            <FeatureCard
              icon={<TrendingUp className="w-7 h-7" />}
              title="Cross-Margin"
              description="Efficient capital utilization with unified margin management across all positions and trading pairs."
            />
          </div>
        </motion.div>
      </section>

      {/* Data Flow Visualization */}
      <section id="architecture" className="relative z-10 py-32 px-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 2.0 }}
          className="max-w-7xl mx-auto"
        >
          <h2 className="text-5xl md:text-6xl font-bold text-center mb-8">
            <AuroraText
              speed={0.8}
              colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
              className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
            >
              Data Flow Architecture
            </AuroraText>
          </h2>
          
          <p className="text-lg md:text-xl text-gray-400 text-center mb-16 max-w-3xl mx-auto leading-relaxed">
            Our unified memory architecture enables seamless data flow between core components, 
            ensuring efficient cross-margin trading with maximum security and minimal latency.
          </p>

          <div className="relative w-full max-w-4xl mx-auto flex justify-center items-center">
            {/* Decorative corner elements */}
            <div className="absolute -top-2 -left-2 w-4 h-4 border-t-2 border-l-2 border-[#B8B8FF] opacity-60"></div>
            <div className="absolute -top-2 -right-2 w-4 h-4 border-t-2 border-r-2 border-[#B8B8FF] opacity-60"></div>
            <div className="absolute -bottom-2 -left-2 w-4 h-4 border-b-2 border-l-2 border-[#B8B8FF] opacity-60"></div>
            <div className="absolute -bottom-2 -right-2 w-4 h-4 border-b-2 border-r-2 border-[#B8B8FF] opacity-60"></div>
            
            {/* Main container */}
            <div 
              ref={containerRef}
              className="relative w-full h-[700px] bg-black/3 border border-[#181825] rounded-2xl overflow-hidden"
            >
              <div className="absolute inset-0 bg-gradient-radial from-white/5 via-transparent to-transparent opacity-30"></div>
              
              {/* Decorative dots */}
              
            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  ref={routerRef}
                  className="absolute top-12 left-1/2 transform -translate-x-1/2 bg-[#B8B8FF]/0 backdrop-blur-md text-white rounded-full font-semibold shadow-[0_0_10px_rgba(184,184,255,0.5)] border border-[#B8B8FF]/30 w-20 h-20 flex items-center justify-center z-20 cursor-pointer hover:scale-110 hover:bg-[#B8B8FF]/0 hover:shadow-[0_0_15px_rgba(184,184,255,0.8)] transition-all duration-300"
                >
                  <Router className="w-8 h-8" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-xs">
                <div className="p-3">
                  <h4 className="font-bold text-white mb-2">Router (Custody)</h4>
                  <p className="text-sm text-gray-300">Central hub managing all data flow and transaction routing across the system</p>
                </div>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  ref={vaultRef}
                  className="absolute top-48 left-20 bg-[#B8B8FF]/0 backdrop-blur-md text-white rounded-full font-semibold shadow-[0_0_10px_rgba(184,184,255,0.5)] border border-[#B8B8FF]/30 w-20 h-20 flex items-center justify-center z-20 cursor-pointer hover:scale-110 hover:bg-[#B8B8FF]/0 hover:shadow-[0_0_15px_rgba(184,184,255,0.8)] transition-all duration-300"
                >
                  <Wallet className="w-8 h-8" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-xs">
                <div className="p-3">
                  <h4 className="font-bold text-white mb-2">Vault[USDC]</h4>
                  <p className="text-sm text-gray-300">Secure storage vault for USDC tokens with multi-signature protection</p>
                </div>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  ref={portfolioRef}
                  className="absolute top-48 right-20 bg-[#B8B8FF]/0 backdrop-blur-md text-white rounded-full font-semibold shadow-[0_0_10px_rgba(184,184,255,0.5)] border border-[#B8B8FF]/30 w-20 h-20 flex items-center justify-center z-20 cursor-pointer hover:scale-110 hover:bg-[#B8B8FF]/0 hover:shadow-[0_0_15px_rgba(184,184,255,0.8)] transition-all duration-300"
                >
                  <User className="w-8 h-8" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-xs">
                <div className="p-3">
                  <h4 className="font-bold text-white mb-2">Portfolio[User]</h4>
                  <p className="text-sm text-gray-300">Cross-margin portfolio management system tracking user positions and balances</p>
                </div>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  ref={escrowRef}
                  className="absolute bottom-48 left-32 bg-[#B8B8FF]/0 backdrop-blur-md text-white rounded-full font-semibold shadow-[0_0_10px_rgba(184,184,255,0.5)] border border-[#B8B8FF]/30 w-20 h-20 flex items-center justify-center z-20 cursor-pointer hover:scale-110 hover:bg-[#B8B8FF]/0 hover:shadow-[0_0_15px_rgba(184,184,255,0.8)] transition-all duration-300"
                >
                  <Shield className="w-8 h-8" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-xs">
                <div className="p-3">
                  <h4 className="font-bold text-white mb-2">Escrow(User, Slab, Asset)</h4>
                  <p className="text-sm text-gray-300">Secure escrow system for user assets with restricted access controls</p>
                </div>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <div
                  ref={capRef}
                  className="absolute bottom-48 right-32 bg-[#B8B8FF]/0 backdrop-blur-md text-white rounded-full font-semibold shadow-[0_0_10px_rgba(184,184,255,0.5)] border border-[#B8B8FF]/30 w-20 h-20 flex items-center justify-center z-20 cursor-pointer hover:scale-110 hover:bg-[#B8B8FF]/0 hover:shadow-[0_0_15px_rgba(184,184,255,0.8)] transition-all duration-300"
                >
                  <Clock className="w-8 h-8" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="max-w-xs">
                <div className="p-3">
                  <h4 className="font-bold text-white mb-2">Cap(Scope, Amount, Expiry)</h4>
                  <p className="text-sm text-gray-300">Time-limited debit tokens with scope and amount restrictions for secure operations</p>
                </div>
              </TooltipContent>
            </Tooltip>

            {/* Animated Beams */}
            <AnimatedBeam
              containerRef={containerRef}
              fromRef={routerRef}
              toRef={vaultRef}
              curvature={80}
              duration={4}
              delay={0}
              gradientStartColor="#B8B8FF"
              gradientStopColor="#B8B8FF"
              pathWidth={3}
            />
            <AnimatedBeam
              containerRef={containerRef}
              fromRef={routerRef}
              toRef={portfolioRef}
              curvature={80}
              duration={4}
              delay={0.8}
              gradientStartColor="#B8B8FF"
              gradientStopColor="#B8B8FF"
              pathWidth={3}
            />
            <AnimatedBeam
              containerRef={containerRef}
              fromRef={routerRef}
              toRef={escrowRef}
              curvature={-80}
              duration={4}
              delay={1.6}
              gradientStartColor="#B8B8FF"
              gradientStopColor="#B8B8FF"
              pathWidth={3}
            />
            <AnimatedBeam
              containerRef={containerRef}
              fromRef={routerRef}
              toRef={capRef}
              curvature={-80}
              duration={4}
              delay={2.4}
              gradientStartColor="#B8B8FF"
              gradientStopColor="#B8B8FF"
              pathWidth={3}
            />
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
          © 2024 Percolator AI. Powered by solana.
        </motion.p>
      </footer>
    </div>
  )
}
