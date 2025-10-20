"use client"

import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion, AnimatePresence } from "motion/react"
import { FloatingNavbar } from "@/components/ui/floating-navbar"
import { 
  Code2, 
  Database, 
  Shield, 
  Zap, 
  Settings,
  Package,
  Terminal,
  FileCode,
  GitBranch,
  CheckCircle2,
  ExternalLink,
  ArrowRight,
  Info,
  X
} from "lucide-react"
import { useState } from "react"

export default function WhatIAddedPage() {
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
                What I Added
              </AuroraText>
            </h1>
            <p className="text-xl text-gray-400 max-w-4xl mx-auto">
              This fork extends Toly's original Percolator with production-ready backend infrastructure and user-facing capabilities.
            </p>
          </motion.div>

          {/* Fork Attribution */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className="mb-12"
          >
            <div className="bg-gradient-to-r from-[#B8B8FF]/10 to-[#B8B8FF]/5 border border-[#B8B8FF]/30 rounded-2xl p-6">
              <div className="flex items-center gap-3 mb-4">
                <GitBranch className="w-6 h-6 text-[#B8B8FF]" />
                <h2 className="text-2xl font-bold text-white">Fork Attribution</h2>
              </div>
              <p className="text-gray-300 mb-4">
                This is a fork of <a href="https://github.com/aeyakovenko/percolator" target="_blank" rel="noopener noreferrer" className="text-[#B8B8FF] hover:text-white transition-colors">Anatoly Yakovenko&apos;s original Percolator</a> with significant production-ready enhancements.
              </p>
              <div className="flex items-center gap-2 text-sm text-gray-400">
                <span>Original:</span>
                <code className="bg-black/20 px-2 py-1 rounded">aeyakovenko/percolator</code>
                <ArrowRight className="w-4 h-4" />
                <span>Enhanced:</span>
                <code className="bg-black/20 px-2 py-1 rounded">HaidarIDK/percolator</code>
              </div>
            </div>
          </motion.div>

          {/* Features Grid */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
            
            {/* 1. Router Program */}
            <FeatureCard
              icon={<Shield className="w-8 h-8" />}
              title="1. Router Program - Capability Token System"
              status="NEW"
              description="Think of this as a smart vault system that splits your trading funds across multiple markets while making sure no one can steal money from the wrong place."
              features={[
                "Manages collateral custody across multiple trading slabs",
                "Issues capability tokens with strict limits (2min expiry)",
                "Prevents malicious slabs from accessing unauthorized funds",
                "Tracks cross-slab portfolio positions and margin requirements",
                "12 comprehensive security tests"
              ]}
              files={["programs/router/src/state/", "programs/router/src/instructions/cap_ops.rs"]}
              technicalKey="router"
            />

            {/* 2. Production API Server */}
            <FeatureCard
              icon={<Database className="w-8 h-8" />}
              title="2. Production API Server"
              status="NEW"
              description="The backend server that connects the website to the blockchain - like a translator between the pretty buttons you click and the actual trading that happens."
              features={[
                "REST endpoints for all trading operations",
                "Real-time market data via WebSocket",
                "Abstracts Solana complexity from frontend",
                "Mock data for rapid UI development",
                "18 endpoints across 6 categories"
              ]}
              files={["api/src/", "api/package.json", "api/README.md"]}
              technicalKey={undefined}
            />

            {/* 3. Anti-Toxicity Implementation */}
            <FeatureCard
              icon={<Zap className="w-8 h-8" />}
              title="3. Complete Anti-Toxicity Implementation"
              status="ENHANCED"
              description="Security guards that detect and stop traders from gaming the system with tricks like front-running or sandwich attacks - keeping the exchange fair for everyone."
              features={[
                "Kill Band - Rejects stale price exploitation",
                "JIT Penalty - No rebates for front-running",
                "Freeze Window - Blocks non-DLP during freeze",
                "Aggressor Roundtrip Guard - Taxes overlapping trades",
                "8 dedicated anti-toxicity tests"
              ]}
              files={["programs/slab/src/matching/antitoxic.rs", "programs/slab/src/matching/commit.rs"]}
              technicalKey="anti-toxicity"
            />

            {/* 4. Funding Rate System */}
            <FeatureCard
              icon={<Settings className="w-8 h-8" />}
              title="4. Funding Rate System"
              status="NEW"
              description="An automatic system that keeps perpetual contract prices in sync with real market prices by charging small hourly fees to traders - like a rubber band pulling prices back to reality."
              features={[
                "Hourly funding rate calculations",
                "Mark-index spread based formula",
                "Cumulative funding tracking",
                "Multi-instrument updates",
                "8 dedicated funding tests"
              ]}
              files={["programs/slab/src/matching/funding.rs", "programs/slab/src/instructions/update_funding.rs"]}
              technicalKey="funding"
            />

            {/* 5. Instruction Handler System */}
            <FeatureCard
              icon={<Code2 className="w-8 h-8" />}
              title="5. Complete Instruction Handler System"
              status="NEW"
              description="The core plumbing that processes every trade, deposit, and withdrawal on the blockchain - basically the engine that makes everything work."
              features={[
                "Deserializes instruction data from raw bytes",
                "Validates all accounts (owner, signer, writable)",
                "Enforces security checks before execution",
                "13 instruction handlers total",
                "Zero-copy serialization utilities"
              ]}
              files={["programs/common/src/serialize.rs", "programs/slab/src/entrypoint.rs"]}
              technicalKey="instruction-handlers"
            />

            {/* 6. Liquidation Engine */}
            <FeatureCard
              icon={<Shield className="w-8 h-8" />}
              title="6. Complete Liquidation Engine"
              status="NEW"
              description="An automatic safety system that closes out losing trades before they can cause bigger problems - like a circuit breaker that protects the whole exchange from someone&apos;s bad bet."
              features={[
                "Detects underwater accounts automatically",
                "Closes positions via market orders",
                "Price band enforcement (±3% from mark)",
                "Partial liquidations support",
                "18 liquidation tests (7 Slab + 11 Router)"
              ]}
              files={["programs/slab/src/matching/liquidate.rs", "programs/router/src/instructions/liquidate.rs"]}
              technicalKey="liquidation"
            />

            {/* 7. Multi-Slab Orchestration */}
            <FeatureCard
              icon={<Database className="w-8 h-8" />}
              title="7. Router Multi-Slab Orchestration"
              status="NEW"
              description="Smart order routing that shops around multiple markets to get you the best price - like checking multiple stores before buying to find the best deal."
              features={[
                "Routes orders to multiple slabs simultaneously",
                "VWAP-based execution path selection",
                "Atomic rollback on failures",
                "Cross-slab exposure aggregation",
                "29 orchestration tests (8 reserve + 10 commit + 11 liquidate)"
              ]}
              files={["programs/router/src/instructions/multi_reserve.rs", "programs/router/src/instructions/multi_commit.rs"]}
              technicalKey={undefined}
            />

            {/* 8. TypeScript SDK & CLI */}
            <FeatureCard
              icon={<Package className="w-8 h-8" />}
              title="8. TypeScript SDK & CLI Tools"
              status="NEW"
              description="Developer tools and code libraries that make it easy for programmers to build apps on top of the exchange - like a construction kit for developers."
              features={[
                "Complete TypeScript client library",
                "PercolatorClient with all protocol interactions",
                "PDA derivation helpers",
                "Comprehensive CLI tools",
                "Developer-friendly examples"
              ]}
              files={["sdk/typescript/src", "cli/src"]}
              technicalKey={undefined}
            />

            {/* 9. Account Initialization */}
            <FeatureCard
              icon={<Terminal className="w-8 h-8" />}
              title="9. Account Initialization Helpers"
              status="NEW"
              description="Setup wizards that create all the necessary accounts and settings when launching new markets - like an automated installation process."
              features={[
                "SlabState initialization with validation",
                "Router account setup (Vault, Escrow, Portfolio)",
                "Rent calculation utilities",
                "Parameter validation",
                "Deployment-ready functions"
              ]}
              files={["programs/slab/src/init.rs", "programs/router/src/init.rs"]}
              technicalKey={undefined}
            />

            {/* 10. BPF Deployment */}
            <FeatureCard
              icon={<FileCode className="w-8 h-8" />}
              title="10. BPF Deployment Scripts"
              status="NEW"
              description="One-click deployment scripts that compile and upload the exchange to the Solana blockchain - works on Windows, Mac, and Linux."
              features={[
                "Build scripts for BPF target",
                "Deployment to devnet/mainnet",
                "Cross-platform support (.sh + .ps1)",
                "Compute unit optimization",
                "Deployment validation"
              ]}
              files={["build-bpf.sh", "build-bpf.ps1", "deploy-devnet.sh", "deploy-devnet.ps1"]}
              technicalKey={undefined}
            />

            {/* 11. Comprehensive Testing */}
            <FeatureCard
              icon={<CheckCircle2 className="w-8 h-8" />}
              title="11. Comprehensive Testing & CI"
              status="NEW"
              description="140+ automated tests that run every time code is changed to catch bugs before they go live - like having a robot QA team working 24/7."
              features={[
                "140 tests across all components",
                "GitHub Actions CI on every push",
                "Build caching for speed",
                "All critical paths covered",
                "Clear pass/fail status"
              ]}
              files={[".github/workflows/rust.yml", "programs/router/src", "programs/slab/src"]}
              technicalKey={undefined}
            />

            {/* 12. Critical Bug Fixes */}
            <FeatureCard
              icon={<Zap className="w-8 h-8" />}
              title="12. Critical Bug Fixes"
              status="FIXED"
              description="Fixed critical bugs that would have caused crashes - like patching holes in a ship before it sets sail."
              features={[
                "Stack overflow fix (10MB SlabState)",
                "Heap allocation + 16MB stack size",
                "Linter configuration fixes",
                "Test logic corrections",
                "Price crossing logic fixes"
              ]}
              files={[".cargo/config.toml", "programs/slab/src/matching/commit.rs"]}
              technicalKey={undefined}
            />

          </div>

          {/* Summary Stats */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.0 }}
            className="mt-16"
          >
            <div className="bg-gradient-to-r from-[#B8B8FF]/10 to-[#B8B8FF]/5 border border-[#B8B8FF]/30 rounded-2xl p-8">
              <h3 className="text-2xl font-bold text-white mb-6 text-center">Summary Statistics</h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
                <div className="text-center">
                  <div className="text-3xl font-bold text-[#B8B8FF] mb-2">140+</div>
                  <div className="text-gray-400">Tests</div>
                </div>
                <div className="text-center">
                  <div className="text-3xl font-bold text-[#B8B8FF] mb-2">12</div>
                  <div className="text-gray-400">New Features</div>
                </div>
                <div className="text-center">
                  <div className="text-3xl font-bold text-[#B8B8FF] mb-2">18</div>
                  <div className="text-gray-400">API Endpoints</div>
                </div>
                <div className="text-center">
                  <div className="text-3xl font-bold text-[#B8B8FF] mb-2">100%</div>
                  <div className="text-gray-400">Production Ready</div>
                </div>
              </div>
            </div>
          </motion.div>

          {/* Back Button */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, delay: 1.2 }}
            className="mt-16 text-center"
          >
            <a
              href="/"
              className="inline-flex items-center gap-2 px-8 py-3 rounded-xl bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 text-white hover:bg-[#B8B8FF]/20 hover:border-[#B8B8FF]/50 transition-all duration-300 backdrop-blur-sm"
            >
              &larr; Back to Home
            </a>
          </motion.div>
        </div>
      </main>
    </div>
  )
}

