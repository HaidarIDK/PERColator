"use client"

import { useState } from 'react'
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { Transaction, SystemProgram, PublicKey } from '@solana/web3.js'
import { useToast } from './ToastProvider'
import { cn } from '@/lib/utils'
import { Lock, Unlock, Droplets, TrendingUp, AlertCircle } from 'lucide-react'
import { 
  ROUTER_PROGRAM_ID, 
  SLAB_PROGRAM_ID,
  derivePortfolioAddress,
  deriveVaultPDA,
  deriveRouterAuthorityPDA,
  getRegistryAddress,
  getSlabForCoin
} from '@/lib/program-config'

type Coin = 'SOL' | 'ETH' | 'BTC'

interface LPPanelProps {
  coin: Coin
  className?: string
}

export function LPPanel({ coin, className }: LPPanelProps) {
  const { publicKey, sendTransaction } = useWallet()
  const { connection } = useConnection()
  const { success, error, warning } = useToast()
  
  const [reserveAmount, setReserveAmount] = useState('')
  const [releaseAmount, setReleaseAmount] = useState('')
  const [loading, setLoading] = useState(false)
  const [activeTab, setActiveTab] = useState<'reserve' | 'release'>('reserve')

  // Handle Reserve (Commit liquidity)
  const handleReserve = async () => {
    if (!publicKey || !sendTransaction) {
      warning('Please connect your wallet')
      return
    }

    const amount = parseFloat(reserveAmount)
    if (!amount || amount <= 0) {
      error('Please enter a valid amount')
      return
    }

    const slabAddress = getSlabForCoin(coin)
    if (!slabAddress) {
      error(`No slab configured for ${coin}`)
      return
    }

    setLoading(true)

    try {
      // Get portfolio address
      const portfolioAddress = await derivePortfolioAddress(publicKey)

      // Get vault PDA
      const [vaultPDA] = deriveVaultPDA()

      // Get router authority PDA
      const [routerAuthorityPDA] = deriveRouterAuthorityPDA()

      // Get registry address
      const registryAddress = await getRegistryAddress()

      // Build RouterReserve instruction
      const instruction = new Transaction()

      // Instruction discriminator for RouterReserve (index 8)
      const discriminator = Buffer.from([8])
      
      // Amount in lamports (scaled by 1e6 for fixed point)
      const amountScaled = Math.floor(amount * 1_000_000)
      const amountBuffer = Buffer.alloc(8)
      const view = new DataView(amountBuffer.buffer)
      view.setBigInt64(0, BigInt(amountScaled), true)

      // Venue index (0 for now - you may need to adjust)
      const venueIndex = Buffer.from([0])

      const instructionData = Buffer.concat([
        discriminator,
        amountBuffer,
        venueIndex
      ])

      instruction.add({
        keys: [
          { pubkey: portfolioAddress, isSigner: false, isWritable: true },
          { pubkey: publicKey, isSigner: true, isWritable: false },
          { pubkey: vaultPDA, isSigner: false, isWritable: true },
          { pubkey: registryAddress, isSigner: false, isWritable: false },
          { pubkey: routerAuthorityPDA, isSigner: false, isWritable: false },
          { pubkey: slabAddress, isSigner: false, isWritable: true },
          { pubkey: SLAB_PROGRAM_ID, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: ROUTER_PROGRAM_ID,
        data: instructionData,
      })

      const signature = await sendTransaction(instruction, connection, {
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      })

      await connection.confirmTransaction(signature, 'confirmed')

      success(`✅ Reserved ${amount} ${coin} liquidity!`)
      setReserveAmount('')

    } catch (err: any) {
      console.error('Reserve failed:', err)
      error(`Failed to reserve: ${err.message}`)
    } finally {
      setLoading(false)
    }
  }

  // Handle Release (Uncommit liquidity)
  const handleRelease = async () => {
    if (!publicKey || !sendTransaction) {
      warning('Please connect your wallet')
      return
    }

    const amount = parseFloat(releaseAmount)
    if (!amount || amount <= 0) {
      error('Please enter a valid amount')
      return
    }

    const slabAddress = getSlabForCoin(coin)
    if (!slabAddress) {
      error(`No slab configured for ${coin}`)
      return
    }

    setLoading(true)

    try {
      // Get portfolio address
      const portfolioAddress = await derivePortfolioAddress(publicKey)

      // Get vault PDA
      const [vaultPDA] = deriveVaultPDA()

      // Get router authority PDA
      const [routerAuthorityPDA] = deriveRouterAuthorityPDA()

      // Get registry address
      const registryAddress = await getRegistryAddress()

      // Build RouterRelease instruction
      const instruction = new Transaction()

      // Instruction discriminator for RouterRelease (index 9)
      const discriminator = Buffer.from([9])
      
      // Amount in lamports (scaled by 1e6 for fixed point)
      const amountScaled = Math.floor(amount * 1_000_000)
      const amountBuffer = Buffer.alloc(8)
      const view = new DataView(amountBuffer.buffer)
      view.setBigInt64(0, BigInt(amountScaled), true)

      // Venue index (0 for now - you may need to adjust)
      const venueIndex = Buffer.from([0])

      const instructionData = Buffer.concat([
        discriminator,
        amountBuffer,
        venueIndex
      ])

      instruction.add({
        keys: [
          { pubkey: portfolioAddress, isSigner: false, isWritable: true },
          { pubkey: publicKey, isSigner: true, isWritable: false },
          { pubkey: vaultPDA, isSigner: false, isWritable: true },
          { pubkey: registryAddress, isSigner: false, isWritable: false },
          { pubkey: routerAuthorityPDA, isSigner: false, isWritable: false },
          { pubkey: slabAddress, isSigner: false, isWritable: true },
          { pubkey: SLAB_PROGRAM_ID, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: ROUTER_PROGRAM_ID,
        data: instructionData,
      })

      const signature = await sendTransaction(instruction, connection, {
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      })

      await connection.confirmTransaction(signature, 'confirmed')

      success(`✅ Released ${amount} ${coin} liquidity!`)
      setReleaseAmount('')

    } catch (err: any) {
      console.error('Release failed:', err)
      error(`Failed to release: ${err.message}`)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className={cn(
      "w-full bg-gradient-to-br from-black/40 via-black/30 to-black/20 backdrop-blur-xl rounded-2xl border border-[#181825] overflow-hidden",
      className
    )}>
      {/* Header */}
      <div className="px-4 py-3 border-b border-[#181825] bg-gradient-to-r from-purple-900/20 to-blue-900/20">
        <div className="flex items-center gap-2">
          <Droplets className="w-5 h-5 text-purple-400" />
          <h3 className="text-white font-semibold">LP Operations</h3>
        </div>
        <p className="text-xs text-gray-400 mt-1">Manage your liquidity provision</p>
      </div>

      {/* Info Banner */}
      <div className="mx-4 mt-4 p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg flex items-start gap-2">
        <AlertCircle className="w-4 h-4 text-blue-400 mt-0.5 shrink-0" />
        <div className="text-xs text-blue-300">
          <p className="font-semibold mb-1">Liquidity Provider Mode</p>
          <p className="text-blue-400">Reserve liquidity to earn trading fees. Release when you want to withdraw.</p>
        </div>
      </div>

      {/* Tab Selector */}
      <div className="p-4">
        <div className="flex gap-2 bg-black/40 p-1 rounded-lg mb-4">
          <button
            onClick={() => setActiveTab('reserve')}
            className={cn(
              "flex-1 py-2 px-3 rounded-lg text-sm font-semibold transition-all flex items-center justify-center gap-2",
              activeTab === 'reserve'
                ? "bg-gradient-to-r from-purple-500/30 to-blue-500/20 text-white border border-purple-500/50"
                : "bg-transparent text-gray-400 hover:text-white"
            )}
          >
            <Lock className="w-4 h-4" />
            Reserve
          </button>
          <button
            onClick={() => setActiveTab('release')}
            className={cn(
              "flex-1 py-2 px-3 rounded-lg text-sm font-semibold transition-all flex items-center justify-center gap-2",
              activeTab === 'release'
                ? "bg-gradient-to-r from-orange-500/30 to-red-500/20 text-white border border-orange-500/50"
                : "bg-transparent text-gray-400 hover:text-white"
            )}
          >
            <Unlock className="w-4 h-4" />
            Release
          </button>
        </div>

        {activeTab === 'reserve' ? (
          <div className="space-y-4">
            {/* Reserve Form */}
            <div>
              <label className="block text-sm text-gray-400 mb-2">
                Amount to Reserve ({coin})
              </label>
              <input
                type="number"
                value={reserveAmount}
                onChange={(e) => setReserveAmount(e.target.value)}
                placeholder="0.00"
                step="0.01"
                min="0"
                className="w-full px-4 py-3 bg-black/40 border border-gray-700 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-purple-500 transition-colors"
              />
              <p className="text-xs text-gray-500 mt-1">
                Commit liquidity to the {coin} pool
              </p>
            </div>

            {/* Reserve Button */}
            <button
              onClick={handleReserve}
              disabled={loading || !publicKey || !reserveAmount || parseFloat(reserveAmount) <= 0}
              className="w-full py-3 px-4 rounded-xl bg-gradient-to-r from-purple-600 to-blue-600 text-white font-bold text-sm hover:opacity-90 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {loading ? (
                <>
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  <span>Reserving...</span>
                </>
              ) : (
                <>
                  <Lock className="w-4 h-4" />
                  <span>Reserve Liquidity</span>
                </>
              )}
            </button>

            {/* Stats */}
            <div className="grid grid-cols-2 gap-3 pt-3 border-t border-gray-800">
              <div className="bg-black/40 rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">Your Reserved</div>
                <div className="text-lg font-bold text-white">0.00 {coin}</div>
              </div>
              <div className="bg-black/40 rounded-lg p-3">
                <div className="text-xs text-gray-400 mb-1">Earnings</div>
                <div className="text-lg font-bold text-green-400 flex items-center gap-1">
                  <TrendingUp className="w-4 h-4" />
                  0.00 {coin}
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Release Form */}
            <div>
              <label className="block text-sm text-gray-400 mb-2">
                Amount to Release ({coin})
              </label>
              <input
                type="number"
                value={releaseAmount}
                onChange={(e) => setReleaseAmount(e.target.value)}
                placeholder="0.00"
                step="0.01"
                min="0"
                className="w-full px-4 py-3 bg-black/40 border border-gray-700 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-orange-500 transition-colors"
              />
              <p className="text-xs text-gray-500 mt-1">
                Withdraw liquidity from the {coin} pool
              </p>
            </div>

            {/* Release Button */}
            <button
              onClick={handleRelease}
              disabled={loading || !publicKey || !releaseAmount || parseFloat(releaseAmount) <= 0}
              className="w-full py-3 px-4 rounded-xl bg-gradient-to-r from-orange-600 to-red-600 text-white font-bold text-sm hover:opacity-90 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {loading ? (
                <>
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  <span>Releasing...</span>
                </>
              ) : (
                <>
                  <Unlock className="w-4 h-4" />
                  <span>Release Liquidity</span>
                </>
              )}
            </button>

            {/* Available to Release */}
            <div className="bg-black/40 rounded-lg p-4 border border-gray-800">
              <div className="text-sm text-gray-400 mb-2">Available to Release</div>
              <div className="text-2xl font-bold text-white">0.00 {coin}</div>
              <button 
                onClick={() => setReleaseAmount('0.00')}
                className="mt-2 text-xs text-blue-400 hover:text-blue-300 transition-colors"
              >
                Max
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

