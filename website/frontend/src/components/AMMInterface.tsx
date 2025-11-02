"use client"

import { useState, useEffect } from 'react'
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { cn } from '@/lib/utils'
import { useToast } from './ToastProvider'
import { ArrowDown, Settings, Info, Droplets, PlusCircle, MinusCircle } from 'lucide-react'

type AMMMode = 'swap' | 'addLiquidity' | 'removeLiquidity'

interface AMMInterfaceProps {
  coin: 'SOL' | 'ETH' | 'BTC'
  className?: string
}

export function AMMInterface({ coin, className }: AMMInterfaceProps) {
  const { publicKey, connected } = useWallet()
  const { connection } = useConnection()
  const { success, error, warning, info } = useToast()
  
  const [mode, setMode] = useState<AMMMode>('swap')
  const [amountIn, setAmountIn] = useState('')
  const [amountOut, setAmountOut] = useState('')
  const [slippage, setSlippage] = useState('0.5') // 0.5%
  const [priceImpact, setPriceImpact] = useState(0)
  const [loading, setLoading] = useState(false)
  
  // Mock pool reserves (TODO: Fetch from on-chain AMM)
  const [poolReserveBase, setPoolReserveBase] = useState(1000) // SOL/ETH/BTC
  const [poolReserveQuote, setPoolReserveQuote] = useState(180000) // USDC

  // Calculate swap output using constant product formula: x * y = k
  const calculateSwapOutput = (input: number, reserveIn: number, reserveOut: number) => {
    if (input <= 0 || reserveIn <= 0 || reserveOut <= 0) return 0
    
    const fee = 0.003 // 0.3% fee
    const inputWithFee = input * (1 - fee)
    const numerator = inputWithFee * reserveOut
    const denominator = reserveIn + inputWithFee
    return numerator / denominator
  }

  // Calculate price impact
  const calculatePriceImpact = (input: number, reserveIn: number, reserveOut: number) => {
    if (input <= 0) return 0
    
    const priceBeforeSwap = reserveOut / reserveIn
    const output = calculateSwapOutput(input, reserveIn, reserveOut)
    const priceAfterSwap = (reserveOut - output) / (reserveIn + input)
    
    return ((priceAfterSwap - priceBeforeSwap) / priceBeforeSwap) * 100
  }

  // Update output when input changes
  useEffect(() => {
    if (!amountIn || isNaN(parseFloat(amountIn))) {
      setAmountOut('')
      setPriceImpact(0)
      return
    }

    const input = parseFloat(amountIn)
    const output = calculateSwapOutput(input, poolReserveQuote, poolReserveBase)
    const impact = calculatePriceImpact(input, poolReserveQuote, poolReserveBase)
    
    setAmountOut(output.toFixed(6))
    setPriceImpact(impact)
  }, [amountIn, poolReserveBase, poolReserveQuote])

  const handleSwap = async () => {
    if (!connected || !publicKey) {
      warning('Please connect your wallet')
      return
    }

    if (!amountIn || parseFloat(amountIn) <= 0) {
      error('Please enter a valid amount')
      return
    }

    setLoading(true)

    try {
      // TODO: Integrate with actual AMM program
      await new Promise(resolve => setTimeout(resolve, 2000)) // Simulate transaction
      
      success(`✅ Swapped ${amountIn} USDC for ${amountOut} ${coin}`)
      setAmountIn('')
      setAmountOut('')
    } catch (err: any) {
      console.error('Swap failed:', err)
      error(`Failed to swap: ${err.message}`)
    } finally {
      setLoading(false)
    }
  }

  const handleAddLiquidity = async () => {
    if (!connected || !publicKey) {
      warning('Please connect your wallet')
      return
    }

    info('Add liquidity feature coming soon!')
  }

  const handleRemoveLiquidity = async () => {
    if (!connected || !publicKey) {
      warning('Please connect your wallet')
      return
    }

    info('Remove liquidity feature coming soon!')
  }

  return (
    <div className={cn(
      "w-full bg-gradient-to-br from-black/40 via-black/30 to-black/20 backdrop-blur-xl rounded-2xl border border-[#181825] p-4",
      className
    )}>
      {/* Mode Selector */}
      <div className="flex items-center gap-2 mb-4">
        <button
          onClick={() => setMode('swap')}
          className={cn(
            "flex-1 py-2 px-3 rounded-lg text-sm font-semibold transition-all",
            mode === 'swap'
              ? "bg-gradient-to-r from-[#B8B8FF]/30 to-purple-500/20 text-white border border-[#B8B8FF]/50"
              : "bg-black/30 text-gray-400 hover:text-white border border-transparent"
          )}
        >
          <ArrowDown className="w-4 h-4 inline mr-1" />
          Swap
        </button>
        <button
          onClick={() => setMode('addLiquidity')}
          className={cn(
            "flex-1 py-2 px-3 rounded-lg text-sm font-semibold transition-all",
            mode === 'addLiquidity'
              ? "bg-gradient-to-r from-green-500/30 to-emerald-500/20 text-white border border-green-500/50"
              : "bg-black/30 text-gray-400 hover:text-white border border-transparent"
          )}
        >
          <PlusCircle className="w-4 h-4 inline mr-1" />
          Add
        </button>
        <button
          onClick={() => setMode('removeLiquidity')}
          className={cn(
            "flex-1 py-2 px-3 rounded-lg text-sm font-semibold transition-all",
            mode === 'removeLiquidity'
              ? "bg-gradient-to-r from-red-500/30 to-orange-500/20 text-white border border-red-500/50"
              : "bg-black/30 text-gray-400 hover:text-white border border-transparent"
          )}
        >
          <MinusCircle className="w-4 h-4 inline mr-1" />
          Remove
        </button>
      </div>

      {mode === 'swap' && (
        <>
          {/* Input */}
          <div className="bg-black/40 rounded-xl p-4 mb-2">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-gray-400">You Pay</span>
              <span className="text-xs text-gray-500">Balance: 0.00 USDC</span>
            </div>
            <div className="flex items-center gap-3">
              <input
                type="number"
                value={amountIn}
                onChange={(e) => setAmountIn(e.target.value)}
                placeholder="0.00"
                className="flex-1 bg-transparent text-2xl font-bold text-white outline-none"
              />
              <div className="flex items-center gap-2 px-3 py-2 bg-[#181825] rounded-lg">
                <span className="text-white font-semibold">USDC</span>
              </div>
            </div>
          </div>

          {/* Arrow */}
          <div className="flex justify-center -my-1 relative z-10">
            <div className="bg-[#181825] rounded-lg p-2 border border-[#B8B8FF]/20">
              <ArrowDown className="w-5 h-5 text-[#B8B8FF]" />
            </div>
          </div>

          {/* Output */}
          <div className="bg-black/40 rounded-xl p-4 mb-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-gray-400">You Receive</span>
              <span className="text-xs text-gray-500">Balance: 0.00 {coin}</span>
            </div>
            <div className="flex items-center gap-3">
              <input
                type="text"
                value={amountOut}
                readOnly
                placeholder="0.00"
                className="flex-1 bg-transparent text-2xl font-bold text-white outline-none"
              />
              <div className="flex items-center gap-2 px-3 py-2 bg-[#181825] rounded-lg">
                <span className="text-white font-semibold">{coin}</span>
              </div>
            </div>
          </div>

          {/* Price Impact Warning */}
          {priceImpact > 0 && (
            <div className={cn(
              "flex items-center gap-2 p-3 rounded-lg mb-4 text-sm",
              priceImpact > 5 
                ? "bg-red-500/10 border border-red-500/30 text-red-400"
                : priceImpact > 2
                ? "bg-yellow-500/10 border border-yellow-500/30 text-yellow-400"
                : "bg-blue-500/10 border border-blue-500/30 text-blue-400"
            )}>
              <Info className="w-4 h-4" />
              <span>Price Impact: {priceImpact.toFixed(2)}%</span>
            </div>
          )}

          {/* Pool Info */}
          <div className="bg-black/20 rounded-lg p-3 mb-4 text-xs space-y-1">
            <div className="flex justify-between text-gray-400">
              <span>Pool Reserves:</span>
              <span className="text-white">{poolReserveBase.toFixed(2)} {coin} / {poolReserveQuote.toFixed(0)} USDC</span>
            </div>
            <div className="flex justify-between text-gray-400">
              <span>Fee:</span>
              <span className="text-white">0.3%</span>
            </div>
            <div className="flex justify-between text-gray-400">
              <span>Slippage Tolerance:</span>
              <span className="text-white">{slippage}%</span>
            </div>
          </div>

          {/* Swap Button */}
          <button
            onClick={handleSwap}
            disabled={loading || !connected || !amountIn || parseFloat(amountIn) <= 0}
            className="w-full py-3 px-4 rounded-xl bg-gradient-to-r from-[#B8B8FF] to-purple-500 text-white font-bold text-sm hover:opacity-90 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {loading ? (
              <div className="flex items-center justify-center gap-2">
                <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                <span>Swapping...</span>
              </div>
            ) : !connected ? (
              'Connect Wallet'
            ) : !amountIn || parseFloat(amountIn) <= 0 ? (
              'Enter Amount'
            ) : (
              `Swap ${amountIn} USDC for ${amountOut} ${coin}`
            )}
          </button>
        </>
      )}

      {mode === 'addLiquidity' && (
        <div className="text-center py-12">
          <Droplets className="w-12 h-12 text-[#B8B8FF] mx-auto mb-4" />
          <h3 className="text-white font-semibold mb-2">Add Liquidity</h3>
          <p className="text-gray-400 text-sm mb-4">
            Provide liquidity to earn trading fees
          </p>
          <button
            onClick={handleAddLiquidity}
            className="px-6 py-2 rounded-lg bg-green-500/20 border border-green-500/50 text-green-400 font-semibold hover:bg-green-500/30 transition-all"
          >
            Coming Soon
          </button>
        </div>
      )}

      {mode === 'removeLiquidity' && (
        <div className="text-center py-12">
          <MinusCircle className="w-12 h-12 text-[#B8B8FF] mx-auto mb-4" />
          <h3 className="text-white font-semibold mb-2">Remove Liquidity</h3>
          <p className="text-gray-400 text-sm mb-4">
            Withdraw your liquidity and earned fees
          </p>
          <button
            onClick={handleRemoveLiquidity}
            className="px-6 py-2 rounded-lg bg-red-500/20 border border-red-500/50 text-red-400 font-semibold hover:bg-red-500/30 transition-all"
          >
            Coming Soon
          </button>
        </div>
      )}
    </div>
  )
}

