"use client"

import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
import { 
  Github, 
  ExternalLink, 
  CheckCircle2, 
  Code2, 
  Database, 
  Rocket,
  FileCode,
  Terminal,
  Package,
  Settings,
  Shield,
  Zap
} from "lucide-react"

export default function InfoPage() {
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
      
      <main className="relative z-10 pt-32 pb-20 px-4">
        <div className="max-w-6xl mx-auto">
          {/* Header */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
            className="text-center mb-16"
          >
            <h1 className="text-5xl md:text-7xl font-bold mb-6">
              <AuroraText
                speed={0.8}
                colors={["#B8B8FF", "#B8B8FF", "#B8B8FF", "#B8B8FF"]}
                className="bg-gradient-to-r from-orange-400 via-yellow-400 to-orange-500 bg-clip-text text-transparent"
              >
                Project Documentation
              </AuroraText>
            </h1>
            <p className="text-xl text-gray-400 max-w-3xl mx-auto">
              Complete technical overview of PERColator - A sharded perpetual exchange protocol on Solana
            </p>
          </motion.div>

          {/* GitHub Link */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className="mb-12"
          >
            <a
              href="https://github.com/HaidarIDK/percolator"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center justify-center gap-3 w-full max-w-md mx-auto px-6 py-4 rounded-xl bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 text-white hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50 transition-all duration-300 backdrop-blur-sm group"
            >
              <Github className="w-6 h-6 group-hover:rotate-12 transition-transform" />
              <span className="text-lg font-semibold">View on GitHub</span>
              <ExternalLink className="w-5 h-5 group-hover:translate-x-1 group-hover:-translate-y-1 transition-transform" />
            </a>
          </motion.div>

          {/* Architecture Overview */}
          <Section
            icon={<Database className="w-6 h-6" />}
            title="Architecture"
            delay={0.4}
          >
            <p className="text-gray-300 mb-4">
              PERColator consists of two main on-chain Solana programs:
            </p>
            
            <div className="space-y-4">
              <Card
                title="Router Program"
                programId="RoutR1VdCpHqj89WEMJhb6TkGT9cPfr1rVjhM3e2YQr"
                description="Global coordinator managing collateral, portfolio margin, and cross-slab routing"
                features={[
                  "Vault - Collateral custody per asset",
                  "Escrow - Per-user pledges with anti-replay nonces",
                  "Cap - Time-limited debit authorization tokens (2min TTL)",
                  "Portfolio - Cross-margin tracking across slabs",
                  "SlabRegistry - Governance-controlled registry"
                ]}
              />
              
              <Card
                title="Slab Program"
                programId="SLabZ6PsDLh2X6HzEoqxFDMqCVcJXDKCNEYuPzUvGPk"
                description="Per-market order book and matching engine in a single 10MB account"
                features={[
                  "SlabState - Header + 5 memory pools (64KB each)",
                  "Order book with price-time priority",
                  "CLOB matching engine",
                  "Risk calculations (IM/MM)",
                  "Anti-toxicity mechanisms (Kill Band, JIT Penalty, ARG)"
                ]}
              />
            </div>
          </Section>

          {/* Completed Features */}
          <Section
            icon={<CheckCircle2 className="w-6 h-6" />}
            title="Completed Features"
            delay={0.6}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <FeatureItem text="Core data structures (Router & Slab)" />
              <FeatureItem text="Memory pools with O(1) freelists" />
              <FeatureItem text="Order book management" />
              <FeatureItem text="Reserve & Commit operations" />
              <FeatureItem text="Risk calculations (IM/MM)" />
              <FeatureItem text="Capability system (scoped debits)" />
              <FeatureItem text="Funding rate system" />
              <FeatureItem text="Instruction handlers" />
              <FeatureItem text="Router orchestration (multi-slab)" />
              <FeatureItem text="Liquidation engine" />
              <FeatureItem text="Account initialization helpers" />
              <FeatureItem text="BPF deployment scripts" />
              <FeatureItem text="TypeScript SDK" />
              <FeatureItem text="CLI Tools" />
              <FeatureItem text="140+ passing tests" />
              <FeatureItem text="Integration tests (Surfpool)" />
            </div>
          </Section>

          {/* Tech Stack */}
          <Section
            icon={<Code2 className="w-6 h-6" />}
            title="Technology Stack"
            delay={0.8}
          >
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <TechCard
                icon={<Package className="w-8 h-8 text-[#B8B8FF]" />}
                title="Pinocchio v0.9.2"
                description="Zero-dependency Solana SDK for on-chain programs"
              />
              <TechCard
                icon={<Terminal className="w-8 h-8 text-[#B8B8FF]" />}
                title="Surfpool"
                description="Local Solana test validator with mainnet state"
              />
              <TechCard
                icon={<Settings className="w-8 h-8 text-[#B8B8FF]" />}
                title="Rust (no_std)"
                description="Zero allocations, panic = abort for BPF"
              />
            </div>
          </Section>

          {/* Key Files & Links */}
          <Section
            icon={<FileCode className="w-6 h-6" />}
            title="Key Files & Documentation"
            delay={1.0}
          >
            <div className="space-y-3">
              <FileLink
                href="https://github.com/HaidarIDK/percolator/blob/master/README.md"
                text="README.md - Complete project documentation"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/blob/master/plan.md"
                text="plan.md - Full protocol specification"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/tree/master/programs/router/src"
                text="programs/router/src/ - Router program source"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/tree/master/programs/slab/src"
                text="programs/slab/src/ - Slab program source"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/tree/master/sdk/typescript"
                text="sdk/typescript/ - TypeScript SDK for frontend"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/tree/master/cli"
                text="cli/ - Command-line tools"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/blob/master/build-bpf.sh"
                text="build-bpf.sh - Build script for Solana BPF"
              />
              <FileLink
                href="https://github.com/HaidarIDK/percolator/blob/master/deploy-devnet.sh"
                text="deploy-devnet.sh - Deployment script"
              />
            </div>
          </Section>

          {/* Design Invariants */}
          <Section
            icon={<Shield className="w-6 h-6" />}
            title="Design Invariants"
            delay={1.2}
          >
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div className="space-y-2">
                <h4 className="text-[#B8B8FF] font-semibold text-lg mb-3">Safety</h4>
                <InvariantItem text="Slabs cannot access Router vaults directly" />
                <InvariantItem text="Slabs can only debit via unexpired Caps" />
                <InvariantItem text="Total debits ≤ min(cap.remaining, escrow.balance)" />
                <InvariantItem text="No cross-contamination between users/slabs" />
              </div>
              
              <div className="space-y-2">
                <h4 className="text-[#B8B8FF] font-semibold text-lg mb-3">Matching</h4>
                <InvariantItem text="Price-time priority strictly maintained" />
                <InvariantItem text="Reserved qty ≤ available qty always" />
                <InvariantItem text="Book links acyclic and consistent" />
                <InvariantItem text="Pending orders never match before promotion" />
              </div>
            </div>
          </Section>

          {/* Anti-Toxicity Mechanisms */}
          <Section
            icon={<Zap className="w-6 h-6" />}
            title="Anti-Toxicity Mechanisms"
            delay={1.4}
          >
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <MechanismCard
                title="Kill Band"
                description="Reject orders if mark price moved beyond threshold since batch open"
              />
              <MechanismCard
                title="JIT Penalty"
                description="DLP orders posted after batch_open get no rebate, discouraging toxic flow"
              />
              <MechanismCard
                title="ARG Tax"
                description="Roundtrip trades within the same batch are taxed/clipped to prevent gaming"
              />
            </div>
          </Section>

          {/* Back Button */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.6 }}
            className="mt-16 text-center"
          >
            <a
              href="/"
              className="inline-flex items-center gap-2 px-8 py-3 rounded-xl bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 text-white hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50 transition-all duration-300 backdrop-blur-sm"
            >
              ← Back to Home
            </a>
          </motion.div>
        </div>
      </main>
    </div>
  )
}

