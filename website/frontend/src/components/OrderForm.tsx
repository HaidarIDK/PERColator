"use client"

import { useState, useEffect, useRef } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Transaction, Keypair } from '@solana/web3.js';
import { connection, createPlaceOrderInstruction } from '@/lib/solana-client';
import { useToast } from '@/components/ToastProvider';
import { Clock, AlertCircle } from 'lucide-react';

type OrderSide = 'buy' | 'sell';
type OrderType = 'limit' | 'market';

interface OrderFormProps {
  coin: 'SOL' | 'ETH' | 'BTC';
  currentPrice: number;
}

interface ReservedOrder {
  side: OrderSide;
  price: number;
  size: number;
  expiresAt: number;
  receiptKeypair: Keypair;
  transaction: Transaction;
}

export function OrderForm({ coin, currentPrice }: OrderFormProps) {
  const { publicKey, signTransaction } = useWallet();
  const toast = useToast();
  const [orderSide, setOrderSide] = useState<OrderSide>('buy');
  const [orderType, setOrderType] = useState<OrderType>('limit');
  const [price, setPrice] = useState(currentPrice.toString());
  const [size, setSize] = useState('');
  const [loading, setLoading] = useState(false);
  const [reservedOrder, setReservedOrder] = useState<ReservedOrder | null>(null);
  const [timeRemaining, setTimeRemaining] = useState(0);
  const timerRef = useRef<NodeJS.Timeout | null>(null);

  // Countdown timer effect
  useEffect(() => {
    if (reservedOrder) {
      timerRef.current = setInterval(() => {
        const remaining = Math.max(0, reservedOrder.expiresAt - Date.now());
        setTimeRemaining(remaining);
        
        if (remaining === 0) {
          // Auto-cancel expired reservation
          setReservedOrder(null);
          toast.warning('Reservation expired! Please reserve again.');
          if (timerRef.current) clearInterval(timerRef.current);
        }
      }, 100);
      
      return () => {
        if (timerRef.current) clearInterval(timerRef.current);
      };
    }
  }, [reservedOrder, toast]);

  // Step 1: Reserve the order (lock in price)
  const handleReserve = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!publicKey || !signTransaction) {
      toast.warning('Please connect your wallet');
      return;
    }

    setLoading(true);
    try {
      const orderPrice = orderType === 'limit' ? parseFloat(price) : currentPrice;
      const orderSize = parseFloat(size);

      if (orderSize <= 0 || orderPrice <= 0) {
        throw new Error('Invalid price or size');
      }

      console.log('Reserving order:', {
        coin,
        side: orderSide,
        type: orderType,
        price: orderPrice,
        size: orderSize,
      });

      // Build transaction (includes receipt account creation + order execution)
      const { instructions, receiptKeypair } = await createPlaceOrderInstruction({
        userPubkey: publicKey,
        coin,
        side: orderSide,
        price: orderPrice,
        size: orderSize,
      });

      const transaction = new Transaction().add(...instructions);
      transaction.feePayer = publicKey;
      transaction.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      
      // The receipt account needs to sign for its creation
      transaction.partialSign(receiptKeypair);

      // Create reservation (valid for 30 seconds)
      const reservation: ReservedOrder = {
        side: orderSide,
        price: orderPrice,
        size: orderSize,
        expiresAt: Date.now() + 30000, // 30 seconds
        receiptKeypair,
        transaction,
      };

      setReservedOrder(reservation);
      toast.success(`Order reserved! You have 30 seconds to commit.`);
      
    } catch (error: any) {
      console.error('Failed to reserve order:', error);
      toast.error(`Failed to reserve: ${error.message || error.toString()}`);
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
      
      console.log('Order committed, waiting for confirmation...');
      await connection.confirmTransaction(signature, 'confirmed');

      console.log('Order confirmed:', signature);
      toast.success(`${reservedOrder.side.toUpperCase()} order committed! ${reservedOrder.size} ${coin} @ $${reservedOrder.price.toFixed(2)}`);
      
      // Reset form
      setReservedOrder(null);
      setSize('');
      setPrice(currentPrice.toString());
      if (timerRef.current) clearInterval(timerRef.current);
      
    } catch (error: any) {
      console.error('Failed to commit order:', error);
      toast.error(`Failed to commit: ${error.message || error.toString()}`);
    } finally {
      setLoading(false);
    }
  };

  // Cancel reservation
  const handleCancel = () => {
    setReservedOrder(null);
    if (timerRef.current) clearInterval(timerRef.current);
    toast.info('Reservation cancelled');
  };

  const totalValue = orderType === 'limit' 
    ? (parseFloat(price) * parseFloat(size || '0')).toFixed(2)
    : (currentPrice * parseFloat(size || '0')).toFixed(2);

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
      {!reservedOrder ? (
        <form onSubmit={handleReserve} className="space-y-4">
          {/* Order Side */}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setOrderSide('buy')}
              className={`flex-1 py-2 text-sm font-semibold rounded transition-colors ${
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
              className={`flex-1 py-2 text-sm font-semibold rounded transition-colors ${
                orderSide === 'sell'
                  ? 'bg-red-600 text-white'
                  : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
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
              className={`flex-1 py-1.5 text-xs font-medium rounded transition-colors ${
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
              className={`flex-1 py-1.5 text-xs font-medium rounded transition-colors ${
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
              <label className="text-xs text-gray-400 mb-1 block">Price (USDC)</label>
              <input
                type="number"
                step="0.01"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                placeholder={currentPrice.toFixed(2)}
                className="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-600"
                required
              />
            </div>
          )}

          {/* Size Input */}
          <div>
            <label className="text-xs text-gray-400 mb-1 block">Size ({coin})</label>
            <input
              type="number"
              step="0.001"
              min="0"
              value={size}
              onChange={(e) => setSize(e.target.value)}
              placeholder="0.000"
              className="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-600"
              required
            />
          </div>

          {/* Total Value */}
          <div className="flex justify-between items-center text-xs">
            <span className="text-gray-400">Total</span>
            <span className="text-white font-medium">{totalValue} USDC</span>
          </div>

          {/* Reserve Button */}
          <button
            type="submit"
            disabled={loading || !publicKey || !size}
            className={`w-full py-3 text-sm font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
              orderSide === 'buy'
                ? 'bg-green-600 hover:bg-green-700 text-white'
                : 'bg-red-600 hover:bg-red-700 text-white'
            }`}
          >
            {loading ? 'Processing...' : !publicKey ? 'Connect Wallet' : `Reserve ${orderSide === 'buy' ? 'Buy' : 'Sell'}`}
          </button>
        </form>
      ) : (
        <div className="space-y-4">
          {/* Reservation Info Banner */}
          <div className="bg-yellow-900/20 border border-yellow-600/50 rounded-lg p-3">
            <div className="flex items-start gap-2">
              <AlertCircle className="w-4 h-4 text-yellow-400 mt-0.5 shrink-0" />
              <div className="flex-1">
                <p className="text-xs font-semibold text-yellow-300 mb-1">Order Reserved!</p>
                <p className="text-xs text-yellow-400">Price locked. Commit within 30 seconds or it expires.</p>
              </div>
            </div>
          </div>

          {/* Countdown Timer */}
          <div className="bg-gray-900 rounded-lg p-4 border-2 border-blue-500/50">
            <div className="flex items-center justify-between mb-3">
              <Clock className="w-5 h-5 text-blue-400" />
              <span className={`text-2xl font-bold font-mono ${
                timeRemaining < 10000 ? 'text-red-400 animate-pulse' : 'text-blue-400'
              }`}>
                {(timeRemaining / 1000).toFixed(1)}s
              </span>
            </div>
            
            {/* Order Details */}
            <div className="space-y-2 text-sm">
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
                <span className="text-white font-medium">{reservedOrder.size} {coin}</span>
              </div>
              <div className="flex justify-between border-t border-gray-700 pt-2">
                <span className="text-gray-400">Total</span>
                <span className="text-white font-bold">${(reservedOrder.price * reservedOrder.size).toFixed(2)} USDC</span>
              </div>
            </div>
          </div>

          {/* Commit & Cancel Buttons */}
          <div className="flex gap-2">
            <button
              onClick={handleCommit}
              disabled={loading}
              className="flex-1 py-3 bg-gradient-to-r from-green-600 to-green-700 hover:from-green-700 hover:to-green-800 text-white font-bold rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Committing...' : 'Commit Order ✓'}
            </button>
            <button
              onClick={handleCancel}
              disabled={loading}
              className="px-4 py-3 bg-gray-700 hover:bg-gray-600 text-gray-300 font-semibold rounded-lg transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

