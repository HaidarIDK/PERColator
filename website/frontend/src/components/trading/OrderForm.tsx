"use client"

import { useState, useEffect, useRef } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Transaction, Connection, clusterApiUrl } from '@solana/web3.js';
import { toast } from '@/components/ui/use-toast';
import { Clock, AlertCircle } from 'lucide-react';

type OrderSide = 'buy' | 'sell';
type OrderType = 'limit' | 'market';

interface OrderFormProps {
  coin: "ethereum" | "bitcoin" | "solana" | "jupiter" | "bonk" | "dogwifhat";
  currentPrice: number;
}

interface ReservedOrder {
  side: OrderSide;
  price: number;
  size: number;
  expiresAt: number;
  transaction: Transaction;
  holdId?: number;
}

// Helper to convert coin string to v2 format
const coinToV2 = (coin: "ethereum" | "bitcoin" | "solana" | "jupiter" | "bonk" | "dogwifhat"): 'SOL' | 'ETH' | 'BTC' | 'JUP' | 'BONK' | 'WIF' => {
  switch(coin) {
    case 'ethereum': return 'ETH';
    case 'bitcoin': return 'BTC';
    case 'solana': return 'SOL';
    case 'jupiter': return 'JUP';
    case 'bonk': return 'BONK';
    case 'dogwifhat': return 'WIF';
  }
};