// Helper Components
function Section({ 
  icon, 
  title, 
  delay, 
  children 
}: { 
  icon: React.ReactNode
  title: string
  delay: number
  children: React.ReactNode 
}) {
  return (
    <motion.section
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.8, delay }}
      className="mb-16"
    >
      <div className="flex items-center gap-3 mb-6">
        <div className="p-2 rounded-lg bg-[#B8B8FF]/20 text-[#B8B8FF]">
          {icon}
        </div>
        <h2 className="text-3xl font-bold text-white">{title}</h2>
      </div>
      <div className="pl-0 md:pl-14">
        {children}
      </div>
    </motion.section>
  )
}

function Card({ 
  title, 
  programId, 
  description, 
  features 
}: { 
  title: string
  programId: string
  description: string
  features: string[] 
}) {
  return (
    <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300">
      <h3 className="text-2xl font-bold text-[#B8B8FF] mb-2">{title}</h3>
      <code className="text-xs text-gray-500 font-mono">{programId}</code>
      <p className="text-gray-300 mt-4 mb-4">{description}</p>
      <ul className="space-y-2">
        {features.map((feature, i) => (
          <li key={i} className="flex items-start gap-2 text-gray-400 text-sm">
            <CheckCircle2 className="w-4 h-4 text-[#B8B8FF] mt-0.5 flex-shrink-0" />
            <span>{feature}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}

function FeatureItem({ text }: { text: string }) {
  return (
    <div className="flex items-center gap-2 text-gray-300">
      <CheckCircle2 className="w-5 h-5 text-[#B8B8FF] flex-shrink-0" />
      <span>{text}</span>
    </div>
  )
}

function TechCard({ 
  icon, 
  title, 
  description 
}: { 
  icon: React.ReactNode
  title: string
  description: string 
}) {
  return (
    <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300">
      <div className="mb-4">{icon}</div>
      <h3 className="text-lg font-bold text-white mb-2">{title}</h3>
      <p className="text-gray-400 text-sm">{description}</p>
    </div>
  )
}

function FileLink({ href, text }: { href: string, text: string }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="flex items-center gap-3 px-4 py-3 bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/20 rounded-lg hover:border-[#B8B8FF]/40 hover:bg-[#B8B8FF]/5 transition-all duration-300 group"
    >
      <FileCode className="w-5 h-5 text-[#B8B8FF] flex-shrink-0" />
      <span className="text-gray-300 flex-1 font-mono text-sm">{text}</span>
      <ExternalLink className="w-4 h-4 text-gray-500 group-hover:text-[#B8B8FF] group-hover:translate-x-1 transition-all" />
    </a>
  )
}

function InvariantItem({ text }: { text: string }) {
  return (
    <div className="flex items-start gap-2 text-gray-400 text-sm">
      <div className="w-1.5 h-1.5 rounded-full bg-[#B8B8FF] mt-2 flex-shrink-0" />
      <span>{text}</span>
    </div>
  )
}

function MechanismCard({ 
  title, 
  description 
}: { 
  title: string
  description: string 
}) {
  return (
    <div className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300">
      <h4 className="text-lg font-bold text-[#B8B8FF] mb-3">{title}</h4>
      <p className="text-gray-400 text-sm leading-relaxed">{description}</p>
    </div>
  )
}

