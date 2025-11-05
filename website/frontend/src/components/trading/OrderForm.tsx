"use client"

import { useState, useEffect, useRef } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Transaction, Connection, clusterApiUrl } from '@solana/web3.js';
import { toast } from '@/components/ui/use-toast';
import { Clock, AlertCircle } from 'lucide-react';

type OrderSide = 'buy' | 'sell';
type OrderType = 'limit' | 'market';

interface OrderFormProps {
  coin: "ethereum" | "bitcoin" | "solana";
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
const coinToV2 = (coin: "ethereum" | "bitcoin" | "solana"): 'SOL' | 'ETH' | 'BTC' => {
  switch(coin) {
    case 'ethereum': return 'ETH';
    case 'bitcoin': return 'BTC';
    case 'solana': return 'SOL';
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

  // Step 2: Commit the reserved order
  const handleCommit = async () => {
    if (!reservedOrder || !signTransaction) return;

    setLoading(true);
    try {
      // Sign and send the reserved transaction
      const signed = await signTransaction(reservedOrder.transaction);
      const signature = await connection.sendRawTransaction(signed.serialize());
      
      await connection.confirmTransaction(signature, 'confirmed');

      toast({
        type: 'success',
        title: 'Order Committed',
        message: `${reservedOrder.side.toUpperCase()} order committed! ${reservedOrder.size} ${coinToV2(coin)} @ $${reservedOrder.price.toFixed(2)}`
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
    <div className="bg-gray-800 rounded-lg p-2 sm:p-3 border border-gray-700">
      {!reservedOrder ? (
        <form onSubmit={handleReserve} className="space-y-2 sm:space-y-3">
          {/* Order Side */}
          <div className="flex gap-1.5">
            <button
              type="button"
              onClick={() => setOrderSide('buy')}
              className={`flex-1 py-1.5 sm:py-2 text-[10px] sm:text-xs font-semibold rounded transition-colors touch-manipulation ${
                orderSide === 'buy'
                  ? 'bg-green-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
              }`}
            >
              Buy
            </button>
            <button
              type="button"
              onClick={() => setOrderSide('sell')}
              className={`flex-1 py-1.5 sm:py-2 text-[10px] sm:text-xs font-semibold rounded transition-colors touch-manipulation ${
                orderSide === 'sell'
                  ? 'bg-red-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
              }`}
            >
              Sell
            </button>
          </div>

          {/* Order Type */}
          <div className="flex gap-1.5">
            <button
              type="button"
              onClick={() => setOrderType('limit')}
              className={`flex-1 py-1 text-[9px] sm:text-[10px] font-medium rounded transition-colors touch-manipulation ${
                orderType === 'limit'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
              }`}
            >
              Limit
            </button>
            <button
              type="button"
              onClick={() => setOrderType('market')}
              className={`flex-1 py-1 text-[9px] sm:text-[10px] font-medium rounded transition-colors touch-manipulation ${
                orderType === 'market'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
              }`}
            >
              Market
            </button>
          </div>

          {/* Price Input (for limit orders) */}
          {orderType === 'limit' && (
            <div>
              <label className="text-[10px] sm:text-xs text-gray-400 mb-1 block">Price (USDC)</label>
              <input
                type="number"
                step="0.01"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                placeholder={currentPrice > 0 ? currentPrice.toFixed(2) : '0.00'}
                className="w-full px-2 py-1.5 bg-gray-900 border border-gray-700 rounded text-[11px] sm:text-xs text-white placeholder-gray-500 focus:outline-none focus:border-blue-600"
                required
              />
            </div>
          )}

          {/* Size Input */}
          <div>
            <label className="text-[10px] sm:text-xs text-gray-400 mb-1 block">Size ({coinV2})</label>
              <input
                type="number"
                step="0.001"
                min="0"
                value={size}
                onChange={(e) => setSize(e.target.value)}
                placeholder="0.000"
                className="w-full px-2 py-1.5 bg-gray-900 border border-gray-700 rounded text-[11px] sm:text-xs text-white placeholder-gray-500 focus:outline-none focus:border-blue-600"
                required
              />
          </div>

          {/* Total Value */}
          <div className="flex justify-between items-center text-[10px] sm:text-xs">
            <span className="text-gray-400">Total</span>
            <span className="text-white font-medium">{totalValue} USDC</span>
          </div>

          {/* Reserve Button */}
          <button
            type="submit"
            disabled={loading || !publicKey || !size}
            className={`w-full py-2 sm:py-2.5 text-[11px] sm:text-xs font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed touch-manipulation ${
              orderSide === 'buy'
                ? 'bg-green-600 hover:bg-green-700 text-white'
                : 'bg-red-600 hover:bg-red-700 text-white'
            }`}
          >
            {loading ? 'Processing...' : !publicKey ? 'Connect Wallet' : `Reserve ${orderSide === 'buy' ? 'Buy' : 'Sell'}`}
          </button>
        </form>
      ) : (
        <div className="space-y-2 sm:space-y-3">
          {/* Reservation Info Banner */}
          <div className="bg-yellow-900/20 border border-yellow-600/50 rounded-lg p-2">
            <div className="flex items-start gap-2">
              <AlertCircle className="w-3 h-3 sm:w-4 sm:h-4 text-yellow-400 mt-0.5 shrink-0" />
              <div className="flex-1">
                <p className="text-[10px] sm:text-xs font-semibold text-yellow-300 mb-1">Order Reserved!</p>
                <p className="text-[10px] sm:text-xs text-yellow-400">Price locked. Commit within 30 seconds or it expires.</p>
              </div>
            </div>
          </div>

          {/* Countdown Timer */}
          <div className="bg-gray-900 rounded-lg p-2 sm:p-3 border-2 border-blue-500/50">
            <div className="flex items-center justify-between mb-2">
              <Clock className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-blue-400" />
              <span className={`text-lg sm:text-xl font-bold font-mono ${
                timeRemaining < 10000 ? 'text-red-400 animate-pulse' : 'text-blue-400'
              }`}>
                {(timeRemaining / 1000).toFixed(1)}s
              </span>
            </div>
            
            {/* Order Details */}
            <div className="space-y-1 sm:space-y-1.5 text-[10px] sm:text-xs">
              <div className="flex justify-between">
                <span className="text-gray-400">Side</span>
                <span className={`font-bold ${reservedOrder.side === 'buy' ? 'text-green-400' : 'text-red-400'}`}>
                  {reservedOrder.side.toUpperCase()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Price</span>
                <span className="text-white font-medium">${reservedOrder.price.toFixed(2)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-400">Size</span>
                <span className="text-white font-medium">{reservedOrder.size} {coinV2}</span>
              </div>
              <div className="flex justify-between border-t border-gray-700 pt-1.5 sm:pt-2">
                <span className="text-gray-400">Total</span>
                <span className="text-white font-bold">${(reservedOrder.price * reservedOrder.size).toFixed(2)} USDC</span>
              </div>
            </div>
          </div>

          {/* Commit & Cancel Buttons */}
          <div className="flex gap-1.5">
            <button
              onClick={handleCommit}
              disabled={loading}
              className="flex-1 py-2 sm:py-2.5 bg-gradient-to-r from-green-600 to-green-700 hover:from-green-700 hover:to-green-800 text-white font-bold text-[11px] sm:text-xs rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed touch-manipulation"
            >
              {loading ? 'Committing...' : 'Commit Order ✓'}
            </button>
            <button
              onClick={handleCancel}
              disabled={loading}
              className="px-2 sm:px-3 py-2 sm:py-2.5 bg-gray-700 hover:bg-gray-600 text-gray-300 font-semibold text-[11px] sm:text-xs rounded-lg transition-colors disabled:opacity-50 touch-manipulation"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
