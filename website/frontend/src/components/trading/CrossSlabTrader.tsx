"use client"

import { useState, useEffect } from "react"
import { motion } from "motion/react"
import { Zap, Shield, Target, DollarSign } from "lucide-react"
import { cn } from "@/lib/utils"
import { useWallet, useConnection } from "@solana/wallet-adapter-react"
import { Transaction } from "@solana/web3.js"
import { Buffer } from "buffer"

interface CrossSlabTraderProps {
  selectedCoin: "ethereum" | "bitcoin" | "solana"
}

export const CrossSlabTrader = ({ selectedCoin }: CrossSlabTraderProps) => {
  const wallet = useWallet();
  const { publicKey, connected, signTransaction } = wallet;
  const { connection } = useConnection();
  
  const [tradeSide, setTradeSide] = useState<"buy" | "sell">("buy");
  const [quantity, setQuantity] = useState("");
  const [limitPrice, setLimitPrice] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [executionPlan, setExecutionPlan] = useState<{ 
    slabs: Array<{ 
      slabId: string; 
      fillAmount: number; 
      price: number; 
      slabName?: string; 
      quantity?: number; 
      cost?: number;
    }>; 
    totalCost: number; 
    totalFees: number; 
    estimatedSlippage: number; 
    totalQuantity?: number; 
    avgPrice?: number; 
    unfilled?: number;
  } | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(true);
  const [showCrossSlabInfo, setShowCrossSlabInfo] = useState(false);
  const [deploymentVersion, setDeploymentVersion] = useState<"v0" | "v1">("v0");

  const [availableSlabs, setAvailableSlabs] = useState<any[]>([]);
  const [loadingSlabs, setLoadingSlabs] = useState(false);

  useEffect(() => {
    const fetchSlabs = async () => {
      setLoadingSlabs(true);
      try {
        const getMarketPrice = () => {
          switch(selectedCoin) {
            case "ethereum": return 3882;
            case "bitcoin": return 97500;
            case "solana": return 185;
            default: return 3882;
          }
        };

        const basePrice = getMarketPrice();
        
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
        const response = await fetch(`${API_URL}/api/router/slabs?coin=${selectedCoin}`);
        const data = await response.json();
        
        const updatedSlabs = (data.slabs || []).map((slab: { id: string; name: string; buyPrice: number; sellPrice: number }) => ({
          ...slab,
          vwap: basePrice * (1 + (slab.id === '1' ? 0.00005 : slab.id === '2' ? 0.00008 : -0.00005)),
        }));
        
        setAvailableSlabs(updatedSlabs);
      } catch (error) {
        const getMarketPrice = () => {
          switch(selectedCoin) {
            case "ethereum": return 3882;
            case "bitcoin": return 97500;
            case "solana": return 185;
            default: return 3882;
          }
        };
        
        const basePrice = getMarketPrice();
        
        setAvailableSlabs([
          { id: 1, name: "Slab A", liquidity: 1500, vwap: basePrice * 1.00005, fee: 0.02 },
          { id: 2, name: "Slab B", liquidity: 2300, vwap: basePrice * 1.00008, fee: 0.015 },
          { id: 3, name: "Slab C", liquidity: 980, vwap: basePrice * 0.99995, fee: 0.025 },
        ]);
      } finally {
        setLoadingSlabs(false);
      }
    };
    fetchSlabs();
  }, [selectedCoin]);

  const getBaseCurrency = () => {
    switch(selectedCoin) {
      case "ethereum": return "ETH";
      case "bitcoin": return "BTC";
      case "solana": return "SOL";
    }
  };

  const getQuoteCurrency = () => "USDC";

  useEffect(() => {
    if (!quantity || !limitPrice || availableSlabs.length === 0) {
      setExecutionPlan(null);
      return;
    }

    const qty = parseFloat(quantity);
    const limit = parseFloat(limitPrice);

    if (isNaN(qty) || isNaN(limit) || qty <= 0 || limit <= 0) {
      setExecutionPlan(null);
      return;
    }

    const sorted = [...availableSlabs].sort((a, b) => 
      tradeSide === "buy" ? a.vwap - b.vwap : b.vwap - a.vwap
    );

    let remaining = qty;
    const plan: Array<{ 
      slabId: string; 
      fillAmount: number; 
      price: number; 
      slabName?: string; 
      quantity?: number; 
      cost?: number;
    }> = [];
    let totalCost = 0;
    let totalFees = 0;

    for (const slab of sorted) {
      if (remaining <= 0.001) break;
      
      const withinLimit = tradeSide === "buy" 
        ? slab.vwap <= limit 
        : slab.vwap >= limit;
      
      if (!withinLimit) continue;

      const qtyFromSlab = Math.min(remaining, slab.liquidity);
      const cost = qtyFromSlab * slab.vwap;
      const fee = cost * ((slab.fee || 2) / 100);
      
      plan.push({
        slabId: slab.id,
        fillAmount: qtyFromSlab,
        price: slab.vwap,
        slabName: slab.name,
        quantity: qtyFromSlab,
        cost: cost
      });

      remaining -= qtyFromSlab;
      totalCost += cost;
      totalFees += fee;
    }

    const filledQty = qty - remaining;
    const avgPrice = filledQty > 0 ? totalCost / filledQty : 0;
    const totalWithFees = totalCost + totalFees;
    const estimatedSlippage = 0;

    setExecutionPlan({
      slabs: plan,
      totalQuantity: filledQty,
      totalCost: totalWithFees,
      totalFees: totalFees,
      estimatedSlippage: estimatedSlippage,
      avgPrice: avgPrice,
      unfilled: Math.max(0, remaining)
    });

  }, [quantity, limitPrice, tradeSide, selectedCoin, availableSlabs]);

  const handleExecuteCrossSlab = async () => {
    if (!connected || !publicKey || !signTransaction) {
      alert('Connect wallet first');
      return;
    }

    if (!executionPlan || (executionPlan.unfilled && executionPlan.unfilled > 0)) {
      alert('Not enough liquidity across slabs');
      return;
    }

    setSubmitting(true);
    try {
      const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
      const sdkResponse = await fetch(`${API_URL}/api/router/execute-cross-slab`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          wallet: publicKey.toBase58(),
          slabs: executionPlan.slabs.map((s: { slabId: string; fillAmount: number; price: number }) => ({
            slabId: s.slabId,
            quantity: s.fillAmount,
            price: s.price,
          })),
          side: tradeSide,
          instrumentIdx: 0,
          totalQuantity: executionPlan.totalQuantity,
          limitPrice: parseFloat(limitPrice),
        })
      });

      const sdkResult = await sdkResponse.json();
      
      if (!sdkResult.success || !sdkResult.transaction) {
        alert(sdkResult.error || 'Failed to build cross-slab transaction');
        setSubmitting(false);
        return;
      }
      
      const transaction = Transaction.from(
        Buffer.from(sdkResult.transaction, 'base64')
      );
      
      const signed = await signTransaction(transaction);
      const signature = await connection.sendRawTransaction(signed.serialize());
      await connection.confirmTransaction(signature, 'confirmed');
      
      alert(`✅ Cross-Slab Execution Complete!\n\nFilled: ${executionPlan.totalQuantity} ${getBaseCurrency()}\nAvg Price: ${executionPlan.avgPrice?.toFixed(2) || 'N/A'} ${getQuoteCurrency()}\nSlabs used: ${executionPlan.slabs.length}\n\nSignature: ${signature.substring(0, 20)}...`);
      
      setQuantity("");
      setLimitPrice("");
      setExecutionPlan(null);
      
    } catch (error: unknown) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      if (errorMessage.includes('User rejected')) {
        alert('Transaction cancelled');
      } else if (errorMessage.includes('simulation failed')) {
        alert('⚠️ Transaction simulation failed (normal on testnet)\n\nThe flow works but programs need initialization.');
      } else {
        alert(`Execution failed: ${errorMessage || 'Unknown error'}`);
      }
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <>
      {showCrossSlabInfo && (
        <div className="fixed inset-0 bg-black/90 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <motion.div 
            initial={{ scale: 0.9, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            className="bg-[#0a0a0f] border border-purple-500/30 rounded-2xl max-w-5xl w-full max-h-[90vh] overflow-hidden shadow-2xl"
          >
            <div className="px-6 py-4 border-b border-[#181825] flex items-center justify-between bg-gradient-to-r from-purple-500/10 to-pink-500/10">
              <h3 className="text-white font-bold text-xl flex items-center gap-3">
                <Zap className="w-6 h-6 text-purple-400" />
                Cross-Slab Router - How It Works
              </h3>
              <button 
                onClick={() => setShowCrossSlabInfo(false)}
                className="text-gray-400 hover:text-white transition-colors"
              >
                <span className="text-3xl">×</span>
              </button>
            </div>

            <div className="px-6 py-6 max-h-[75vh] overflow-y-auto space-y-6">
              <motion.div 
                initial={{ x: -20, opacity: 0 }}
                animate={{ x: 0, opacity: 1 }}
                transition={{ delay: 0.1 }}
                className="bg-gradient-to-r from-blue-500/10 to-cyan-500/10 rounded-xl p-5 border border-blue-500/30"
              >
                <h4 className="text-cyan-300 font-bold text-lg mb-4 flex items-center gap-2">
                  <div className="w-8 h-8 rounded-full bg-cyan-500/20 flex items-center justify-center border border-cyan-500/40">
                    <span className="text-cyan-300 font-bold">1</span>
                  </div>
                  Phase 1: Order Submission
                </h4>
                <div className="space-y-3">
                  <div className="bg-black/40 rounded-lg p-4">
                    <pre className="text-sm text-gray-300 font-mono leading-relaxed">
{`You: "Buy 500 ETH @ max $3,885 each"
         ↓
Frontend: Build ExecuteCrossSlab instruction
         ↓
SDK: Validate portfolio & margin requirements
         ↓
✅ Order accepted for routing`}
                    </pre>
                  </div>
                  <div className="text-sm text-gray-300">
                    The router checks your <span className="text-cyan-400 font-semibold">portfolio health</span> and ensures you have enough <span className="text-cyan-400 font-semibold">free collateral</span> before proceeding.
                  </div>
                </div>
              </motion.div>

              <motion.div 
                initial={{ x: -20, opacity: 0 }}
                animate={{ x: 0, opacity: 1 }}
                transition={{ delay: 0.2 }}
                className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 rounded-xl p-5 border border-purple-500/30"
              >
                <h4 className="text-purple-300 font-bold text-lg mb-4 flex items-center gap-2">
                  <div className="w-8 h-8 rounded-full bg-purple-500/20 flex items-center justify-center border border-purple-500/40">
                    <span className="text-purple-300 font-bold">2</span>
                  </div>
                  Phase 2: Quote Aggregation
                </h4>
                <div className="space-y-3">
                  <div className="bg-black/40 rounded-lg p-4">
                    <pre className="text-sm text-gray-300 font-mono leading-relaxed">
{`Router reads QuoteCache from multiple slabs:

Slab A: 1500 ETH @ $3,881.95  (0.02% fee)
Slab B: 2300 ETH @ $3,882.15  (0.015% fee)
Slab C:  980 ETH @ $3,881.75  (0.025% fee)
         ↓
Sort by VWAP (best price first):
1. Slab C: $3,881.75 ← BEST!
2. Slab A: $3,881.95
3. Slab B: $3,882.15

Optimal split calculation:
• Take 500 ETH from Slab C
• Total cost: $1,940,875
• Avg price: $3,881.75
✅ Fully filled within limit!`}
                    </pre>
                  </div>
                  <div className="grid grid-cols-3 gap-2 text-xs">
                    <div className="bg-purple-500/10 rounded-lg p-3 border border-purple-500/20 text-center">
                      <div className="text-gray-400">Available Liquidity</div>
                      <div className="text-white font-bold text-lg mt-1">4780 ETH</div>
                    </div>
                    <div className="bg-purple-500/10 rounded-lg p-3 border border-purple-500/20 text-center">
                      <div className="text-gray-400">Price Levels</div>
                      <div className="text-white font-bold text-lg mt-1">3 Slabs</div>
                    </div>
                    <div className="bg-purple-500/10 rounded-lg p-3 border border-purple-500/20 text-center">
                      <div className="text-gray-400">Best VWAP</div>
                      <div className="text-green-400 font-bold text-lg mt-1">$3,881.75</div>
                    </div>
                  </div>
                </div>
              </motion.div>

              <motion.div 
                initial={{ x: -20, opacity: 0 }}
                animate={{ x: 0, opacity: 1 }}
                transition={{ delay: 0.3 }}
                className="bg-gradient-to-r from-green-500/10 to-emerald-500/10 rounded-xl p-5 border border-green-500/30"
              >
                <h4 className="text-green-300 font-bold text-lg mb-4 flex items-center gap-2">
                  <div className="w-8 h-8 rounded-full bg-green-500/20 flex items-center justify-center border border-green-500/40">
                    <span className="text-green-300 font-bold">3</span>
                  </div>
                  Phase 3: Atomic Execution
                </h4>
                <div className="space-y-3">
                  <div className="bg-black/40 rounded-lg p-4">
                    <pre className="text-sm text-gray-300 font-mono leading-relaxed">
{`Router CPIs to each slab's commit_fill:

┌────────────┐
│  Router    │ Execute on Slab C (500 ETH)
└─────┬──────┘
      │ CPI
      ↓
┌────────────┐
│  Slab C    │ commit_fill(500 ETH)
└─────┬──────┘
      │
      ├──→ Match against orderbook
      ├──→ Execute fills
      └──→ Return FillReceipt
             ↓
      ✅ Receipt: {
           filled: 500 ETH,
           avgPrice: 3881.75,
           fees: 0.025%
         }

ALL-OR-NOTHING ATOMICITY:
✅ If Slab C succeeds → Commit
❌ If Slab C fails → ROLLBACK entire tx
   No partial fills!`}
                    </pre>
                  </div>
                  <div className="bg-green-500/10 rounded-lg p-3 border border-green-500/20">
                    <div className="text-sm text-green-300 font-semibold mb-2">🛡️ Atomic Guarantee</div>
                    <div className="text-xs text-gray-300">
                      Either <span className="text-white font-semibold">ALL slabs execute</span> or <span className="text-white font-semibold">NONE execute</span>. 
                      No partial fills, no stuck capital. If any slab fails, the entire transaction reverts.
                    </div>
                  </div>
                </div>
              </motion.div>

              <motion.div 
                initial={{ x: -20, opacity: 0 }}
                animate={{ x: 0, opacity: 1 }}
                transition={{ delay: 0.4 }}
                className="bg-gradient-to-r from-orange-500/10 to-yellow-500/10 rounded-xl p-5 border border-orange-500/30"
              >
                <h4 className="text-orange-300 font-bold text-lg mb-4 flex items-center gap-2">
                  <div className="w-8 h-8 rounded-full bg-orange-500/20 flex items-center justify-center border border-orange-500/40">
                    <span className="text-orange-300 font-bold">4</span>
                  </div>
                  Phase 4: Portfolio Update
                </h4>
                <div className="space-y-3">
                  <div className="bg-black/40 rounded-lg p-4">
                    <pre className="text-sm text-gray-300 font-mono leading-relaxed">
{`Router aggregates FillReceipts:

Receipt from Slab C:
  + 500 ETH position
  - $1,940,875 USDC

Portfolio NET exposure calculation:
  Old: 0 ETH position
  New: +500 ETH position
  
Margin calculation on NET exposure:
  Initial Margin (IM):  $388,175 (20%)
  Maintenance Margin:   $194,087 (10%)
  Free Collateral:      Updated

✅ Portfolio updated with net positions!`}
                    </pre>
                  </div>
                  <div className="bg-orange-500/10 rounded-lg p-3 border border-orange-500/20">
                    <div className="text-sm text-orange-300 font-semibold mb-2">💡 Capital Efficiency</div>
                    <div className="text-xs text-gray-300">
                      Margin is calculated on your <span className="text-white font-semibold">NET exposure</span> across all slabs, 
                      not per-slab. This means offsetting positions reduce margin requirements!
                    </div>
                  </div>
                </div>
              </motion.div>

              <motion.div 
                initial={{ y: 20, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                transition={{ delay: 0.5 }}
                className="bg-[#181825] rounded-xl p-6 border border-purple-500/20"
              >
                <h4 className="text-white font-bold text-lg mb-4 text-center">Complete Transaction Flow</h4>
                <pre className="text-xs text-gray-300 font-mono leading-relaxed overflow-x-auto">
{`┌─────────────────────────────────────────────────────────────────────┐
│                         USER FRONTEND                               │
│  "Buy 500 ETH @ $3,885 max"                                        │
└────────────┬────────────────────────────────────────────────────────┘
             │
             │ 1. Submit Order
             ↓
┌─────────────────────────────────────────────────────────────────────┐
│                      CROSS-SLAB ROUTER                              │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  QUOTE AGGREGATION ENGINE                                    │  │
│  │  • Read QuoteCache from Slab A, B, C                        │  │
│  │  • Calculate VWAP for each slab                             │  │
│  │  • Sort by best price                                        │  │
│  │  • Optimize split: Slab C (500 ETH @ $3,881.75)            │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  ATOMIC EXECUTION ENGINE                                     │  │
│  │  • CPI to Slab C: reserve(500 ETH)                          │  │
│  │  • Receive hold_id from Slab C                              │  │
│  │  • CPI to Slab C: commit_fill(hold_id)                      │  │
│  │  • Receive FillReceipt from Slab C                          │  │
│  │  • IF SUCCESS → Update Portfolio                            │  │
│  │  • IF FAIL → ROLLBACK entire transaction                    │  │
│  └──────────────────────────────────────────────────────────────┘  │
└────────────┬────────────────────────────────────────────────────────┘
             │
             │ 2. CPI Calls
             ↓
┌─────────────────────────────────────────────────────────────────────┐
│                          SLAB C (Selected)                          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌─────────────────┐  │
│  │   ORDERBOOK      │  │   MATCHING       │  │  EXECUTION      │  │
│  │                  │  │   ENGINE         │  │                 │  │
│  │  Bids:           │  │                  │  │  Fill: 500 ETH  │  │
│  │  $3881.80: 200   │→ │  Match best      │→ │  @ $3,881.75   │  │
│  │  $3881.75: 800   │  │  prices          │  │  Total: $1.94M  │  │
│  │  $3881.70: 500   │  │                  │  │  ✅ Success     │  │
│  └──────────────────┘  └──────────────────┘  └─────────────────┘  │
└────────────┬────────────────────────────────────────────────────────┘
             │
             │ 3. Return FillReceipt
             ↓
┌─────────────────────────────────────────────────────────────────────┐
│                      PORTFOLIO ACCOUNT                              │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  NET POSITIONS (across all slabs):                           │  │
│  │                                                               │  │
│  │  ETH:  0 → +500 (+500)                                       │  │
│  │  USDC: $2M → $60K (-$1.94M)                                  │  │
│  │                                                               │  │
│  │  MARGIN (calculated on NET exposure):                        │  │
│  │  IM (20%):  $388,175                                         │  │
│  │  MM (10%):  $194,087                                         │  │
│  │  Free:      $2M - $388K = $1.61M ✅                          │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘`}
                </pre>
              </motion.div>

              <motion.div 
                initial={{ y: 20, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                transition={{ delay: 0.6 }}
                className="grid grid-cols-2 gap-4"
              >
                <div className="bg-gradient-to-br from-green-500/10 to-emerald-500/10 rounded-xl p-4 border border-green-500/20">
                  <div className="text-green-300 font-bold mb-2 flex items-center gap-2">
                    <Shield className="w-4 h-4" />
                    Atomic Execution
                  </div>
                  <div className="text-xs text-gray-300 space-y-1">
                    <div>✅ All slabs execute together</div>
                    <div>✅ Or none execute at all</div>
                    <div>✅ No partial fills</div>
                    <div>✅ No stuck capital</div>
                  </div>
                </div>

                <div className="bg-gradient-to-br from-blue-500/10 to-cyan-500/10 rounded-xl p-4 border border-blue-500/20">
                  <div className="text-cyan-300 font-bold mb-2 flex items-center gap-2">
                    <Target className="w-4 h-4" />
                    Best Execution
                  </div>
                  <div className="text-xs text-gray-300 space-y-1">
                    <div>✅ Auto-finds best prices</div>
                    <div>✅ Aggregates liquidity</div>
                    <div>✅ Minimizes slippage</div>
                    <div>✅ Optimizes fees</div>
                  </div>
                </div>

                <div className="bg-gradient-to-br from-purple-500/10 to-pink-500/10 rounded-xl p-4 border border-purple-500/20">
                  <div className="text-purple-300 font-bold mb-2 flex items-center gap-2">
                    <DollarSign className="w-4 h-4" />
                    Capital Efficiency
                  </div>
                  <div className="text-xs text-gray-300 space-y-1">
                    <div>✅ Margin on NET exposure</div>
                    <div>✅ Not per-slab margin</div>
                    <div>✅ Offsetting positions reduce margin</div>
                    <div>✅ Use capital across all slabs</div>
                  </div>
                </div>

                <div className="bg-gradient-to-br from-orange-500/10 to-yellow-500/10 rounded-xl p-4 border border-orange-500/20">
                  <div className="text-orange-300 font-bold mb-2 flex items-center gap-2">
                    <Zap className="w-4 h-4" />
                    Fast & Secure
                  </div>
                  <div className="text-xs text-gray-300 space-y-1">
                    <div>✅ One transaction, one approval</div>
                    <div>✅ Phantom wallet signs</div>
                    <div>✅ ~7-10 seconds total</div>
                    <div>✅ On-chain settlement</div>
                  </div>
                </div>
              </motion.div>

              <motion.div 
                initial={{ y: 20, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                transition={{ delay: 0.7 }}
                className="bg-gradient-to-r from-cyan-500/10 to-blue-500/10 rounded-xl p-5 border border-cyan-500/30"
              >
                <h4 className="text-cyan-300 font-bold text-lg mb-4">📊 Real Example</h4>
                <div className="bg-black/40 rounded-lg p-4 space-y-3">
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400 mb-1">Your Order</div>
                      <div className="text-white">Buy 500 ETH @ $3,885 limit</div>
                    </div>
                    <div>
                      <div className="text-gray-400 mb-1">Router Finds</div>
                      <div className="text-green-300">500 ETH @ $3,881.75 avg</div>
                    </div>
                  </div>
                  <div className="border-t border-gray-700 pt-3">
                    <div className="text-xs text-gray-400 mb-2">Execution Breakdown:</div>
                    <div className="space-y-1 text-xs">
                      <div className="flex justify-between">
                        <span className="text-gray-300">Slab C: 500 ETH @ $3,881.75</span>
                        <span className="text-white">$1,940,875</span>
                      </div>
                      <div className="flex justify-between border-t border-gray-700 pt-1 font-semibold">
                        <span className="text-cyan-300">Total Cost</span>
                        <span className="text-white">$1,940,875</span>
                      </div>
                      <div className="flex justify-between text-green-400">
                        <span>You SAVED</span>
                        <span>$1,625 vs limit!</span>
                      </div>
                    </div>
                  </div>
                </div>
              </motion.div>

              <motion.div 
                initial={{ y: 20, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                transition={{ delay: 0.8 }}
                className="bg-gradient-to-r from-pink-500/10 to-rose-500/10 rounded-xl p-5 border border-pink-500/30"
              >
                <h4 className="text-pink-300 font-bold text-lg mb-4">⚡ Speed Comparison</h4>
                <div className="grid grid-cols-2 gap-4">
                  <div className="bg-black/40 rounded-lg p-4">
                    <div className="text-gray-400 text-sm mb-2">Simple Trading</div>
                    <div className="space-y-1 text-xs text-gray-300">
                      <div>1. Reserve (approve)</div>
                      <div>2. Wait for confirmation</div>
                      <div>3. Commit (approve again)</div>
                      <div>4. Wait for confirmation</div>
                      <div className="text-yellow-400 pt-2 border-t border-gray-700">⏱️ ~15 seconds, 2 approvals</div>
                    </div>
                  </div>
                  <div className="bg-gradient-to-br from-purple-500/20 to-pink-500/20 rounded-lg p-4 border border-purple-500/40">
                    <div className="text-purple-300 text-sm mb-2 font-semibold">Cross-Slab Router</div>
                    <div className="space-y-1 text-xs text-gray-300">
                      <div>1. Click Execute</div>
                      <div>2. Approve once</div>
                      <div>3. Atomic execution</div>
                      <div>4. Done!</div>
                      <div className="text-green-400 pt-2 border-t border-purple-500/40 font-semibold">⚡ ~7 seconds, 1 approval!</div>
                    </div>
                  </div>
                </div>
              </motion.div>

            </div>

            <div className="px-6 py-4 bg-[#181825]/30 border-t border-[#181825] flex justify-between items-center">
              <div className="text-xs text-gray-400">
                <Zap className="w-3 h-3 inline mr-1 text-purple-400" />
                Powered by Solana&apos;s atomic CPI
              </div>
              <button
                onClick={() => setShowCrossSlabInfo(false)}
                className="px-6 py-2 rounded-lg font-semibold text-sm bg-gradient-to-r from-purple-500/30 to-pink-500/30 border border-purple-500/50 text-purple-200 hover:from-purple-500/40 hover:to-pink-500/40 transition-all"
              >
                Got it! Let&apos;s trade 🚀
              </button>
            </div>
          </motion.div>
        </div>
      )}

      <div className="w-full h-full bg-black/20 rounded-2xl border border-[#181825] overflow-hidden">
      <div className="h-10 flex items-center justify-between px-3 border-b border-[#181825]">
        <div className="flex items-center gap-2">
          <h3 className="text-white font-medium text-sm">Cross-Slab Router</h3>
          <span className="text-xs px-2 py-0.5 bg-purple-500/20 border border-purple-500/40 rounded text-purple-300 font-semibold">
            ADVANCED
          </span>
          <button
            onClick={() => setShowCrossSlabInfo(true)}
            className="w-5 h-5 flex items-center justify-center bg-purple-500/20 hover:bg-purple-500/30 border border-purple-500/40 rounded transition-all"
            title="How Cross-Slab Router works"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-purple-300">
              <circle cx="12" cy="12" r="10"/>
              <path d="M12 16v-4"/>
              <path d="M12 8h.01"/>
            </svg>
          </button>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="text-xs text-gray-400 hover:text-white"
          >
            {showAdvanced ? "Hide" : "Show"} Details
          </button>
        </div>
      </div>
      
      <div className="px-3 py-2 bg-[#0a0a0f] border-b border-[#181825]">
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-400">Deployment:</span>
          <button
            onClick={() => setDeploymentVersion("v0")}
            className={cn(
              "text-xs px-2 py-1 rounded transition-all",
              deploymentVersion === "v0"
                ? "bg-cyan-500/30 border border-cyan-500/50 text-cyan-300 font-semibold"
                : "bg-[#181825] border border-[#181825] text-gray-500 hover:text-gray-300"
            )}
            title="v0: Proof of concept, <$4 rent"
          >
            v0 PoC
          </button>
          <button
            onClick={() => setDeploymentVersion("v1")}
            className={cn(
              "text-xs px-2 py-1 rounded transition-all",
              deploymentVersion === "v1"
                ? "bg-purple-500/30 border border-purple-500/50 text-purple-300 font-semibold"
                : "bg-[#181825] border border-[#181825] text-gray-500 hover:text-gray-300"
            )}
            title="v1: Full production, ~$10k+ rent"
          >
            v1 Production
          </button>
          <div className="ml-auto text-xs">
            {deploymentVersion === "v0" ? (
              <span className="text-cyan-400">💎 Less than $4</span>
            ) : (
              <span className="text-purple-400">🚀 ~$10,000+</span>
            )}
          </div>
        </div>
        {deploymentVersion === "v0" ? (
          <div className="text-xs text-gray-500 mt-1">
            128KB slabs · 50 accounts · 300 orders · Proof of concept
          </div>
        ) : (
          <div className="text-xs text-gray-500 mt-1">
            10MB slabs · 10K accounts · 100K orders · Full production scale
          </div>
        )}
      </div>
      
      <div className="p-3 space-y-3">
        <div className="flex bg-[#0a0a0f] rounded-lg p-1 border border-[#181825] relative overflow-hidden">
          <button
            onClick={() => setTradeSide("buy")}
            className={cn(
              "relative flex-1 py-2 rounded-md text-sm font-bold transition-all duration-300",
              tradeSide === "buy" ? "text-[#B8B8FF]" : "text-gray-500 hover:text-gray-300"
            )}
          >
            {tradeSide === "buy" && (
              <motion.span
                layoutId="cross-slab-side"
                className="absolute inset-0 rounded-md bg-gradient-to-r from-[#B8B8FF]/30 to-[#B8B8FF]/20 border border-[#B8B8FF]/40"
                transition={{ type: "spring", stiffness: 400, damping: 35 }}
              />
            )}
            <span className="relative">BUY</span>
          </button>
          <button
            onClick={() => setTradeSide("sell")}
            className={cn(
              "relative flex-1 py-2 rounded-md text-sm font-bold transition-all duration-300",
              tradeSide === "sell" ? "text-red-300" : "text-gray-500 hover:text-gray-300"
            )}
          >
            {tradeSide === "sell" && (
              <motion.span
                layoutId="cross-slab-side"
                className="absolute inset-0 rounded-md bg-gradient-to-r from-red-500/30 to-red-400/20 border border-red-500/40"
                transition={{ type: "spring", stiffness: 400, damping: 35 }}
              />
            )}
            <span className="relative">SELL</span>
          </button>
        </div>

        <div>
          <label className="block text-sm text-gray-300 mb-2 font-medium">
            Total Quantity ({getBaseCurrency()})
          </label>
          <input
            type="number"
            value={quantity}
            onChange={(e) => setQuantity(e.target.value)}
            className="w-full bg-[#181825] border border-[#181825] focus:border-[#B8B8FF]/50 focus:ring-[#B8B8FF]/20 rounded-xl px-4 py-3 text-white text-base font-medium focus:outline-none transition-all duration-300 hover:border-[#181825]/80"
            placeholder="Enter total amount..."
            step="0.1"
          />
        </div>

        <div>
          <label className="block text-sm text-gray-300 mb-2 font-medium">
            Limit Price ({getQuoteCurrency()})
          </label>
          <input
            type="number"
            value={limitPrice}
            onChange={(e) => setLimitPrice(e.target.value)}
            className="w-full bg-[#181825] border border-[#181825] focus:border-[#B8B8FF]/50 focus:ring-[#B8B8FF]/20 rounded-xl px-4 py-3 text-white text-base font-medium focus:outline-none transition-all duration-300 hover:border-[#181825]/80"
            placeholder={tradeSide === "buy" ? "Max price willing to pay..." : "Min price willing to accept..."}
          />
        </div>

        {showAdvanced && (
          <div className="bg-[#181825] rounded-xl p-3 space-y-2">
            <div className="text-xs text-gray-400 font-semibold mb-2">Available Slabs</div>
            {availableSlabs.map((slab) => (
              <div key={slab.id} className="flex items-center justify-between text-xs bg-black/30 rounded-lg p-2">
                <div>
                  <div className="text-white font-medium">{slab.name}</div>
                  <div className="text-gray-500">Liquidity: {slab.liquidity} {getBaseCurrency()}</div>
                </div>
                <div className="text-right">
                  <div className="text-gray-300">VWAP: ${slab.vwap.toFixed(2)}</div>
                  <div className="text-gray-500">Fee: {slab.fee}%</div>
                </div>
              </div>
            ))}
          </div>
        )}

        {executionPlan && (
          <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 rounded-xl p-4 border border-purple-500/30 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-purple-300 font-semibold">Execution Plan</span>
              <span className="text-xs px-2 py-1 bg-purple-500/30 border border-purple-500/50 rounded text-purple-200">
                {executionPlan.slabs.length} Slabs
              </span>
            </div>

            <div className="space-y-2">
              {executionPlan.slabs.map((slab: { slabId: string; fillAmount: number; price: number; slabName?: string; cost?: number }, idx: number) => (
                <div key={idx} className="bg-black/30 rounded-lg p-2 text-xs">
                  <div className="flex justify-between mb-1">
                    <span className="text-gray-300">{slab.slabName}</span>
                    <span className="text-white font-semibold">{slab.fillAmount} {getBaseCurrency()}</span>
                  </div>
                  <div className="flex justify-between text-gray-500">
                    <span>@ ${slab.price.toFixed(2)}</span>
                    <span>${(slab.cost || 0).toFixed(2)}</span>
                  </div>
                </div>
              ))}
            </div>

            <div className="pt-2 border-t border-purple-500/20 space-y-1">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Total Filled:</span>
                <span className="text-white font-semibold">{executionPlan.totalQuantity} {getBaseCurrency()}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Avg Price:</span>
                <span className="text-white font-semibold">${(executionPlan.avgPrice || 0).toFixed(2)}</span>
              </div>
              <div className="flex justify-between text-base font-bold pt-1 border-t border-purple-500/20">
                <span className={tradeSide === "buy" ? "text-purple-300" : "text-green-300"}>
                  {tradeSide === "buy" ? "Total Cost:" : "Total Revenue:"}
                </span>
                <span className={tradeSide === "buy" ? "text-white" : "text-green-400"}>
                  ${executionPlan.totalCost.toFixed(2)}
                </span>
              </div>
              {(executionPlan.unfilled && executionPlan.unfilled > 0) && (
                <div className="text-xs text-yellow-400 mt-2">
                  ⚠️ {executionPlan.unfilled.toFixed(2)} {getBaseCurrency()} unfilled
                  {(executionPlan.totalQuantity || 0) === 0 && tradeSide === "sell" && (
                    <div className="text-xs text-orange-400 mt-1">
                      💡 Slabs are offering ${availableSlabs[0]?.vwap.toFixed(2) || 0}, but your minimum is ${limitPrice}
                      <br />
                      Lower your limit price to ${(availableSlabs[0]?.vwap * 0.99).toFixed(2)} or less to fill
                    </div>
                  )}
                  {(executionPlan.totalQuantity || 0) === 0 && tradeSide === "buy" && (
                    <div className="text-xs text-orange-400 mt-1">
                      💡 Slabs are asking ${availableSlabs[0]?.vwap.toFixed(2) || 0}, but your max is ${limitPrice}
                      <br />
                      Raise your limit price to ${(availableSlabs[0]?.vwap * 1.01).toFixed(2)} or more to fill
                    </div>
                  )}
                </div>
              )}
              {tradeSide === "sell" && (executionPlan.totalQuantity || 0) > 0 && (
                <div className="text-xs text-green-400/70 mt-2">
                  💰 You will receive ${executionPlan.totalCost.toFixed(2)} USDC
                </div>
              )}
            </div>
          </div>
        )}

        <button
          onClick={handleExecuteCrossSlab}
          disabled={!connected || submitting || !executionPlan || Boolean(executionPlan.unfilled && executionPlan.unfilled > 0)}
          className={cn(
            "w-full py-4 rounded-xl font-bold text-base transition-all duration-300 shadow-xl",
            tradeSide === "buy"
              ? "bg-gradient-to-r from-[#B8B8FF]/40 to-[#B8B8FF]/30 hover:from-[#B8B8FF]/50 hover:to-[#B8B8FF]/40 border border-[#B8B8FF]/60 text-[#B8B8FF] hover:shadow-[#B8B8FF]/30"
              : "bg-gradient-to-r from-red-500/40 to-red-400/30 hover:from-red-500/50 hover:to-red-400/40 border border-red-500/60 text-red-300 hover:shadow-red-500/30",
            (!connected || submitting || !executionPlan || (executionPlan.unfilled && executionPlan.unfilled > 0)) && "opacity-50 cursor-not-allowed"
          )}
        >
          {submitting ? (
            <span className="flex items-center justify-center gap-2">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-current"></div>
              <span>Executing Across Slabs...</span>
            </span>
          ) : !connected ? (
            "Connect Wallet"
          ) : !executionPlan ? (
            "Enter Quantity & Price"
          ) : (executionPlan.unfilled && executionPlan.unfilled > 0) ? (
            "Insufficient Liquidity"
          ) : (
            <span className="flex items-center justify-center gap-2">
              <Zap className="w-5 h-5" />
              <span>Execute Cross-Slab {tradeSide.toUpperCase()}</span>
              {tradeSide === "sell" && " 💰"}
            </span>
          )}
        </button>

        <div className="text-xs text-gray-400 text-center">
          <p>🚀 Advanced routing aggregates liquidity from multiple slabs</p>
          <p className="text-purple-400 mt-1">Best execution · Atomic commits · Capital efficient</p>
        </div>
      </div>
      </div>
    </>
  );
};


