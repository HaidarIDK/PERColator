"use client"

import { useEffect, useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
import { percolatorClient, getPortfolioRentCost, type PortfolioSize } from '@/lib/solana-client';
import { derivePortfolioAddress } from '@/lib/program-config';
import { useToast } from '@/components/ToastProvider';

export function PortfolioPanel() {
  const { publicKey, sendTransaction } = useWallet();
  const toast = useToast();
  const [portfolioExists, setPortfolioExists] = useState(false);
  const [portfolioAddress, setPortfolioAddress] = useState<PublicKey | null>(null);
  const [walletBalance, setWalletBalance] = useState<number | null>(null);
  const [portfolioBalance, setPortfolioBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [depositAmount, setDepositAmount] = useState('');
  const [selectedSize, setSelectedSize] = useState<PortfolioSize>('TEST');
  const [rentCost, setRentCost] = useState<number | null>(null);

  // Load portfolio info
  useEffect(() => {
    if (!publicKey) {
      setPortfolioExists(false);
      setPortfolioAddress(null);
      setWalletBalance(null);
      setPortfolioBalance(null);
      return;
    }

    const loadPortfolio = async () => {
      try {
        // Derive portfolio address
        const addr = await derivePortfolioAddress(publicKey);
        setPortfolioAddress(addr);

        // Check if portfolio exists
        const exists = await percolatorClient.portfolioExists(publicKey);
        setPortfolioExists(exists);

        // Get wallet balance
        const balance = await percolatorClient.getBalance(publicKey);
        setWalletBalance(balance);

        // Get portfolio info if exists
        if (exists) {
          const portfolio = await percolatorClient.getPortfolio(publicKey);
          if (portfolio) {
            setPortfolioBalance(portfolio.balance / LAMPORTS_PER_SOL);
          }
        }
      } catch (error) {
        console.error('Failed to load portfolio:', error);
      }
    };

    loadPortfolio();
    const interval = setInterval(loadPortfolio, 5000); // Refresh every 5s

    return () => clearInterval(interval);
  }, [publicKey]);

  // Load rent cost when size changes
  useEffect(() => {
    getPortfolioRentCost(selectedSize).then(setRentCost);
  }, [selectedSize]);

  // Initialize portfolio
  const handleInitPortfolio = async () => {
    if (!publicKey || !sendTransaction) {
      toast.warning('Please connect your wallet first');
      return;
    }

    setLoading(true);
    try {
      console.log('Creating portfolio with size:', selectedSize);
      console.log('Wallet connected:', publicKey.toString());
      
      const { Transaction } = await import('@solana/web3.js');
      const instructions = await percolatorClient.createInitPortfolioInstructions(publicKey, selectedSize);
      
      const transaction = new Transaction();
      instructions.forEach(ix => transaction.add(ix));
      
      console.log('Sending transaction...');
      
      let signature: string;
      try {
        signature = await sendTransaction(transaction, percolatorClient['connection'], {
          skipPreflight: false,
          preflightCommitment: 'confirmed',
        });
      } catch (txError: any) {
        console.error('Full transaction error:', txError);
        console.error('Error name:', txError?.name);
        console.error('Error message:', txError?.message);
        console.error('Error code:', txError?.code);
        console.error('Error logs:', txError?.logs);
        
        if (txError?.message?.includes('User rejected')) {
          throw new Error('Transaction rejected by user');
        }
        
        throw new Error(txError?.message || txError?.toString() || 'Failed to send transaction. Please approve in Phantom.');
      }
      
      console.log('Transaction sent:', signature);
      console.log('Waiting for confirmation...');
      
      await percolatorClient['connection'].confirmTransaction(signature, 'confirmed');
      
      console.log('Portfolio initialized successfully!');
      toast.success(`Portfolio created successfully! Size: ${selectedSize}`);
      
      // Refresh
      setTimeout(async () => {
        const exists = await percolatorClient.portfolioExists(publicKey);
        setPortfolioExists(exists);
        if (exists) {
          const portfolio = await percolatorClient.getPortfolio(publicKey);
          if (portfolio) {
            setPortfolioBalance(portfolio.balance / LAMPORTS_PER_SOL);
          }
        }
      }, 2000);
    } catch (error: any) {
      console.error('Failed to initialize portfolio:', error);
      
      let errorMsg = 'Unknown error';
      if (error?.message) {
        errorMsg = error.message;
      } else if (error?.toString) {
        errorMsg = error.toString();
      }
      
      toast.error('Failed to initialize portfolio: ' + errorMsg);
    } finally {
      setLoading(false);
    }
  };

  // Deposit funds
  const handleDeposit = async () => {
    if (!publicKey || !sendTransaction || !depositAmount) {
      if (!publicKey) toast.warning('Please connect your wallet first');
      return;
    }

    const amount = parseFloat(depositAmount);
    if (isNaN(amount) || amount <= 0) {
      toast.warning('Please enter a valid amount');
      return;
    }

    // Check maximum deposit limit (< 100M lamports = < 0.1 SOL)
    const MAX_DEPOSIT_SOL = 0.099; // Stay slightly under the limit
    if (amount > MAX_DEPOSIT_SOL) {
      toast.warning(`Maximum deposit is ${MAX_DEPOSIT_SOL} SOL per transaction (program safety limit)`);
      return;
    }

    setLoading(true);
    try {
      const lamports = Math.floor(amount * LAMPORTS_PER_SOL);
      console.log('Depositing', amount, 'SOL (', lamports, 'lamports)');
      
      const { Transaction } = await import('@solana/web3.js');
      const instruction = await percolatorClient.createDepositInstruction(publicKey, lamports);
      const transaction = new Transaction().add(instruction);
      
      const signature = await sendTransaction(transaction, percolatorClient['connection'], {
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      });
      
      console.log('Deposit transaction sent:', signature);
      await percolatorClient['connection'].confirmTransaction(signature, 'confirmed');
      console.log('Deposit confirmed!');
      
      toast.success(`Deposited ${amount} SOL successfully!`);
      setDepositAmount('');
      
      // Refresh balances
      setTimeout(async () => {
        const balance = await percolatorClient.getBalance(publicKey);
        setWalletBalance(balance);
        const portfolio = await percolatorClient.getPortfolio(publicKey);
        if (portfolio) {
          setPortfolioBalance(portfolio.balance / LAMPORTS_PER_SOL);
        }
      }, 2000);
    } catch (error) {
      console.error('Failed to deposit:', error);
      const errorMsg = error instanceof Error ? error.message : String(error);
      toast.error(`Failed to deposit: ${errorMsg}. Try a smaller amount (max 0.099 SOL per tx)`);
    } finally {
      setLoading(false);
    }
  };

  if (!publicKey) {
    return (
      <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
        <p className="text-sm text-gray-400 text-center py-8">
          Connect your wallet to manage portfolio
        </p>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 space-y-4">
      <div>
        <h3 className="text-sm font-semibold text-white mb-3">Portfolio</h3>
        
        {/* Portfolio Address */}
        {portfolioAddress && (
          <div className="mb-3 p-2 bg-gray-900 rounded border border-gray-700">
            <div className="text-[10px] text-gray-400 mb-1">Portfolio Account</div>
            <div className="text-xs font-mono text-blue-400 break-all">
              {portfolioAddress.toString()}
            </div>
          </div>
        )}
        
        {/* Wallet Balance */}
        <div className="flex justify-between items-center mb-2">
          <span className="text-xs text-gray-400">Wallet Balance</span>
          <span className="text-sm font-medium text-white">
            {walletBalance !== null ? `${walletBalance.toFixed(4)} SOL` : '...'}
          </span>
        </div>

        {/* Portfolio Status */}
        {!portfolioExists ? (
          <div className="mt-4 space-y-3">
            <p className="text-xs text-gray-400 mb-2">No portfolio account found</p>
            
            {/* Portfolio Size Selector */}
            <div>
              <label className="text-xs text-gray-400 mb-1.5 block">Portfolio Size</label>
              <div className="space-y-2">
                {/* Test Size */}
                <button
                  type="button"
                  onClick={() => setSelectedSize('TEST')}
                  className={`w-full p-2.5 rounded border transition-all text-left ${
                    selectedSize === 'TEST'
                      ? 'border-blue-500 bg-blue-500/10'
                      : 'border-gray-700 bg-gray-900 hover:border-gray-600'
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-medium text-white">Test Size</span>
                    <span className="text-xs text-blue-400">Recommended</span>
                  </div>
                  <div className="text-xs text-gray-400 mb-1">
                    10 KB - Perfect for testing and small portfolios
                  </div>
                  <div className="text-xs font-medium text-green-500">
                    Cost: ~{rentCost !== null && selectedSize === 'TEST' ? rentCost.toFixed(4) : '0.07'} SOL
                  </div>
                </button>

                {/* Full Size */}
                <button
                  type="button"
                  onClick={() => setSelectedSize('FULL')}
                  className={`w-full p-2.5 rounded border transition-all text-left ${
                    selectedSize === 'FULL'
                      ? 'border-blue-500 bg-blue-500/10'
                      : 'border-gray-700 bg-gray-900 hover:border-gray-600'
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-medium text-white">Full Size (v1)</span>
                    <span className="text-xs text-purple-400">Production</span>
                  </div>
                  <div className="text-xs text-gray-400 mb-1">
                    500 KB - Maximum capacity for large portfolios
                  </div>
                  <div className="text-xs font-medium text-yellow-500">
                    Cost: ~{rentCost !== null && selectedSize === 'FULL' ? rentCost.toFixed(2) : '3.56'} SOL
                  </div>
                </button>
              </div>
            </div>

            {/* Create Button */}
            <button
              onClick={handleInitPortfolio}
              disabled={loading || (walletBalance !== null && rentCost !== null && walletBalance < rentCost)}
              className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Creating...' : `Create ${selectedSize === 'TEST' ? 'Test' : 'Full'} Portfolio`}
            </button>
            
            {walletBalance !== null && rentCost !== null && walletBalance < rentCost && (
              <p className="text-xs text-red-400 text-center">
                Insufficient SOL. Need {rentCost.toFixed(4)} SOL, have {walletBalance.toFixed(4)} SOL
              </p>
            )}
          </div>
        ) : (
          <div>
            {/* Portfolio Balance */}
            <div className="flex justify-between items-center mb-4">
              <span className="text-xs text-gray-400">Portfolio Balance</span>
              <span className="text-sm font-medium text-green-500">
                {portfolioBalance !== null ? `${portfolioBalance.toFixed(4)} SOL` : '...'}
              </span>
            </div>

            {/* Deposit Form */}
            <div>
              <label className="text-xs text-gray-400 mb-1 block">
                Deposit Amount (SOL)
                <span className="text-[10px] text-gray-500 ml-2">(max 0.099 per tx)</span>
              </label>
              <div className="flex gap-2">
                <input
                  type="number"
                  step="0.01"
                  min="0"
                  max="0.099"
                  value={depositAmount}
                  onChange={(e) => setDepositAmount(e.target.value)}
                  placeholder="0.05"
                  className="flex-1 px-3 py-2 bg-gray-900 border border-gray-700 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-600"
                />
                <button
                  onClick={handleDeposit}
                  disabled={loading || !depositAmount}
                  className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Deposit
                </button>
              </div>
              <p className="text-[10px] text-gray-500 mt-1">
                Program safety limit. Deposit multiple times if needed.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

