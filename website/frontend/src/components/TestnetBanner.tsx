"use client"

import { useState } from 'react'
import { AlertCircle, Zap, ExternalLink } from 'lucide-react'
import { useWallet, useConnection } from '@solana/wallet-adapter-react'
import { LAMPORTS_PER_SOL } from '@solana/web3.js'
import { useToast } from './ToastProvider'

export function TestnetBanner() {
  const { publicKey, connected } = useWallet()
  const { connection } = useConnection()
  const { success, error, warning } = useToast()
  const [faucetLoading, setFaucetLoading] = useState(false)
  const [showSuccess, setShowSuccess] = useState(false)

  const handleFaucetRequest = async () => {
    if (!connected || !publicKey) {
      warning('Please connect your wallet first')
      return
    }

    setFaucetLoading(true)
    setShowSuccess(false)

    try {
      // Request airdrop from Solana devnet
      const signature = await connection.requestAirdrop(
        publicKey,
        2 * LAMPORTS_PER_SOL // 2 SOL
      )

      // Wait for confirmation
      await connection.confirmTransaction(signature, 'confirmed')

      setShowSuccess(true)
      success('✅ Received 2 SOL from faucet!')

      // Hide success message after 3 seconds
      setTimeout(() => setShowSuccess(false), 3000)
    } catch (err: any) {
      console.error('Faucet error:', err)
      
      if (err.message?.includes('airdrop request limit')) {
        error('Airdrop limit reached. Try again in a few hours or use faucet.solana.com')
      } else {
        error('Failed to request airdrop. Please try faucet.solana.com')
      }
    } finally {
      setFaucetLoading(false)
    }
  }

  return (
    <div className="relative z-20 bg-gradient-to-r from-yellow-900/30 to-orange-900/30 border-b border-yellow-700/50">
      <div className="max-w-[1600px] mx-auto px-3 sm:px-6 py-2 sm:py-3">
        <div className="flex flex-col gap-1.5 sm:gap-2">
          {/* Main Warning */}
          <div className="flex items-center justify-center gap-2 sm:gap-3 text-xs sm:text-sm flex-wrap">
            <AlertCircle className="w-4 h-4 text-yellow-400" />
            <span className="text-yellow-400 font-bold text-xs sm:text-sm">
              ⚠️ DEVNET TESTNET ONLY
            </span>
            <span className="text-zinc-300 hidden sm:inline">•</span>
            <span className="text-zinc-300 text-center text-xs sm:text-sm">
              Make sure you're using <strong className="text-yellow-300">Devnet SOL</strong> (not real SOL!)
            </span>
            
            {/* Faucet Buttons */}
            <span className="text-zinc-300 hidden sm:inline">•</span>
            <div className="flex items-center gap-2">
              {connected && (
                <button
                  onClick={handleFaucetRequest}
                  disabled={faucetLoading}
                  className="px-2 sm:px-3 py-1 bg-amber-500/20 border border-amber-500/40 rounded text-amber-300 text-xs font-semibold hover:bg-amber-500/30 transition-all disabled:opacity-50 flex items-center gap-1.5"
                >
                  {faucetLoading ? (
                    <>
                      <div className="animate-spin rounded-full h-3 w-3 border-b border-current"></div>
                      <span>Requesting...</span>
                    </>
                  ) : showSuccess ? (
                    <>
                      <span>✅</span>
                      <span>+2 SOL Received!</span>
                    </>
                  ) : (
                    <>
                      <Zap className="w-3 h-3" />
                      <span>Get 2 SOL</span>
                    </>
                  )}
                </button>
              )}
              
              <a 
                href="https://faucet.solana.com" 
                target="_blank" 
                rel="noopener noreferrer"
                className="text-blue-400 hover:text-blue-300 underline font-semibold text-xs sm:text-sm flex items-center gap-1"
              >
                <span>Solana Faucet</span>
                <ExternalLink className="w-3 h-3" />
              </a>
            </div>
          </div>
          
          {/* Phantom Setup Instructions */}
          <div className="flex flex-col sm:flex-row items-center justify-center gap-1 sm:gap-2 text-[10px] sm:text-xs text-zinc-400 text-center">
            <span className="font-semibold">📱 Phantom Setup:</span>
            <span className="text-zinc-500">
              Settings → Developer Settings → Enable Testnet Mode → Select "Solana Devnet"
            </span>
          </div>
        </div>
      </div>
    </div>
  )
}

