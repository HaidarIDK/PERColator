"use client"

import { useState, useEffect } from "react"
import { useWallet } from "@solana/wallet-adapter-react"
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui"
import { Transaction, Connection, clusterApiUrl } from "@solana/web3.js"
import { Buffer } from "buffer"
import { Particles } from "@/components/ui/particles"
import { cn } from "@/lib/utils"
import Link from "next/link"
import { ArrowLeft, TrendingUp, TrendingDown, Zap, Wallet as WalletIcon } from "lucide-react"
import { ACCOUNTS, NETWORK, EXPLORERS, formatAddress } from "@/lib/program-config"
import { toast } from "@/components/ui/use-toast"

export default function SimpleTradePage() {
  const { publicKey, connected, signTransaction } = useWallet()
  const [mounted, setMounted] = useState(false)
  const [side, setSide] = useState<'buy' | 'sell'>('buy')
  const [price, setPrice] = useState('3900')
  const [amount, setAmount] = useState('1.0')
  const [submitting, setSubmitting] = useState(false)
  const [lastHoldId, setLastHoldId] = useState<number | null>(null)
  const [mode, setMode] = useState<'reserve' | 'commit'>('reserve')
  
  // Order book state for YOUR Slab account
  const [orderbook, setOrderbook] = useState<{ bids: { price: number; quantity: number }[]; asks: { price: number; quantity: number }[]; midPrice?: number; spread?: number } | null>(null)
  const [recentTrades, setRecentTrades] = useState<Array<{ price: number; quantity: number; timestamp: number; side: string }>>([])
  const [transactions, setTransactions] = useState<Array<{ signature: string; blockTime?: number; err?: unknown; signer?: string; solscanLink?: string; slot?: number; memo?: string }>>([])
  const [balance, setBalance] = useState<number>(0)
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date())

  useEffect(() => {
    setMounted(true)
  }, [])

  // Fetch YOUR Slab's order book (LIVE from blockchain!)
  useEffect(() => {
    const fetchOrderbook = async () => {
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'
        console.log('📖 Fetching LIVE order book from Slab account...')
        const response = await fetch(`${API_URL}/api/slab-live/orderbook`)
        const data = await response.json()
        
        if (data.success) {
          console.log(`✅ Slab account: ${data.slabAccount}`)
          console.log(`   Size: ${data.accountSize} bytes`)
          console.log(`   Orders: ${data.orderbook.bids.length} bids, ${data.orderbook.asks.length} asks`)
          
          setOrderbook(data.orderbook)
          setRecentTrades(data.recentTrades || [])
          setLastUpdate(new Date())
        }
      } catch (error) {
        console.error('Failed to fetch orderbook:', error)
      }
    }

    fetchOrderbook()
    const interval = setInterval(fetchOrderbook, 5000) // Update every 5s from blockchain
    return () => clearInterval(interval)
  }, [])

  // Fetch transaction history from blockchain
  useEffect(() => {
    const fetchTransactions = async () => {
      try {
        const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'
        const response = await fetch(`${API_URL}/api/slab-live/transactions?limit=20`)
        const data = await response.json()
        
        if (data.success) {
          setTransactions(data.transactions)
        }
      } catch (error) {
        console.error('Failed to fetch transactions:', error)
      }
    }

    fetchTransactions()
    const interval = setInterval(fetchTransactions, 10000) // Update every 10s
    return () => clearInterval(interval)
  }, [])

  // Fetch balance
  useEffect(() => {
    if (!connected || !publicKey) return

    const fetchBalance = async () => {
      try {
        const connection = new Connection(clusterApiUrl('devnet'), 'confirmed')
        const bal = await connection.getBalance(publicKey)
        setBalance(bal / 1e9)
      } catch (error) {
        console.error('Failed to fetch balance:', error)
      }
    }

    fetchBalance()
    const interval = setInterval(fetchBalance, 10000)
    return () => clearInterval(interval)
  }, [connected, publicKey])

  const handleTrade = async () => {
    if (!connected || !publicKey || !signTransaction) {
      toast({ type: 'error', title: 'Wallet Not Connected', message: 'Please connect your wallet first' });
      return
    }

    if (!price || !amount) {
      toast({ type: 'error', title: 'Invalid Input', message: 'Please enter price and amount' });
      return
    }

    setSubmitting(true)
    try {
      const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001'
      
      if (mode === 'reserve') {
        // RESERVE
        const response = await fetch(`${API_URL}/api/trade/reserve`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            user: publicKey.toBase58(),
            instrument: 0, // ETH
            side: side,
            price: parseFloat(price),
            quantity: parseFloat(amount)
          })
        })

        const result = await response.json()
        
        if (!result.success) {
          alert(`Error: ${result.error}`)
          setSubmitting(false)
          return
        }

        if (result.needsSigning && result.transaction) {
          const txBuffer = Buffer.from(result.transaction, 'base64')
          const transaction = Transaction.from(txBuffer)
          const signedTx = await signTransaction(transaction)
          
          const connection = new Connection(clusterApiUrl('devnet'), 'confirmed')
          
          // Send transaction with maxRetries: 0 to prevent double submission
          const signature = await connection.sendRawTransaction(signedTx.serialize(), {
            skipPreflight: false,
            maxRetries: 0,
          })
          
          await connection.confirmTransaction(signature, 'confirmed')
          
          setLastHoldId(result.holdId)
          setMode('commit')
          toast({ 
            type: 'success', 
            title: 'Reservation Successful!', 
            message: `Hold ID: ${result.holdId}. Now click COMMIT to execute.` 
          });
          toast({
            type: 'info',
            title: 'View Transaction',
            message: `Tx: ${signature.substring(0, 8)}... (Click to copy)`,
            duration: 10000
          });
        }
      } else {
        // COMMIT
        if (!lastHoldId) {
          toast({ type: 'error', title: 'No Reservation', message: 'Please reserve first before committing' });
          setSubmitting(false)
          return
        }

        const response = await fetch(`${API_URL}/api/trade/commit`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            user: publicKey.toBase58(),
            holdId: lastHoldId
          })
        })

        const result = await response.json()
        
        if (!result.success) {
          alert(`Error: ${result.error}`)
          setSubmitting(false)
          return
        }

        if (result.needsSigning && result.transaction) {
          const txBuffer = Buffer.from(result.transaction, 'base64')
          const transaction = Transaction.from(txBuffer)
          const signedTx = await signTransaction(transaction)
          
          const connection = new Connection(clusterApiUrl('devnet'), 'confirmed')
          
          // Send transaction with skipPreflight to avoid "already processed" errors  
          const signature = await connection.sendRawTransaction(signedTx.serialize(), {
            skipPreflight: false,
            maxRetries: 0, // Don't retry - prevents double submission
          })
          
          await connection.confirmTransaction(signature, 'confirmed')
          
          // Record the successful trade
          try {
            await fetch(`${API_URL}/api/trade/record-fill`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                user: publicKey.toBase58(),
                side: result.orderData?.side || side,
                price: result.vwapPrice || parseFloat(price),
                quantity: result.filledQty || parseFloat(amount),
                signature
              })
            });
          } catch (recordError) {
            console.error('Failed to record trade:', recordError);
          }
          
          toast({ 
            type: 'success', 
            title: '🎉 Trade Executed!', 
            message: `${side.toUpperCase()} ${amount} @ $${price}` 
          });
          toast({
            type: 'info',
            title: 'View on Explorer',
            message: `${EXPLORERS.transaction(signature, NETWORK.cluster)}`,
            duration: 15000
          });
          
          setPrice('')
          setAmount('')
          setLastHoldId(null)
          setMode('reserve')
        }
      }
    } catch (error: unknown) {
      console.error('Trade failed:', error)
      const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
      toast({ 
        type: 'error', 
        title: 'Trade Failed', 
        message: errorMessage
      });
    } finally {
      setSubmitting(false)
    }
  }

  const total = (parseFloat(price || '0') * parseFloat(amount || '0')).toFixed(2)

  if (!mounted) return null

  return (
    <div className="relative min-h-screen bg-gradient-to-b from-black via-zinc-950 to-black text-white overflow-hidden">
      {/* Starry Background */}
      <Particles 
        className="absolute inset-0 z-0" 
        quantity={100}
        color="#ffffff"
        size={0.4}
        staticity={50}
        ease={50}
      />
      
      {/* Testnet Notice Banner */}
      <div className="relative z-10 bg-gradient-to-r from-yellow-900/30 to-orange-900/30 border-b border-yellow-700/50">
        <div className="max-w-[1600px] mx-auto px-6 py-3">
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-center gap-3 text-sm flex-wrap">
              <span className="text-yellow-400 font-bold">⚠️ DEVNET TESTNET ONLY</span>
              <span className="text-zinc-300">•</span>
              <span className="text-zinc-300">
                Make sure you&apos;re using <strong className="text-yellow-300">Devnet SOL</strong> (not real SOL!)
              </span>
              <span className="text-zinc-300">•</span>
              <a 
                href="https://faucet.solana.com" 
                target="_blank" 
                rel="noopener noreferrer"
                className="text-blue-400 hover:text-blue-300 underline font-semibold"
              >
                Get Free Testnet SOL →
              </a>
            </div>
            <div className="flex items-center justify-center gap-2 text-xs text-zinc-400">
              <span>📱 Phantom Setup:</span>
              <span className="text-zinc-500">Settings → Developer Settings → Enable Testnet Mode → Select &quot;Solana Devnet&quot;</span>
            </div>
          </div>
        </div>
      </div>

      {/* Header */}
      <div className="relative z-10 border-b border-zinc-800 bg-black/50 backdrop-blur-md">
        <div className="max-w-[1600px] mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Link href="/dashboard">
                <button className="p-2 rounded-lg bg-zinc-800 hover:bg-zinc-700 transition-all">
                  <ArrowLeft className="w-5 h-5" />
                </button>
              </Link>
              <div>
                <h1 className="text-2xl font-bold">Percolator Trade</h1>
                <p className="text-sm text-zinc-400">ETH/USDC · Devnet</p>
              </div>
            </div>
            
            <div className="flex items-center space-x-3">
              <Link href="/portfolio">
                <button className="px-4 py-2 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-sm flex items-center space-x-2 transition-all">
                  <WalletIcon className="w-4 h-4" />
                  <span>Portfolio</span>
                </button>
              </Link>
              <Link href="/v0">
                <button className="px-4 py-2 rounded-lg bg-green-600/20 hover:bg-green-600/30 border border-green-500/50 text-green-400 text-sm font-bold transition-all">
                  v0 POC
                </button>
              </Link>
              {mounted && <WalletMultiButton />}
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="relative z-10 max-w-[1600px] mx-auto px-6 py-6">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          
          {/* LEFT: Your Slab's Order Book & Transaction History (2/3 width) */}
          <div className="lg:col-span-2 space-y-6">
            <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-6 backdrop-blur-sm">
              <div className="flex items-center justify-between mb-6">
                <div>
                  <h3 className="text-xl font-bold">Order Book</h3>
                  <p className="text-xs text-zinc-500 mt-1">ETH/USDC · Your Slab Pool</p>
                </div>
                
                <div className="text-xs text-zinc-500">
                  Updated: {lastUpdate.toLocaleTimeString()}
                </div>
              </div>

              {/* Asks (Sell Orders) */}
              <div className="mb-6">
                <div className="flex items-center justify-between text-xs text-zinc-500 font-semibold mb-2 px-3">
                  <span>Price (USDC)</span>
                  <span>Size (ETH)</span>
                  <span>Total (USDC)</span>
                </div>
                <div className="space-y-0.5">
                  {orderbook?.asks?.slice(0, 10).reverse().map((ask: { price: number; quantity: number }, i: number) => (
                    <div 
                      key={i} 
                      className="flex items-center justify-between text-sm px-3 py-1.5 hover:bg-red-500/10 rounded transition-colors group cursor-pointer"
                      onClick={() => setPrice(ask.price.toFixed(2))}
                    >
                      <span className="text-red-400 font-mono font-semibold group-hover:text-red-300">
                        ${ask.price.toFixed(2)}
                      </span>
                      <span className="text-zinc-400">{ask.quantity.toFixed(4)}</span>
                      <span className="text-zinc-500">${(ask.price * ask.quantity).toFixed(2)}</span>
                    </div>
                  )) || (
                    <div className="text-center text-zinc-600 py-8 border border-zinc-800 rounded-lg">
                      No sell orders
                    </div>
                  )}
                </div>
              </div>

              {/* Mid Price */}
              <div className="my-6 py-4 bg-zinc-800/50 border border-zinc-700 rounded-lg text-center">
                <div className="text-3xl font-bold text-white mb-1">
                  ${orderbook?.midPrice?.toFixed(2) || '0.00'}
                </div>
                <div className="text-xs text-zinc-500">Mid Price · Spread: {orderbook?.spread?.toFixed(4) || '0'}%</div>
              </div>

              {/* Bids (Buy Orders) */}
              <div>
                <div className="space-y-0.5">
                  {orderbook?.bids?.slice(0, 10).map((bid: { price: number; quantity: number }, i: number) => (
                    <div 
                      key={i} 
                      className="flex items-center justify-between text-sm px-3 py-1.5 hover:bg-green-500/10 rounded transition-colors group cursor-pointer"
                      onClick={() => setPrice(bid.price.toFixed(2))}
                    >
                      <span className="text-green-400 font-mono font-semibold group-hover:text-green-300">
                        ${bid.price.toFixed(2)}
                      </span>
                      <span className="text-zinc-400">{bid.quantity.toFixed(4)}</span>
                      <span className="text-zinc-500">${(bid.price * bid.quantity).toFixed(2)}</span>
                    </div>
                  )) || (
                    <div className="text-center text-zinc-600 py-8 border border-zinc-800 rounded-lg">
                      No buy orders
                    </div>
                  )}
                </div>
              </div>

              {/* Recent Trades */}
              {recentTrades.length > 0 && (
                <div className="mt-6">
                  <h4 className="text-sm font-semibold text-zinc-400 mb-3">Recent Trades</h4>
                  <div className="space-y-2">
                    {recentTrades.map((trade, i) => (
                      <div 
                        key={i}
                        className="flex items-center justify-between text-xs p-2 bg-zinc-800/50 rounded border border-zinc-700/50"
                      >
                        <span className={trade.side === 'buy' ? 'text-green-400' : 'text-red-400'}>
                          {trade.side.toUpperCase()}
                        </span>
                        <span className="text-white font-mono">{trade.quantity.toFixed(4)}</span>
                        <span className="text-zinc-400">@ ${trade.price.toFixed(2)}</span>
                        <span className="text-zinc-500 text-[10px]">
                          {new Date(trade.timestamp).toLocaleTimeString()}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Pool Info */}
              <div className="mt-6 p-4 bg-blue-600/10 border border-blue-500/30 rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm text-blue-300 font-semibold">
                    🏊 Your Slab Pool (On-Chain)
                  </div>
                  <div className="flex items-center space-x-1">
                    <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                    <span className="text-xs text-green-400">Live from Blockchain</span>
                  </div>
                </div>
                <div className="text-xs text-zinc-400 font-mono mb-2">
                  {ACCOUNTS.slab.toBase58()}
                </div>
                {(!orderbook || (orderbook.bids?.length === 0 && orderbook.asks?.length === 0)) && (
                  <div className="text-xs text-yellow-400 bg-yellow-500/10 border border-yellow-500/30 rounded px-2 py-1 mb-2">
                    ✨ Fresh pool! Be the first to place an order.
                  </div>
                )}
                <a
                  href={EXPLORERS.solanaExplorer(ACCOUNTS.slab.toBase58(), NETWORK.cluster)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-blue-400 hover:text-blue-300 inline-block"
                >
                  View on Solana Explorer →
                </a>
              </div>
            </div>

            {/* Transaction History Section */}
            <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-6 backdrop-blur-sm">
              <div className="flex items-center justify-between mb-4">
                <div>
                  <h3 className="text-xl font-bold">All Slab Transactions</h3>
                  <p className="text-xs text-zinc-500 mt-1">On-Chain History · Last 20</p>
                </div>
                <a
                  href={EXPLORERS.solanaExplorer(ACCOUNTS.slab.toBase58(), NETWORK.cluster)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-blue-400 hover:text-blue-300"
                >
                  View All on Explorer →
                </a>
              </div>

              <div className="space-y-2">
                {transactions.length === 0 ? (
                  <div className="text-center py-8 text-zinc-600 border border-zinc-800 rounded-lg">
                    No transactions yet. Be the first to trade!
                  </div>
                ) : (
                  transactions.map((tx, i) => {
                    // Alternate colors for Reserve (blue) and Commit (green)
                    const isReserve = i % 2 === 0;
                    const typeColor = isReserve ? 'blue' : 'green';
                    const typeBg = isReserve ? 'bg-blue-900/10' : 'bg-green-900/10';
                    const typeBorder = isReserve ? 'border-blue-700/30' : 'border-green-700/30';
                    const typeLabel = isReserve ? 'RESERVE' : 'COMMIT';
                    
                    return (
                      <a
                        key={i}
                        href={tx.solscanLink}
                        target="_blank"
                        rel="noopener noreferrer"
                        className={`block p-3 ${typeBg} hover:bg-zinc-800/50 rounded-lg border ${typeBorder} transition-all group`}
                      >
                        <div className="flex items-center justify-between mb-2">
                          <div className="flex items-center gap-2">
                            {tx.err ? (
                              <span className="text-red-400 text-xs font-semibold">UNSUCCESSFUL</span>
                            ) : (
                              <span className="text-green-400 text-xs font-semibold">SUCCESS</span>
                            )}
                            <span className={`text-${typeColor}-400 text-[10px] font-mono bg-${typeColor}-900/30 px-2 py-0.5 rounded border border-${typeColor}-700/50`}>
                              {typeLabel}
                            </span>
                          <span className="text-zinc-500 text-xs">
                            Slot {tx.slot?.toLocaleString()}
                          </span>
                          </div>
                          <span className="text-zinc-500 text-xs">
                            {tx.blockTime ? new Date(tx.blockTime * 1000).toLocaleString() : 'Pending'}
                          </span>
                        </div>
                        {tx.signer && tx.signer.length > 8 && (
                          <div className="flex items-center justify-between text-xs mb-1">
                            <span className="text-zinc-500">Wallet:</span>
                            <code className="text-zinc-400 font-mono">
                              {`${tx.signer.substring(0, 6)}...${tx.signer.substring(tx.signer.length - 4)}`}
                            </code>
                          </div>
                        )}
                        <div className="flex items-center justify-between">
                        <code className="text-xs font-mono text-zinc-400 group-hover:text-zinc-300">
                          {tx.signature.substring(0, 16)}...{tx.signature.substring(tx.signature.length - 16)}
                        </code>
                        <span className="text-blue-400 text-xs group-hover:text-blue-300">
                          View →
                        </span>
                      </div>
                      {tx.memo && (
                        <div className="mt-2 text-xs text-zinc-500">{tx.memo}</div>
                      )}
                    </a>
                    );
                  })
                )}
              </div>
            </div>
          </div>

          {/* RIGHT: Trade Panel (1/3 width) */}
          <div className="lg:col-span-1">
            <div className="bg-zinc-900/50 border border-zinc-800 rounded-xl p-6 backdrop-blur-sm sticky top-6">
              <h3 className="text-xl font-bold mb-4">Place Order</h3>

              {/* Buy/Sell Tabs */}
              <div className="flex gap-2 mb-6">
                <button
                  onClick={() => setSide('buy')}
                  className={cn(
                    "flex-1 py-3 rounded-lg font-bold transition-all",
                    side === 'buy'
                      ? "bg-green-600 text-white shadow-lg shadow-green-900/50"
                      : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                  )}
                >
                  BUY
                </button>
                <button
                  onClick={() => setSide('sell')}
                  className={cn(
                    "flex-1 py-3 rounded-lg font-bold transition-all",
                    side === 'sell'
                      ? "bg-red-600 text-white shadow-lg shadow-red-900/50"
                      : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                  )}
                >
                  SELL
                </button>
              </div>

              {/* Price Input */}
              <div className="mb-4">
                <label className="block text-sm mb-2 text-zinc-400 font-semibold">
                  Price (USDC)
                </label>
                <input
                  type="number"
                  value={price}
                  onChange={(e) => setPrice(e.target.value)}
                  placeholder="3900"
                  className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-zinc-500 transition-colors"
                />
              </div>

              {/* Amount Input */}
              <div className="mb-4">
                <label className="block text-sm mb-2 text-zinc-400 font-semibold">
                  Amount (ETH)
                </label>
                <input
                  type="number"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="1.0"
                  step="0.1"
                  className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-zinc-500 transition-colors"
                />
              </div>

              {/* Total */}
              <div className="mb-6 p-4 bg-zinc-800/50 rounded-lg border border-zinc-700">
                <div className="text-xs text-zinc-500 mb-1">TOTAL</div>
                <div className="text-2xl font-bold text-white">${total}</div>
              </div>

              {/* Wallet Balance */}
              <div className="mb-6 flex items-center justify-between text-sm">
                <span className="text-zinc-500">Wallet balance (devnet)</span>
                <div className="flex items-center gap-2">
                  <span className="text-white font-mono">
                    {connected ? `${balance.toFixed(2)} SOL` : '--'}
                  </span>
                </div>
              </div>

              {/* Test Transaction Button (v0 POC) */}
              {connected && (
                <button
                  onClick={async () => {
                    if (!publicKey || !signTransaction) return;
                    
                    try {
                      setSubmitting(true);
                      const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
                      
                      const response = await fetch(`${API_URL}/api/trade/test-transfer`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ user: publicKey.toBase58() })
                      });
                      
                      const result = await response.json();
                      
                      if (result.success && result.transaction) {
                        const txBuffer = Buffer.from(result.transaction, 'base64');
                        const transaction = Transaction.from(txBuffer);
                        const signedTx = await signTransaction(transaction);
                        
                        const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
                        const signature = await connection.sendRawTransaction(signedTx.serialize());
                        await connection.confirmTransaction(signature, 'confirmed');
                        
                        toast({ 
                          type: 'success', 
                          title: '✅ Test Transaction Success!', 
                          message: 'Wallet signing is working correctly!' 
                        });
                        toast({
                          type: 'info',
                          title: 'View on Solscan',
                          message: `${EXPLORERS.transaction(signature, NETWORK.cluster)}`,
                          duration: 15000
                        });
                      }
                    } catch (error: unknown) {
                      const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
                      toast({ type: 'error', title: 'Test Failed', message: errorMessage });
                    } finally {
                      setSubmitting(false);
                    }
                  }}
                  disabled={submitting}
                  className="w-full py-3 mb-4 rounded-lg bg-purple-600 hover:bg-purple-700 text-white font-bold transition-all disabled:opacity-50"
                >
                  {submitting ? '⏳ Processing...' : '🧪 Test Wallet Connection'}
                </button>
              )}

              {/* Trade Button */}
              {!connected ? (
                <div className="w-full py-4 font-bold text-white border border-zinc-800 bg-zinc-900 hover:border-zinc-600 transition-colors rounded-lg flex items-center justify-center">
                  {mounted ? <WalletMultiButton /> : 'Loading...'}
                </div>
              ) : (
                <div className="space-y-3">
                  {mode === 'reserve' ? (
                    <button
                      onClick={handleTrade}
                      disabled={submitting || !price || !amount}
                      className={cn(
                        "w-full py-4 rounded-lg font-bold transition-all disabled:opacity-50",
                        side === 'buy'
                          ? "bg-green-600 hover:bg-green-700 text-white shadow-lg shadow-green-900/50"
                          : "bg-red-600 hover:bg-red-700 text-white shadow-lg shadow-red-900/50"
                      )}
                    >
                      {submitting ? (
                        <span className="flex items-center justify-center space-x-2">
                          <span className="animate-spin">⏳</span>
                          <span>Processing...</span>
                        </span>
                      ) : (
                        `${side.toUpperCase()} ${amount || '0'} ETH`
                      )}
                    </button>
                  ) : (
                    <button
                      onClick={handleTrade}
                      disabled={submitting || !lastHoldId}
                      className="w-full py-4 rounded-lg font-bold bg-blue-600 hover:bg-blue-700 text-white transition-all disabled:opacity-50 shadow-lg shadow-blue-900/50"
                    >
                      {submitting ? 'Committing...' : `✅ COMMIT TRADE (Hold #${lastHoldId})`}
                    </button>
                  )}
                  
                  {mode === 'commit' && (
                    <button
                      onClick={() => {
                        setMode('reserve')
                        setLastHoldId(null)
                      }}
                      className="w-full py-2 rounded-lg border border-zinc-700 text-zinc-400 hover:bg-zinc-800 transition-all text-sm"
                    >
                      Cancel & New Order
                    </button>
                  )}
                </div>
              )}

              {/* v0.1 Update Notice */}
              <div className="mt-6 p-3 bg-green-500/10 border border-green-500/30 rounded-lg">
                <div className="text-xs text-green-300 font-semibold mb-1">
                  ✅ v0.1 - Reserve/Commit Enabled!
                </div>
                <div className="text-xs text-zinc-400">
                  Reserve, Commit, and Cancel instructions are now live on devnet!
                  The program logs your trades (POC mode). Full orderbook matching coming in v0.2!
                </div>
              </div>
              
              {/* Info */}
              <div className="mt-4 p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg">
                <div className="text-xs text-blue-300 font-semibold mb-1">
                  🎯 How It Works
                </div>
                <div className="text-xs text-zinc-400">
                  1. Click BUY/SELL to Reserve liquidity<br/>
                  2. Sign the transaction in your wallet<br/>
                  3. Click COMMIT to execute the trade<br/>
                  4. View your transaction on Solana Explorer!
                </div>
              </div>

              {/* Program Info */}
              <div className="mt-4 p-3 bg-zinc-800/30 rounded-lg border border-zinc-700/50">
                <div className="text-xs text-zinc-500 mb-1">Slab Account</div>
                <div className="text-xs font-mono text-zinc-400">
                  {formatAddress(ACCOUNTS.slab)}
                </div>
                <div className="text-xs text-zinc-500 mb-1 mt-2">Instrument ID</div>
                <div className="text-xs font-mono text-zinc-400">
                  {formatAddress(ACCOUNTS.instrument)}
                </div>
                <a
                  href={EXPLORERS.solanaExplorer(ACCOUNTS.slab.toBase58(), NETWORK.cluster)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-blue-400 hover:text-blue-300 mt-2 inline-block"
                >
                  View on Explorer →
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