export function OrderForm({ coin, currentPrice }: OrderFormProps) {
  const { publicKey, signTransaction } = useWallet();
  const [orderSide, setOrderSide] = useState<OrderSide>('buy');
  const [orderType, setOrderType] = useState<OrderType>('limit');
  const [price, setPrice] = useState(currentPrice > 0 ? currentPrice.toFixed(2) : '');
  const [size, setSize] = useState('');
  const [loading, setLoading] = useState(false);
  const [reservedOrder, setReservedOrder] = useState<ReservedOrder | null>(null);
  const [timeRemaining, setTimeRemaining] = useState(0);
  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

  // Update price when currentPrice changes
  useEffect(() => {
    if (currentPrice > 0 && orderType === 'market') {
      setPrice(currentPrice.toFixed(2));
    }
  }, [currentPrice, orderType]);

  // Countdown timer effect
  useEffect(() => {
    if (reservedOrder) {
      timerRef.current = setInterval(() => {
        const remaining = Math.max(0, reservedOrder.expiresAt - Date.now());
        setTimeRemaining(remaining);
        
        if (remaining === 0) {
          // Auto-cancel expired reservation
          setReservedOrder(null);
          toast({
            type: 'warning',
            title: 'Reservation Expired',
            message: 'Reservation expired! Please reserve again.'
          });
          if (timerRef.current) clearInterval(timerRef.current);
        }
      }, 100);
      
      return () => {
        if (timerRef.current) clearInterval(timerRef.current);
      };
    }
  }, [reservedOrder]);

  // Step 1: Reserve the order (lock in price)
  const handleReserve = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!publicKey || !signTransaction) {
      toast({
        type: 'warning',
        title: 'Wallet Required',
        message: 'Please connect your wallet'
      });
      return;
    }

    setLoading(true);
    try {
      const orderPrice = orderType === 'limit' ? parseFloat(price) : currentPrice;
      const orderSize = parseFloat(size);

      if (orderSize <= 0 || orderPrice <= 0) {
        throw new Error('Invalid price or size');
      }

      const coinV2 = coinToV2(coin);
      const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';

      // Call API to reserve order
      const response = await fetch(`${API_URL}/api/trade/reserve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          user: publicKey.toBase58(),
          slice: coinV2,
          side: orderSide,
          orderType: orderType,
          price: orderPrice,
          quantity: orderSize,
        })
      });

      const result = await response.json();

      if (!response.ok || !result.success) {
        throw new Error(result.error || 'Failed to reserve order');
      }

      // Create transaction from API response (already has blockhash and fee payer set)
      const transaction = Transaction.from(Buffer.from(result.transaction, 'base64'));

      // Use expiry from API response, or default to 30 seconds
      const expiryMs = result.expiryMs || (Date.now() + 30000);

      // Create reservation
      const reservation: ReservedOrder = {
        side: orderSide,
        price: orderPrice,
        size: orderSize,
        expiresAt: expiryMs,
        transaction,
        holdId: result.holdId,
      };

      setReservedOrder(reservation);
      toast({
        type: 'success',
        title: 'Order Reserved',
        message: 'Order reserved! You have 30 seconds to commit.'
      });
      
    } catch (error: any) {
      console.error('Failed to reserve order:', error);
      toast({
        type: 'error',
        title: 'Reserve Failed',
        message: `Failed to reserve: ${error.message || error.toString()}`
      });
    } finally {
      setLoading(false);
    }
  };

  // Step 2: Commit the reserved order (two-phase commit)
  const handleCommit = async () => {
    if (!reservedOrder || !signTransaction || !publicKey) return;

    setLoading(true);
    try {
      const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';

      // Phase 1: Sign and send the RESERVE transaction first
      console.log('Phase 1: Signing reserve transaction...');
      const signedReserve = await signTransaction(reservedOrder.transaction);
      const reserveSignature = await connection.sendRawTransaction(signedReserve.serialize());

      toast({
        type: 'info',
        title: 'Processing',
        message: 'Reserve transaction sent, waiting for confirmation...'
      });

      await connection.confirmTransaction(reserveSignature, 'confirmed');
      console.log('Reserve transaction confirmed:', reserveSignature);

      // Phase 2: Call the commit API to get the commit transaction
      console.log('Phase 2: Requesting commit transaction...');

      const commitResponse = await fetch(`${API_URL}/api/trade/commit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          user: publicKey.toBase58(),
          holdId: reservedOrder.holdId,
        })
      });

      const commitResult = await commitResponse.json();

      if (!commitResponse.ok || !commitResult.success) {
        throw new Error(commitResult.error || 'Failed to get commit transaction');
      }

      // Sign and send the commit transaction
      const commitTx = Transaction.from(Buffer.from(commitResult.transaction, 'base64'));
      console.log('Phase 2: Signing commit transaction...');
      const signedCommit = await signTransaction(commitTx);
      const commitSignature = await connection.sendRawTransaction(signedCommit.serialize());

      await connection.confirmTransaction(commitSignature, 'confirmed');
      console.log('Commit transaction confirmed:', commitSignature);

      // Record the fill on the server
      await fetch(`${API_URL}/api/trade/record-fill`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          user: publicKey.toBase58(),
          side: reservedOrder.side,
          price: commitResult.vwapPrice || reservedOrder.price,
          quantity: commitResult.filledQty || reservedOrder.size,
          signature: commitSignature,
        })
      });

      toast({
        type: 'success',
        title: 'Order Executed',
        message: `${reservedOrder.side.toUpperCase()} order executed! ${reservedOrder.size} ${coinToV2(coin)} @ $${reservedOrder.price.toFixed(2)}`
      });

      // Reset form
      setReservedOrder(null);
      setSize('');
      setPrice(currentPrice > 0 ? currentPrice.toFixed(2) : '');
      if (timerRef.current) clearInterval(timerRef.current);

    } catch (error: any) {
      console.error('Failed to commit order:', error);
      toast({
        type: 'error',
        title: 'Commit Failed',
        message: `Failed to commit: ${error.message || error.toString()}`
      });
    } finally {
      setLoading(false);
    }
  };

  // Cancel reservation
  const handleCancel = () => {
    setReservedOrder(null);
    if (timerRef.current) clearInterval(timerRef.current);
    toast({
      type: 'info',
      title: 'Reservation Cancelled',
      message: 'Reservation cancelled'
    });
  };

  const totalValue = orderType === 'limit' 
    ? (parseFloat(price) * parseFloat(size || '0')).toFixed(2)
    : (currentPrice * parseFloat(size || '0')).toFixed(2);

  const coinV2 = coinToV2(coin);

  return (
    <div className="bg-zinc-900/50 rounded-xl p-3 border border-white/5">
      {!reservedOrder ? (
        <form onSubmit={handleReserve} className="space-y-3">
          {/* Order Side */}
          <div className="flex gap-1 p-1 bg-zinc-950 rounded-lg border border-white/5">
            <button
              type="button"
              onClick={() => setOrderSide('buy')}
              className={`flex-1 py-1.5 text-xs font-medium rounded-md transition-all ${
                orderSide === 'buy'
                  ? 'bg-emerald-500/20 text-emerald-400 shadow-sm border border-emerald-500/20'
                  : 'text-zinc-500 hover:text-zinc-300 hover:bg-white/5'
              }`}
            >
              Buy
            </button>
            <button
              type="button"
              onClick={() => setOrderSide('sell')}
              className={`flex-1 py-1.5 text-xs font-medium rounded-md transition-all ${
                orderSide === 'sell'
                  ? 'bg-red-500/20 text-red-400 shadow-sm border border-red-500/20'
                  : 'text-zinc-500 hover:text-zinc-300 hover:bg-white/5'
              }`}
            >
              Sell
            </button>
          </div>

          {/* Order Type */}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setOrderType('limit')}
              className={`flex-1 text-[10px] font-medium uppercase tracking-wider border-b-2 pb-1 transition-colors ${
                orderType === 'limit'
                  ? 'border-blue-500 text-blue-400'
                  : 'border-transparent text-zinc-500 hover:text-zinc-300'
              }`}
            >
              Limit
            </button>
            <button
              type="button"
              onClick={() => setOrderType('market')}
              className={`flex-1 text-[10px] font-medium uppercase tracking-wider border-b-2 pb-1 transition-colors ${
                orderType === 'market'
                  ? 'border-blue-500 text-blue-400'
                  : 'border-transparent text-zinc-500 hover:text-zinc-300'
              }`}
            >
              Market
            </button>
          </div>

          <div className="space-y-2">
            {/* Price Input (for limit orders) */}
            {orderType === 'limit' && (
              <div className="relative">
                <label className="absolute left-3 top-1/2 -translate-y-1/2 text-xs text-zinc-500 pointer-events-none">Price</label>
                <input
                  type="number"
                  step="0.01"
                  value={price}
                  onChange={(e) => setPrice(e.target.value)}
                  placeholder={currentPrice > 0 ? currentPrice.toFixed(2) : '0.00'}
                  className="w-full pl-12 pr-12 py-2 bg-zinc-950 border border-white/5 rounded-lg text-sm text-right text-zinc-100 placeholder-zinc-700 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20 transition-all"
                  required
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-zinc-600 pointer-events-none">USDC</span>
              </div>
            )}

            {/* Size Input */}
            <div className="relative">
              <label className="absolute left-3 top-1/2 -translate-y-1/2 text-xs text-zinc-500 pointer-events-none">Size</label>
              <input
                type="number"
                step="0.001"
                min="0"
                value={size}
                onChange={(e) => setSize(e.target.value)}
                placeholder="0.000"
                className="w-full pl-12 pr-12 py-2 bg-zinc-950 border border-white/5 rounded-lg text-sm text-right text-zinc-100 placeholder-zinc-700 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/20 transition-all"
                required
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-zinc-600 pointer-events-none">{coinV2}</span>
            </div>
          </div>

          {/* Total Value */}
          <div className="flex justify-between items-center px-1">
            <span className="text-xs text-zinc-500">Total Value</span>
            <span className="text-xs font-mono text-zinc-300">{totalValue} USDC</span>
          </div>

          {/* Reserve Button */}
          <button
            type="submit"
            disabled={loading || !publicKey || !size}
            className={`w-full py-2.5 text-xs font-bold uppercase tracking-wide rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed ${
              orderSide === 'buy'
                ? 'bg-emerald-500 hover:bg-emerald-600 text-white shadow-lg shadow-emerald-500/20'
                : 'bg-red-500 hover:bg-red-600 text-white shadow-lg shadow-red-500/20'
            }`}
          >
            {loading ? 'Processing...' : !publicKey ? 'Connect Wallet' : `Reserve ${orderSide === 'buy' ? 'Buy' : 'Sell'}`}
          </button>
        </form>
      ) : (
        <div className="space-y-3">
          {/* Reservation Info Banner */}
          <div className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-3">
            <div className="flex items-start gap-3">
              <AlertCircle className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
              <div>
                <p className="text-xs font-bold text-amber-500 mb-0.5">Order Reserved</p>
                <p className="text-[10px] text-amber-400/80">Price locked. Commit within 30 seconds.</p>
              </div>
            </div>
          </div>

          {/* Countdown Timer */}
          <div className="bg-zinc-950 rounded-lg p-3 border border-blue-500/20 relative overflow-hidden">
            <div className="absolute top-0 left-0 h-0.5 bg-blue-500/50 transition-all duration-1000" style={{ width: `${(timeRemaining / 30000) * 100}%` }} />
            
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs text-zinc-400">Time Remaining</span>
              <span className={`text-lg font-mono font-bold ${
                timeRemaining < 10000 ? 'text-red-400' : 'text-blue-400'
              }`}>
                {(timeRemaining / 1000).toFixed(1)}s
              </span>
            </div>
            
            {/* Order Details */}
            <div className="space-y-1.5">
              <div className="flex justify-between text-xs">
                <span className="text-zinc-500">Side</span>
                <span className={`font-bold ${reservedOrder.side === 'buy' ? 'text-emerald-400' : 'text-red-400'}`}>
                  {reservedOrder.side.toUpperCase()}
                </span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-zinc-500">Price</span>
                <span className="text-zinc-200 font-mono">${reservedOrder.price.toFixed(2)}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-zinc-500">Size</span>
                <span className="text-zinc-200 font-mono">{reservedOrder.size} {coinV2}</span>
              </div>
              <div className="flex justify-between border-t border-white/5 pt-2 text-xs">
                <span className="text-zinc-500">Total</span>
                <span className="text-zinc-100 font-mono font-bold">${(reservedOrder.price * reservedOrder.size).toFixed(2)} USDC</span>
              </div>
            </div>
          </div>

          {/* Commit & Cancel Buttons */}
          <div className="grid grid-cols-2 gap-2">
            <button
              onClick={handleCommit}
              disabled={loading}
              className="py-2 bg-emerald-500 hover:bg-emerald-600 text-white font-bold text-xs rounded-lg transition-all disabled:opacity-50 shadow-lg shadow-emerald-500/20"
            >
              {loading ? 'Committing...' : 'Commit'}
            </button>
            <button
              onClick={handleCancel}
              disabled={loading}
              className="py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-400 font-medium text-xs rounded-lg transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