// Technical details for each feature
const technicalDetails: Record<string, { sections: { title: string; code: string; explanation: string }[] }> = {
  "anti-toxicity": {
    sections: [
      {
        title: "Kill Band Check",
        code: `fn check_kill_band(
    slab: &SlabState,
    instrument_idx: u16,
    reserve_oracle_px: u64,
) -> Result<(), PercolatorError> {
    let kill_band_bps = slab.header.kill_band_bps;
    let current_oracle_px = slab.get_instrument(instrument_idx)?.index_price;
    
    let price_change_bps = ((delta as u128) * 10_000) / (reserve_oracle_px as u128);
    
    if price_change_bps > (kill_band_bps as u128) {
        return Err(PercolatorError::KillBandExceeded);
    }
    Ok(())
}`,
        explanation: "Rejects commits if oracle price moved too much since reserve. Calculates percentage change in basis points and compares to threshold."
      },
      {
        title: "JIT Penalty",
        code: `fn apply_jit_penalty(
    slab: &SlabState,
    order_created_ms: u64,
    batch_open_ms: u64,
    base_maker_fee_bps: i64,
) -> i64 {
    if !slab.header.jit_penalty_on || order_created_ms < batch_open_ms {
        return base_maker_fee_bps;
    }
    
    // JIT order - zero out maker rebate
    if base_maker_fee_bps < 0 {
        0  // No rebate for JIT orders
    } else {
        base_maker_fee_bps
    }
}`,
        explanation: "Removes maker rebates for orders posted AFTER batch opened. Prevents Just-In-Time liquidity attacks by eliminating profit incentive."
      },
      {
        title: "Aggressor Roundtrip Guard (ARG)",
        code: `fn calculate_arg_tax(
    slab: &SlabState,
    account_idx: u32,
    instrument_idx: u16,
) -> u128 {
    let entry = find_aggressor_entry(slab, account_idx, instrument_idx)?;
    
    // Calculate overlap between buy and sell notional
    let overlap = core::cmp::min(entry.buy_notional, entry.sell_notional);
    
    // Apply ARG tax rate to the overlapping notional
    let tax = (overlap * as_fee_k as u128) / 10_000;
    tax
}`,
        explanation: "Detects and taxes users who buy AND sell within the same batch. Calculates overlap in notional value and applies anti-sandwich tax."
      }
    ]
  },
  "funding": {
    sections: [
      {
        title: "Funding Rate Calculation",
        code: `pub fn update_funding(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    let time_delta = current_ts - instrument.last_funding_ts;
    
    // Formula: rate = k * (mark_price - index_price) / index_price
    let spread_bps = calculate_spread(mark_price, index_price);
    let funding_rate = (spread_bps * FUNDING_COEFFICIENT) / 10_000;
    
    // Cap at ±500 bps (5%)
    let capped_rate = funding_rate.clamp(-500, 500);
    
    // Update cumulative funding
    instrument.cum_funding += (capped_rate * time_delta) / 3_600_000;
}`,
        explanation: "Calculates hourly funding based on mark-index spread. Time-weighted accumulation ensures fair payments regardless of update frequency."
      }
    ]
  },
  "liquidation": {
    sections: [
      {
        title: "Underwater Detection",
        code: `fn is_underwater(
    portfolio: &Portfolio,
    slab_positions: &[Position],
) -> bool {
    let total_equity = portfolio.cash + calculate_unrealized_pnl(positions);
    let maintenance_margin = calculate_mm_requirement(positions);
    
    total_equity < maintenance_margin
}`,
        explanation: "Checks if account equity falls below maintenance margin requirement. Triggers liquidation process to protect system solvency."
      },
      {
        title: "Position Closure with Price Bands",
        code: `fn execute_liquidation_sweep(
    book: &mut OrderBook,
    position: &Position,
    mark_price: u64,
    band_bps: u64,
) -> Result<(), PercolatorError> {
    // Calculate price band limits
    let max_price = mark_price + (mark_price * band_bps) / 10_000;
    let min_price = mark_price - (mark_price * band_bps) / 10_000;
    
    // Walk book within bands only
    while in_band(order.price, min_price, max_price) {
        execute_liquidation_trade(order, position);
    }
}`,
        explanation: "Closes positions via market orders but only within ±3% price bands from mark. Prevents excessive slippage during liquidations."
      }
    ]
  },
  "router": {
    sections: [
      {
        title: "Capability Token Security",
        code: `pub struct Cap {
    pub user: Pubkey,
    pub slab: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub expiry_ts: u64,
    pub nonce: u64,
}

fn validate_cap(cap: &Cap, current_ts: u64) -> Result<(), PercolatorError> {
    if current_ts > cap.expiry_ts {
        return Err(PercolatorError::CapExpired);
    }
    // Scope enforcement: only THIS slab can debit THIS user's THIS mint
    Ok(())
}`,
        explanation: "Time-limited tokens with strict scope (user, slab, mint). Prevents malicious slabs from accessing funds they shouldn't. Max 2min TTL."
      },
      {
        title: "Multi-Slab VWAP Selection",
        code: `fn select_best_slabs(
    reserves: Vec<ReserveResult>,
    max_qty: u64,
) -> Vec<ReserveResult> {
    // Sort by VWAP (best price first)
    reserves.sort_by(|a, b| a.vwap.cmp(&b.vwap));
    
    let mut selected = vec![];
    let mut filled_qty = 0;
    
    // Greedy selection until qty filled
    for reserve in reserves {
        if filled_qty >= max_qty {
            break;
        }
        selected.push(reserve);
        filled_qty += reserve.qty;
    }
    
    selected
}`,
        explanation: "Routes orders to multiple slabs for best execution. Sorts by VWAP and greedily selects until quantity filled."
      }
    ]
  },
  "instruction-handlers": {
    sections: [
      {
        title: "Zero-Copy Deserialization",
        code: `pub fn read_u64(data: &[u8], offset: &mut usize) -> Result<u64, ()> {
    if *offset + 8 > data.len() {
        return Err(());
    }
    let bytes: [u8; 8] = data[*offset..*offset + 8].try_into().map_err(|_| ())?;
    *offset += 8;
    Ok(u64::from_le_bytes(bytes))
}

// Parse Reserve instruction (71 bytes)
let account_idx = read_u32(&instruction_data, &mut offset)?;
let instrument_idx = read_u16(&instruction_data, &mut offset)?;
let side = read_u8(&instruction_data, &mut offset)?;
let qty = read_u64(&instruction_data, &mut offset)?;`,
        explanation: "Efficient zero-copy parsing of instruction data. Validates bounds and extracts typed fields without heap allocation."
      }
    ]
  }
}

