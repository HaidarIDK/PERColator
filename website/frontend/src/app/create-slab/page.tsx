"use client"

import { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Connection, Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction } from '@solana/web3.js';
import { NETWORK, RPC_ENDPOINT, SLAB_PROGRAM_ID, ROUTER_PROGRAM_ID } from '@/lib/program-config';
import { useToast } from '@/components/ToastProvider';
import Link from 'next/link';
import { ArrowLeft, Plus, DollarSign, TrendingUp, Layers } from 'lucide-react';

export default function CreateSlabPage() {
  const { publicKey, signTransaction } = useWallet();
  const { success, error, warning, info } = useToast();
  
  // Slab creation state
  const [baseCurrency, setBaseCurrency] = useState<'SOL' | 'ETH' | 'BTC'>('SOL');
  const [markPrice, setMarkPrice] = useState('');
  const [tickSize, setTickSize] = useState('1.0');
  const [lotSize, setLotSize] = useState('1.0');
  const [creatingSlab, setCreatingSlab] = useState(false);
  
  // Available trading pairs
  const availablePairs = [
    { base: 'SOL', name: 'Solana', suggestedPrice: '186', tickSize: '1.0', lotSize: '1.0' },
    { base: 'ETH', name: 'Ethereum', suggestedPrice: '2800', tickSize: '10.0', lotSize: '0.1' },
    { base: 'BTC', name: 'Bitcoin', suggestedPrice: '43000', tickSize: '100.0', lotSize: '0.01' },
  ] as const;
  
  // LP state
  const [selectedSlab, setSelectedSlab] = useState('7pyCSG18qBXipgUCApPxBo48T3a1M7arhrf1AbkMtAFL');
  const [lpAmount, setLpAmount] = useState('');
  const [lpPrice, setLpPrice] = useState('');
  const [providingLp, setProvidingLp] = useState(false);
  
  // User's slabs
  const [userSlabs, setUserSlabs] = useState<any[]>([]);
  const [loadingSlabs, setLoadingSlabs] = useState(false);

  // Load user's slabs on wallet connect
  useEffect(() => {
    if (publicKey) {
      loadUserSlabs();
    } else {
      setUserSlabs([]);
    }
  }, [publicKey]);

  // Fetch all available markets
  const loadUserSlabs = async () => {
    if (!publicKey) return;
    
    setLoadingSlabs(true);
    try {
      const connection = new Connection(RPC_ENDPOINT, 'confirmed');
      
      console.log('Loading available markets...');
      
      // Get ALL slabs
      const accounts = await connection.getProgramAccounts(SLAB_PROGRAM_ID, {
        filters: [
          { dataSize: 65536 }, // Slab size
        ],
      });
      
      console.log(`Found ${accounts.length} market(s)`);
      
      // Parse all markets
      const slabs = accounts
        .map((account) => {
          try {
            const data = account.account.data;
            
            // Try reading with OLD offsets (1-byte discriminator) first
            let markPrice = 0;
            let tickSize = 0;
            let lotSize = 0;
            let owner = 'Unknown';
            
            try {
              // OLD format offsets (buggy slabs)
              const markPriceRaw = data.readBigInt64LE(1);
              const tickSizeRaw = data.readBigUInt64LE(93);
              const lotSizeRaw = data.readBigUInt64LE(109);
              const ownerBytes = data.slice(25, 57);
              const ownerPubkey = new PublicKey(ownerBytes);
              
              markPrice = Number(markPriceRaw) / 1_000_000;
              tickSize = Number(tickSizeRaw) / 1_000_000;
              lotSize = Number(lotSizeRaw) / 1_000_000;
              owner = ownerPubkey.toBase58();
              
              // If numbers look wrong, try NEW format
              if (markPrice < 0 || markPrice > 100000 || tickSize > 1000 || lotSize > 1000) {
                throw new Error('Invalid OLD format, trying NEW');
              }
            } catch (e) {
              // NEW format offsets (8-byte discriminator)
              const markPriceRaw = data.readBigInt64LE(8);
              const tickSizeRaw = data.readBigUInt64LE(100);
              const lotSizeRaw = data.readBigUInt64LE(116);
              const ownerBytes = data.slice(32, 64);
              const ownerPubkey = new PublicKey(ownerBytes);
              
              markPrice = Number(markPriceRaw) / 1_000_000;
              tickSize = Number(tickSizeRaw) / 1_000_000;
              lotSize = Number(lotSizeRaw) / 1_000_000;
              owner = ownerPubkey.toBase58();
            }
            
            // Determine pair based on price range
            let pair = 'UNKNOWN/USDC';
            if (markPrice < 1000) pair = 'SOL/USDC';
            else if (markPrice < 10000) pair = 'ETH/USDC';
            else pair = 'BTC/USDC';
            
            // Check if data looks valid
            const isBroken = (
              tickSize > 1000 || 
              lotSize > 1000 || 
              markPrice < 0 || 
              markPrice > 100000
            );
            
            return {
              address: account.pubkey.toBase58(),
              pair,
              markPrice,
              tickSize,
              lotSize,
              owner,
              isBroken,
            };
          } catch (err) {
            console.error('Error parsing market:', account.pubkey.toBase58(), err);
            return null;
          }
        })
        .filter((slab) => slab !== null);
      
      setUserSlabs(slabs);
      if (slabs.length > 0) {
        const workingCount = slabs.filter(s => !s.isBroken).length;
        const brokenCount = slabs.filter(s => s.isBroken).length;
        if (brokenCount > 0) {
          warning(`Found ${slabs.length} markets: ${workingCount} working, ${brokenCount} broken`);
        } else {
          info(`Loaded ${slabs.length} trading market${slabs.length > 1 ? 's' : ''}`);
        }
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      console.error('Load markets failed:', errorMessage.split('\n')[0]);
    } finally {
      setLoadingSlabs(false);
    }
  };

  // Handle pair selection
  const handlePairSelect = (base: 'SOL' | 'ETH' | 'BTC') => {
    const pair = availablePairs.find(p => p.base === base);
    if (pair) {
      setBaseCurrency(base);
      setMarkPrice(pair.suggestedPrice);
      setTickSize(pair.tickSize);
      setLotSize(pair.lotSize);
    }
  };

  const handleCreateSlab = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!publicKey || !signTransaction) {
      warning('Please connect your wallet first');
      return;
    }

    setCreatingSlab(true);
    
    try {
      info('Creating new slab...');
      
      const connection = new Connection(RPC_ENDPOINT, 'confirmed');
      const slabKeypair = Keypair.generate();
      
      const markPriceScaled = Math.floor(parseFloat(markPrice) * 1_000_000);
      const tickSizeScaled = Math.floor(parseFloat(tickSize) * 1_000_000);
      const lotSizeScaled = Math.floor(parseFloat(lotSize) * 1_000_000);
      
      const slabSize = 65536;
      const lamports = await connection.getMinimumBalanceForRentExemption(slabSize);
      
      const createAccountIx = SystemProgram.createAccount({
        fromPubkey: publicKey,
        newAccountPubkey: slabKeypair.publicKey,
        lamports,
        space: slabSize,
        programId: SLAB_PROGRAM_ID,
      });
      
      const instructionData = Buffer.alloc(140);
      
      // Discriminator is 8 bytes (u64), not 1 byte!
      const discriminatorView = new DataView(instructionData.buffer, instructionData.byteOffset, 8);
      discriminatorView.setBigUint64(0, BigInt(0), true); // Initialize discriminator
      
      const markPriceView = new DataView(instructionData.buffer, instructionData.byteOffset + 8, 8);
      markPriceView.setBigInt64(0, BigInt(markPriceScaled), true);
      
      const feesView = new DataView(instructionData.buffer, instructionData.byteOffset + 16, 8);
      feesView.setBigUint64(0, BigInt(1000000), true);
      
      const contractSizeView = new DataView(instructionData.buffer, instructionData.byteOffset + 24, 8);
      contractSizeView.setBigUint64(0, BigInt(1000000), true);
      
      // LP owner at offset 32 (8+8+8+8)
      publicKey.toBuffer().copy(instructionData, 32);
      console.log('Setting LP owner to:', publicKey.toBase58());
      
      // Router ID at offset 64 (32+32)
      ROUTER_PROGRAM_ID.toBuffer().copy(instructionData, 64);
      console.log('Setting router ID to:', ROUTER_PROGRAM_ID.toBase58());
      
      // Instrument at offset 96 (64+32)
      instructionData.writeUInt32LE(0, 96);
      
      // Tick size at offset 100 (96+4)
      const tickSizeView = new DataView(instructionData.buffer, instructionData.byteOffset + 100, 8);
      tickSizeView.setBigUint64(0, BigInt(tickSizeScaled), true);
      
      // Tick size exponent at offset 108
      const tickExpView = new DataView(instructionData.buffer, instructionData.byteOffset + 108, 8);
      tickExpView.setBigInt64(0, BigInt(0), true);
      
      // Lot size at offset 116
      const lotSizeView = new DataView(instructionData.buffer, instructionData.byteOffset + 116, 8);
      lotSizeView.setBigUint64(0, BigInt(lotSizeScaled), true);
      
      // Lot size exponent at offset 124
      const lotExpView = new DataView(instructionData.buffer, instructionData.byteOffset + 124, 8);
      lotExpView.setBigInt64(0, BigInt(0), true);
      
      // Min order size at offset 132
      const minOrderView = new DataView(instructionData.buffer, instructionData.byteOffset + 132, 8);
      minOrderView.setBigUint64(0, BigInt(lotSizeScaled), true);
      
      const initializeSlabIx = new TransactionInstruction({
        keys: [
          { pubkey: slabKeypair.publicKey, isSigner: true, isWritable: true },
          { pubkey: publicKey, isSigner: true, isWritable: false },
        ],
        programId: SLAB_PROGRAM_ID,
        data: instructionData,
      });
      
      const transaction = new Transaction().add(createAccountIx, initializeSlabIx);
      transaction.feePayer = publicKey;
      
      // IMPORTANT: Get fresh blockhash to avoid "already processed" error
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed');
      transaction.recentBlockhash = blockhash;
      
      transaction.partialSign(slabKeypair);
      const signedTx = await signTransaction(transaction);
      
      const signature = await connection.sendRawTransaction(signedTx.serialize(), {
        skipPreflight: false,
        maxRetries: 3,
      });
      
      await connection.confirmTransaction({
        signature,
        blockhash,
        lastValidBlockHeight,
      }, 'confirmed');
      
      success(`${baseCurrency}/USDC slab created! Address: ${slabKeypair.publicKey.toBase58().slice(0, 8)}...`);
      info('You can now provide liquidity to your new slab!');
      setSelectedSlab(slabKeypair.publicKey.toBase58());
      
      // Reload user's slabs
      await loadUserSlabs();
      
    } catch (err) {
      // Parse error message for user-friendly display
      const errorMessage = err instanceof Error ? err.message : String(err);
      console.error('Create slab failed:', errorMessage.split('\n')[0]); // Log first line only
      
      if (errorMessage.includes('insufficient lamports') || errorMessage.includes('insufficient funds') || errorMessage.includes('Transfer: insufficient')) {
        error('Insufficient funds. You need at least 0.5 SOL to create a slab.');
      } else if (errorMessage.includes('User rejected') || errorMessage.includes('not been authorized') || errorMessage.includes('WalletSignTransactionError')) {
        warning('Transaction cancelled by user');
      } else if (errorMessage.includes('already been processed')) {
        warning('Transaction already processed. Please wait a moment and try again.');
      } else if (errorMessage.includes('Blockhash not found')) {
        error('Transaction expired. Please try again.');
      } else {
        // Show shortened error for other cases
        const shortError = errorMessage.split('.')[0]; // First sentence only
        error(`Failed to create slab: ${shortError}`);
      }
    } finally {
      setCreatingSlab(false);
    }
  };

  const handleProvideLiquidity = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!publicKey || !signTransaction) {
      warning('Please connect your wallet first');
      return;
    }

    setProvidingLp(true);
    
    try {
      info('Providing liquidity...');
      
      const connection = new Connection(RPC_ENDPOINT, 'confirmed');
      
      const amountScaled = Math.floor(parseFloat(lpAmount) * 1_000_000);
      const priceScaled = Math.floor(parseFloat(lpPrice) * 1_000_000);
      const sideNum = 0; // 0 = Buy side LP
      
      const instructionData = Buffer.alloc(18);
      instructionData.writeUInt8(3, 0); // PlaceOrder discriminator (for LP as maker)
      instructionData.writeUInt8(sideNum, 1);
      
      const priceView = new DataView(instructionData.buffer, instructionData.byteOffset + 2, 8);
      priceView.setBigInt64(0, BigInt(priceScaled), true);
      
      const qtyView = new DataView(instructionData.buffer, instructionData.byteOffset + 10, 8);
      qtyView.setBigInt64(0, BigInt(amountScaled), true);
      
      const slabPubkey = new PublicKey(selectedSlab);
      
      const placeOrderIx = new TransactionInstruction({
        keys: [
          { pubkey: slabPubkey, isSigner: false, isWritable: true },
          { pubkey: publicKey, isSigner: true, isWritable: false },
        ],
        programId: SLAB_PROGRAM_ID,
        data: instructionData,
      });
      
      const transaction = new Transaction().add(placeOrderIx);
      transaction.feePayer = publicKey;
      transaction.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
      
      const signedTx = await signTransaction(transaction);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction(signature, 'confirmed');
      
      success(`Liquidity provided: ${lpAmount} SOL at $${lpPrice}`);
      
      setLpAmount('');
      setLpPrice('');
      
    } catch (err) {
      // Parse error message for user-friendly display
      const errorMessage = err instanceof Error ? err.message : String(err);
      console.error('Provide liquidity failed:', errorMessage.split('\n')[0]); // Log first line only
      
      if (errorMessage.includes('insufficient lamports') || errorMessage.includes('insufficient funds') || errorMessage.includes('Transfer: insufficient')) {
        error('Insufficient funds to provide liquidity');
      } else if (errorMessage.includes('User rejected') || errorMessage.includes('not been authorized') || errorMessage.includes('WalletSignTransactionError')) {
        warning('Transaction cancelled by user');
      } else if (errorMessage.includes('already been processed')) {
        warning('Transaction already processed. Please wait a moment and try again.');
      } else if (errorMessage.includes('Invalid public key')) {
        error('Invalid slab address. Please check the address.');
      } else if (errorMessage.includes('not aligned')) {
        error('Invalid amount or price. Must align with tick/lot sizes.');
      } else {
        // Show shortened error
        const shortError = errorMessage.split('.')[0];
        error(`Failed: ${shortError}`);
      }
    } finally {
      setProvidingLp(false);
    }
  };

  return (
    <div className="min-h-screen bg-black text-white">
      <header className="border-b border-gray-800 bg-gray-900">
        <div className="px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Link href="/dashboard">
              <button className="px-3 py-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 border border-[#B8B8FF]/30 text-[#B8B8FF] text-sm font-bold flex items-center gap-2">
                <ArrowLeft className="w-4 h-4" />
                Dashboard
              </button>
            </Link>
            <h1 className="text-xl font-bold bg-gradient-to-r from-blue-500 to-purple-600 bg-clip-text text-transparent">
              Create Slab & LP
            </h1>
          </div>
        </div>
      </header>

      <main className="max-w-6xl mx-auto px-4 py-8">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Create Slab */}
          <div className="bg-gray-900 rounded-xl border border-gray-700 p-6">
            <div className="flex items-center gap-3 mb-6">
              <Plus className="w-6 h-6 text-blue-400" />
              <h2 className="text-xl font-bold">Create New Slab</h2>
            </div>

            <form onSubmit={handleCreateSlab} className="space-y-4">
              {/* Trading Pair Selector */}
              <div>
                <label className="text-xs text-gray-400 mb-2 block">Select Trading Pair</label>
                <div className="grid grid-cols-3 gap-2">
                  {availablePairs.map((pair) => (
                    <button
                      key={pair.base}
                      type="button"
                      onClick={() => handlePairSelect(pair.base)}
                      className={`py-3 px-2 rounded-lg font-bold text-sm transition-all ${
                        baseCurrency === pair.base
                          ? 'bg-blue-600 text-white border-2 border-blue-400'
                          : 'bg-gray-800 text-gray-300 border border-gray-700 hover:bg-gray-700'
                      }`}
                    >
                      <div className="text-xs opacity-70 mb-1">{pair.name}</div>
                      <div>{pair.base}/USDC</div>
                    </button>
                  ))}
                </div>
                <p className="text-xs text-gray-500 mt-2">
                  Only these 3 pairs are available for now
                </p>
              </div>

              <div>
                <label className="text-xs text-gray-400 mb-1 block">Mark Price (USD)</label>
                <input
                  type="number"
                  step="0.01"
                  value={markPrice}
                  onChange={(e) => setMarkPrice(e.target.value)}
                  placeholder={availablePairs.find(p => p.base === baseCurrency)?.suggestedPrice}
                  className="w-full px-3 py-2 bg-black border border-gray-700 rounded-lg text-white"
                  required
                />
                <p className="text-xs text-gray-500 mt-1">Starting price for your {baseCurrency}/USDC slab</p>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs text-gray-400 mb-1 block">Tick Size (Price Step)</label>
                  <input
                    type="number"
                    step="0.01"
                    value={tickSize}
                    onChange={(e) => setTickSize(e.target.value)}
                    className="w-full px-3 py-2 bg-black border border-gray-700 rounded-lg text-white"
                    required
                  />
                  <p className="text-xs text-gray-500 mt-1">Min price change</p>
                </div>
                <div>
                  <label className="text-xs text-gray-400 mb-1 block">Lot Size (Qty Step)</label>
                  <input
                    type="number"
                    step="0.01"
                    value={lotSize}
                    onChange={(e) => setLotSize(e.target.value)}
                    className="w-full px-3 py-2 bg-black border border-gray-700 rounded-lg text-white"
                    required
                  />
                  <p className="text-xs text-gray-500 mt-1">Min quantity</p>
                </div>
              </div>

              <button
                type="submit"
                disabled={creatingSlab || !publicKey}
                className="w-full py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 text-white font-bold rounded-lg"
              >
                {creatingSlab ? 'Creating...' : 'Create Slab (~0.46 SOL)'}
              </button>
            </form>
          </div>

          {/* Provide Liquidity */}
          <div className="bg-gray-900 rounded-xl border border-gray-700 p-6">
            <div className="flex items-center gap-3 mb-6">
              <Layers className="w-6 h-6 text-green-400" />
              <h2 className="text-xl font-bold">Provide Liquidity</h2>
            </div>

            <form onSubmit={handleProvideLiquidity} className="space-y-4">
              <div>
                <label className="text-xs text-gray-400 mb-1 block">Slab Address</label>
                <input
                  type="text"
                  value={selectedSlab}
                  onChange={(e) => setSelectedSlab(e.target.value)}
                  placeholder="Slab address"
                  className="w-full px-3 py-2 bg-black border border-gray-700 rounded-lg text-white font-mono text-xs"
                />
              </div>

              <div>
                <label className="text-xs text-gray-400 mb-1 block">Amount (SOL)</label>
                <input
                  type="number"
                  step="0.01"
                  value={lpAmount}
                  onChange={(e) => setLpAmount(e.target.value)}
                  placeholder="10.0"
                  className="w-full px-3 py-2 bg-black border border-gray-700 rounded-lg text-white"
                  required
                />
              </div>

              <div>
                <label className="text-xs text-gray-400 mb-1 block">Price (USD)</label>
                <input
                  type="number"
                  step="0.01"
                  value={lpPrice}
                  onChange={(e) => setLpPrice(e.target.value)}
                  placeholder="186.00"
                  className="w-full px-3 py-2 bg-black border border-gray-700 rounded-lg text-white"
                  required
                />
              </div>

              <button
                type="submit"
                disabled={providingLp || !publicKey}
                className="w-full py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 text-white font-bold rounded-lg"
              >
                {providingLp ? 'Providing...' : 'Provide Liquidity'}
              </button>

              <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-3">
                <p className="text-xs text-green-300">
                  You'll earn fees from trades that match your liquidity
                </p>
              </div>
            </form>
          </div>
        </div>

        {/* Available Markets Section */}
        {publicKey && (
          <div className="mt-8 bg-gradient-to-br from-gray-900 to-gray-800 rounded-2xl border border-gray-700 p-6">
            <div className="flex items-center justify-between mb-6">
              <div>
                <h3 className="text-xl font-bold text-white">Available Markets</h3>
                <p className="text-sm text-gray-400">Trading pairs available on Percolator</p>
              </div>
              <div className="flex items-center gap-3">
                <button
                  onClick={loadUserSlabs}
                  disabled={loadingSlabs}
                  className="px-3 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 text-white text-xs font-semibold rounded-lg transition-colors"
                >
                  {loadingSlabs ? 'Loading...' : 'Refresh'}
                </button>
              </div>
            </div>
            
            {userSlabs.length === 0 ? (
              <div className="text-center py-12">
                <div className="w-16 h-16 rounded-full bg-gray-800 flex items-center justify-center mx-auto mb-4">
                  <Layers className="w-8 h-8 text-gray-600" />
                </div>
                <p className="text-gray-400 mb-2">No trading markets available yet</p>
                <p className="text-xs text-gray-500">Create the first market above to enable trading</p>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {userSlabs.map((slab, idx) => (
                  <div key={idx} className={`rounded-lg border transition-colors p-4 ${
                    slab.isBroken 
                      ? 'bg-gray-900/50 border-red-900 opacity-60' 
                      : 'bg-black border-gray-700 hover:border-blue-500'
                  }`}>
                    <div className="flex items-center justify-between mb-3">
                      <h4 className={`font-bold text-lg ${slab.isBroken ? 'text-gray-500' : 'text-white'}`}>
                        {slab.pair}
                      </h4>
                      {slab.isBroken ? (
                        <span className="text-xs px-2 py-1 rounded bg-red-900 text-red-300 font-semibold">BROKEN</span>
                      ) : (
                        <span className="text-xs px-2 py-1 rounded bg-green-600 text-white font-semibold">LIVE</span>
                      )}
                    </div>
                    
                    {slab.isBroken && (
                      <div className="mb-3 p-2 bg-red-900/20 border border-red-900/50 rounded">
                        <p className="text-xs text-red-400">
                          This market was created with buggy code and cannot be used. Please create a new market.
                        </p>
                      </div>
                    )}
                    
                    <div className="space-y-2 text-sm mb-4">
                      <div className="flex justify-between">
                        <span className="text-gray-400">Mark Price:</span>
                        <span className="text-white font-mono">${slab.markPrice.toLocaleString()}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Tick Size:</span>
                        <span className="text-white font-mono">${slab.tickSize}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-400">Lot Size:</span>
                        <span className="text-white font-mono">{slab.lotSize}</span>
                      </div>
                    </div>
                    
                    <div className="mb-3">
                      <p className="text-xs text-gray-500 mb-1">Market Address:</p>
                      <div className="flex items-center gap-2">
                        <code className="text-xs text-blue-400 font-mono truncate flex-1">
                          {slab.address.slice(0, 8)}...{slab.address.slice(-8)}
                        </code>
                        <button
                          onClick={() => {
                            navigator.clipboard.writeText(slab.address);
                            success('Address copied!');
                          }}
                          className="text-xs text-gray-400 hover:text-white px-2 py-1 rounded bg-gray-800 hover:bg-gray-700"
                        >
                          Copy
                        </button>
                      </div>
                    </div>
                    
                    <div className="mb-4">
                      <p className="text-xs text-gray-500 mb-1">Market Creator:</p>
                      <code className="text-xs text-gray-400 font-mono break-all">
                        {slab.owner && slab.owner.length > 32 
                          ? `${slab.owner.slice(0, 16)}...${slab.owner.slice(-16)}`
                          : slab.owner || 'Unknown'}
                      </code>
                    </div>
                    
                    {slab.isBroken ? (
                      <button 
                        disabled 
                        className="w-full py-3 bg-gray-800 text-gray-600 font-semibold rounded-lg cursor-not-allowed"
                      >
                        Cannot Trade (Broken)
                      </button>
                    ) : (
                      <Link href={`/dashboard?slab=${slab.address}`}>
                        <button className="w-full py-3 bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded-lg transition-colors">
                          Trade Now
                        </button>
                      </Link>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Info */}
        <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
            <h4 className="font-bold text-blue-400 mb-2">Available Pairs</h4>
            <p className="text-sm text-gray-300 mb-2">
              Currently supported trading pairs:
            </p>
            <ul className="text-xs text-gray-400 space-y-1">
              <li><span className="text-white">SOL/USDC</span> - Solana</li>
              <li><span className="text-white">ETH/USDC</span> - Ethereum</li>
              <li><span className="text-white">BTC/USDC</span> - Bitcoin</li>
            </ul>
          </div>
          
          <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-4">
            <h4 className="font-bold text-green-400 mb-2">LP Benefits</h4>
            <p className="text-sm text-gray-300">
              As a liquidity provider, you earn <span className="text-green-400 font-semibold">0.1% fees</span> from all trades that match against your orders.
            </p>
          </div>
          
          <div className="bg-purple-500/10 border border-purple-500/30 rounded-lg p-4">
            <h4 className="font-bold text-purple-400 mb-2">Want USDC/BTC?</h4>
            <p className="text-sm text-gray-300">
              To trade USDC → BTC, use the <span className="text-purple-400 font-semibold">BTC/USDC</span> pair with sell orders. It's the inverse of the same trade!
            </p>
          </div>
        </div>
      </main>
    </div>
  );
}
