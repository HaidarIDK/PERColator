"use client"

import Link from "next/link"
import { ArrowLeft } from "lucide-react"

export default function V0Page() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-black via-zinc-950 to-black text-white">
      <div className="max-w-4xl mx-auto px-6 py-12">
        <Link href="/trade">
          <button className="mb-8 p-2 rounded-lg bg-zinc-800 hover:bg-zinc-700 transition-all flex items-center gap-2">
            <ArrowLeft className="w-5 h-5" />
            Back to Trading
          </button>
        </Link>

        <h1 className="text-4xl font-bold mb-6 bg-gradient-to-r from-green-400 to-blue-500 bg-clip-text text-transparent">
          v0.1 Proof of Conceptg
        </h1>

        <div className="space-y-6">
          <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-6">
            <h2 className="text-2xl font-bold mb-4">✅ What&apos;s Working</h2>
            <ul className="space-y-2 text-zinc-300">
              <li>✅ <strong>Slab Program</strong> - Deployed on Devnet with Reserve/Commit/Cancel</li>
              <li>✅ <strong>Router Program</strong> - Initialized with Registry</li>
              <li>✅ <strong>Trading UI</strong> - Full Reserve → Commit workflow</li>
              <li>✅ <strong>Toast Notifications</strong> - Beautiful popup messages</li>
              <li>✅ <strong>Orderbook Display</strong> - Active orders and recent trades</li>
              <li>✅ <strong>Transaction History</strong> - All on-chain transactions</li>
              <li>✅ <strong>Wallet Integration</strong> - Phantom, Solflare support</li>
            </ul>
          </div>

          <div className="bg-yellow-900/20 border border-yellow-700/50 rounded-xl p-6">
            <h2 className="text-2xl font-bold mb-4 text-yellow-400">⚠️ POC Limitations</h2>
            <ul className="space-y-2 text-zinc-300">
              <li>⚠️ <strong>Mock Execution</strong> - Transactions succeed but don&apos;t modify slab state</li>
              <li>⚠️ <strong>No Real Matching</strong> - Orders don&apos;t actually fill against each other</li>
              <li>⚠️ <strong>No Position Tracking</strong> - Balances not updated on-chain</li>
              <li>⚠️ <strong>No Settlement</strong> - No actual token transfers</li>
            </ul>
            <p className="mt-4 text-sm text-zinc-400">
              The v0.1 POC demonstrates the full transaction flow and UI/UX,
              but the actual trading logic will be implemented in v0.2+
            </p>
          </div>

          <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-6">
            <h2 className="text-2xl font-bold mb-4">📋 Deployed Addresses</h2>
            <div className="space-y-3 text-sm font-mono">
              <div>
                <div className="text-zinc-500">Slab Program</div>
                <div className="text-zinc-300">SLAB98WHcToiuUMMX9NQSg5E5iB8CjpK21T4h9ZXiep</div>
              </div>
              <div>
                <div className="text-zinc-500">Slab Account</div>
                <div className="text-zinc-300">5Yd2fL7f1DhmNL3u82ptZ21CUpFJHYs1Fqfg2Qs9CLDB</div>
              </div>
              <div>
                <div className="text-zinc-500">Router Program</div>
                <div className="text-zinc-300">RoutqcxkpVH8jJ2cULG9u6WbdRskQwXkJe8CqZehcyr</div>
              </div>
              <div>
                <div className="text-zinc-500">Router Registry</div>
                <div className="text-zinc-300">DK9uaWYienaQ6XEFBzsGCuKZ8ZapTMjw7Ht3s9HQMdUx</div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-purple-900/20 to-blue-900/20 border border-purple-700/50 rounded-xl p-6">
            <h2 className="text-2xl font-bold mb-4 bg-gradient-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent">
              🚀 v0 vs v1: The Key Differences
            </h2>
            
            <div className="grid md:grid-cols-2 gap-6 mb-6">
              {/* v0 Column */}
              <div className="bg-green-900/10 border border-green-700/30 rounded-lg p-4">
                <h3 className="text-lg font-bold text-green-400 mb-3">v0 (Current - POC)</h3>
                <ul className="space-y-2 text-sm text-zinc-300">
                  <li>💰 <strong>Cost:</strong> ~0.025 SOL (~$5)</li>
                  <li>📦 <strong>Size:</strong> 3,408 bytes (~3.4 KB)</li>
                  <li>⚡ <strong>Purpose:</strong> Test</li>
                  <li>🧪 <strong>Execution:</strong> (logs only)</li>
                  <li>🎯 <strong>Perfect for:</strong> Demos & testing</li>
                  <li>✅ <strong>Reserve/Commit:</strong> Works end-to-end</li>
                  <li>⚠️ <strong>Limitation:</strong> No real matching</li>
                </ul>
              </div>

              {/* v1 Column */}
              <div className="bg-purple-900/10 border border-purple-700/30 rounded-lg p-4">
                <h3 className="text-lg font-bold text-purple-400 mb-3">v1 (Full Production)</h3>
                <ul className="space-y-2 text-sm text-zinc-300">
                  <li>💰 <strong>Cost:</strong> ~73 SOL (~$13,900)</li>
                  <li>📦 <strong>Size:</strong> 10,485,760 bytes (10 MB)</li>
                  <li>⚡ <strong>Purpose:</strong> Production trading</li>
                  <li>🔥 <strong>Execution:</strong> Real matching engine</li>
                  <li>🎯 <strong>Perfect for:</strong> Live markets</li>
                  <li>✅ <strong>Features:</strong> Full orderbook + positions</li>
                  <li>🚀 <strong>Capacity:</strong> 1000s of users & orders</li>
                </ul>
              </div>
            </div>

            <div className="bg-zinc-800/30 rounded-lg p-4 mb-4">
              <h4 className="font-bold text-white mb-2">💡 What v1 Adds:</h4>
              <ul className="grid md:grid-cols-2 gap-2 text-sm text-zinc-400">
                <li>✅ Real price-time orderbook matching</li>
                <li>✅ Position tracking & P&L calculation</li>
                <li>✅ Multi-user account pool (1000+ traders)</li>
                <li>✅ Order pool (10,000+ orders)</li>
                <li>✅ Reservation system with expiry</li>
                <li>✅ Slice-based fill execution</li>
                <li>✅ Cross-margin risk engine</li>
                <li>✅ Funding rate calculations</li>
              </ul>
            </div>

            <div className="bg-yellow-900/20 border border-yellow-700/30 rounded-lg p-4">
              <p className="text-sm text-yellow-200 mb-2">
                <strong>💸 Why 73 SOL for v1?</strong>
              </p>
              <p className="text-xs text-zinc-400">
                v1&apos;s 10MB slab contains: Header (200B) + 1000 user accounts (320KB) + 
                10,000 orders (2.4MB) + 5,000 positions (1.4MB) + 1,000 reservations (480KB) + 
                2,000 slices (512KB) + trade history (800KB) + aggressor tracking (192KB).
                This massive on-chain state costs ~73 SOL rent-exemption (~$13,900 at $190/SOL).
              </p>
            </div>
          </div>

          <div className="bg-blue-900/20 border border-blue-700/50 rounded-xl p-6">
            <h2 className="text-2xl font-bold mb-4 text-blue-400">🎯 Try v0 Now</h2>
            <p className="text-zinc-300 mb-4">
              The v0 POC is fully functional for testing the complete Reserve → Commit workflow!
            </p>
            <Link href="/trade">
              <button className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-bold transition-all">
                Go to Trading →
              </button>
            </Link>
          </div>
        </div>
      </div>
    </div>
  )
}