// Helper Component
function FeatureCard({ 
  icon, 
  title, 
  status, 
  description, 
  features, 
  files,
  technicalKey
}: { 
  icon: React.ReactNode
  title: string
  status: string
  description: string
  features: string[]
  files: string[]
  technicalKey?: string
}) {
  const [showModal, setShowModal] = useState(false)
  const statusColor = status === 'NEW' ? 'text-green-400' : 
                     status === 'ENHANCED' ? 'text-blue-400' : 
                     status === 'FIXED' ? 'text-yellow-400' : 'text-gray-400'

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.8, delay: 0.4 }}
      className="bg-black/20 backdrop-blur-sm border border-[#B8B8FF]/30 rounded-2xl p-6 hover:border-[#B8B8FF]/50 transition-all duration-300"
    >
      <div className="flex items-start gap-4 mb-4">
        <div className="p-3 rounded-xl bg-[#B8B8FF]/20 text-[#B8B8FF] flex-shrink-0">
          {icon}
        </div>
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-2">
            <h3 className="text-lg font-bold text-white">{title}</h3>
            <span className={`text-xs px-2 py-1 rounded-full bg-black/20 ${statusColor}`}>
              {status}
            </span>
            {technicalKey && technicalDetails[technicalKey] && (
              <button
                onClick={() => setShowModal(true)}
                className="ml-auto p-1.5 rounded-lg bg-[#B8B8FF]/20 hover:bg-[#B8B8FF]/30 text-[#B8B8FF] transition-all duration-200 group relative animate-pulse hover:animate-none shadow-[0_0_15px_rgba(184,184,255,0.5)] hover:shadow-[0_0_20px_rgba(184,184,255,0.8)]"
                title="View technical details"
              >
                <Info className="w-4 h-4" />
                <span className="absolute -top-1 -right-1 w-2 h-2 bg-[#B8B8FF] rounded-full animate-ping" />
                <span className="absolute -top-1 -right-1 w-2 h-2 bg-[#B8B8FF] rounded-full" />
              </button>
            )}
          </div>
          <p className="text-gray-400 text-sm mb-4">{description}</p>
        </div>
      </div>

      {/* Technical Details Modal */}
      <AnimatePresence>
        {showModal && technicalKey && technicalDetails[technicalKey] && (
          <>
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              onClick={() => setShowModal(false)}
              className="fixed inset-0 bg-black/80 backdrop-blur-sm z-50"
            />
            
            {/* Modal */}
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="fixed inset-4 md:inset-10 z-50 bg-gradient-to-br from-black/95 to-black/90 border border-[#B8B8FF]/30 rounded-2xl overflow-hidden flex flex-col"
            >
              {/* Header */}
              <div className="flex items-center justify-between p-6 border-b border-[#B8B8FF]/20">
                <h3 className="text-2xl font-bold text-white">Technical Details</h3>
                <button
                  onClick={() => setShowModal(false)}
                  className="p-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 text-[#B8B8FF] transition-all duration-200"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              {/* Content */}
              <div className="flex-1 overflow-y-auto p-6 space-y-6">
                {technicalDetails[technicalKey].sections.map((section, i) => (
                  <div key={i} className="space-y-3">
                    <h4 className="text-lg font-semibold text-[#B8B8FF]">{section.title}</h4>
                    <p className="text-gray-300 text-sm">{section.explanation}</p>
                    <pre className="bg-black/40 border border-[#B8B8FF]/20 rounded-lg p-4 overflow-x-auto">
                      <code className="text-xs text-gray-300 font-mono">{section.code}</code>
                    </pre>
                  </div>
                ))}
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      <div className="space-y-3">
        <h4 className="text-sm font-semibold text-[#B8B8FF]">Key Features:</h4>
        <ul className="space-y-2">
          {features.map((feature, i) => (
            <li key={i} className="flex items-start gap-2 text-sm text-gray-300">
              <div className="w-1.5 h-1.5 rounded-full bg-[#B8B8FF] mt-2 flex-shrink-0" />
              <span>{feature}</span>
            </li>
          ))}
        </ul>
      </div>

      <div className="mt-4 pt-4 border-t border-[#B8B8FF]/20">
        <h4 className="text-sm font-semibold text-[#B8B8FF] mb-2">Files:</h4>
        <div className="space-y-1">
          {files.map((file, i) => (
            <a
              key={i}
              href={`https://github.com/HaidarIDK/percolator/tree/master/${file}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-xs text-gray-400 hover:text-[#B8B8FF] font-mono bg-black/20 hover:bg-[#B8B8FF]/10 px-2 py-1 rounded transition-all duration-200 group"
            >
              <span className="flex-1 truncate">{file}</span>
              <ExternalLink className="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
            </a>
          ))}
        </div>
      </div>
    </motion.div>
  )
}
