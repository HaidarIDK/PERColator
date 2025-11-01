"use client"

import { useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Transaction } from '@solana/web3.js';
import { connection, createPlaceOrderInstruction } from '@/lib/solana-client';
import { useToast } from '@/components/ToastProvider';

type OrderSide = 'buy' | 'sell';
type OrderType = 'limit' | 'market';

interface OrderFormProps {
  coin: 'SOL' | 'ETH' | 'BTC';
  currentPrice: number;
}

export function OrderForm({ coin, currentPrice }: OrderFormProps) {
  const { publicKey, signTransaction } = useWallet();
  const toast = useToast();
  const [orderSide, setOrderSide] = useState<OrderSide>('buy');
  const [orderType, setOrderType] = useState<OrderType>('limit');
  const [price, setPrice] = useState(currentPrice.toString());
  const [size, setSize] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
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

      console.log('Placing order:', {
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

      // Sign and send
      const signed = await signTransaction(transaction);
      const signature = await connection.sendRawTransaction(signed.serialize());
      
      console.log('Order placed, waiting for confirmation...');
      await connection.confirmTransaction(signature, 'confirmed');

      console.log('Order confirmed:', signature);
      toast.success(`${orderSide.toUpperCase()} order placed successfully! ${orderSize} ${coin} @ $${orderPrice.toFixed(2)}`);
      
      // Reset form
      setSize('');
    } catch (error: any) {
      console.error('Failed to place order:', error);
      
      let errorMessage = 'Unknown error';
      if (error.message) {
        errorMessage = error.message;
      } else if (error.toString) {
        errorMessage = error.toString();
      }
      
      toast.error(`Failed to place order: ${errorMessage}`);
    } finally {
      setLoading(false);
    }
  };

  const totalValue = orderType === 'limit' 
    ? (parseFloat(price) * parseFloat(size || '0')).toFixed(2)
    : (currentPrice * parseFloat(size || '0')).toFixed(2);

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
      <form onSubmit={handleSubmit} className="space-y-4">
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

        {/* Submit Button */}
        <button
          type="submit"
          disabled={loading || !publicKey || !size}
          className={`w-full py-3 text-sm font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
            orderSide === 'buy'
              ? 'bg-green-600 hover:bg-green-700 text-white'
              : 'bg-red-600 hover:bg-red-700 text-white'
          }`}
        >
          {loading ? 'Processing...' : !publicKey ? 'Connect Wallet' : `${orderSide === 'buy' ? 'Buy' : 'Sell'} ${coin}`}
        </button>
      </form>
    </div>
  );
}

