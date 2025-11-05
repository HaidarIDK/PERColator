"use client"

import { useState, useEffect } from "react"
import { ArrowDown } from "lucide-react"
import { cn } from "@/lib/utils"
import { useWallet, useConnection } from "@solana/wallet-adapter-react"
import { Transaction, TransactionInstruction, PublicKey as SolanaPublicKey } from "@solana/web3.js"
import { Buffer } from "buffer"
import { PROGRAM_IDS, formatAddress } from "@/lib/program-config"

interface AMMInterfaceProps {
  selectedCoin: "ethereum" | "bitcoin" | "solana"
  mode: "swap" | "add" | "remove"
  showToast: (message: string, type: 'success' | 'error' | 'warning' | 'info') => void
  chartCurrentPrice: number
}

export const AMMInterface = ({ 
  selectedCoin, 
  mode,
  showToast,
  chartCurrentPrice
}: AMMInterfaceProps) => {
  const wallet = useWallet();
  const { publicKey, connected, signTransaction } = wallet;
  const { connection } = useConnection();

  const [swapFromAmount, setSwapFromAmount] = useState("");
  const [swapToAmount, setSwapToAmount] = useState("");
  const [swapToToken, setSwapToToken] = useState("USDC");
  const [priceImpact, setPriceImpact] = useState(0);
  const [swapLoading, setSwapLoading] = useState(false);

  const [addToken1Amount, setAddToken1Amount] = useState("");
  const [addToken2Amount, setAddToken2Amount] = useState("");
  const [expectedLPTokens, setExpectedLPTokens] = useState(0);
  const [addLiquidityLoading, setAddLiquidityLoading] = useState(false);

  const [lpTokenAmount, setLpTokenAmount] = useState("");
  const [expectedToken1, setExpectedToken1] = useState(0);
  const [expectedToken2, setExpectedToken2] = useState(0);
  const [userLPBalance, setUserLPBalance] = useState(0);
  const [removeLiquidityLoading, setRemoveLiquidityLoading] = useState(false);

  const [poolReserve1, setPoolReserve1] = useState(10000);
  const [poolReserve2, setPoolReserve2] = useState(2000000);
  const [poolLiquidity, setPoolLiquidity] = useState(141421);
  const [userShare, setUserShare] = useState(0);

  const getTokenSymbol = () => {
    switch(selectedCoin) {
      case "ethereum": return "ETH";
      case "bitcoin": return "BTC";
      case "solana": return "SOL";
      default: return "SOL";
    }
  };

  const getTokenPrice = () => {
    if (chartCurrentPrice > 0) {
      return chartCurrentPrice;
    }
    switch(selectedCoin) {
      case "ethereum": return 4130;
      case "bitcoin": return 114300;
      case "solana": return 199;
      default: return 199;
    }
  };

  useEffect(() => {
    const currentPrice = getTokenPrice();
    
    switch(selectedCoin) {
      case "ethereum":
        setPoolReserve1(100);
        setPoolReserve2(100 * currentPrice);
        setPoolLiquidity(Math.sqrt(100 * (100 * currentPrice)));
        break;
      case "bitcoin":
        setPoolReserve1(10);
        setPoolReserve2(10 * currentPrice);
        setPoolLiquidity(Math.sqrt(10 * (10 * currentPrice)));
        break;
      case "solana":
      default:
        setPoolReserve1(10000);
        setPoolReserve2(10000 * currentPrice);
        setPoolLiquidity(Math.sqrt(10000 * (10000 * currentPrice)));
        break;
    }
  }, [selectedCoin, chartCurrentPrice]);

  const calculateSwapOutput = (amountIn: number, reserveIn: number, reserveOut: number) => {
    const amountInWithFee = amountIn * 997;
    const numerator = amountInWithFee * reserveOut;
    const denominator = (reserveIn * 1000) + amountInWithFee;
    return numerator / denominator;
  };

  const calculatePriceImpact = (amountIn: number, reserveIn: number, reserveOut: number) => {
    const priceBeforeSwap = reserveOut / reserveIn;
    const amountOut = calculateSwapOutput(amountIn, reserveIn, reserveOut);
    const priceAfterSwap = (reserveOut - amountOut) / (reserveIn + amountIn);
    return ((priceAfterSwap - priceBeforeSwap) / priceBeforeSwap) * 100;
  };

  useEffect(() => {
    if (swapFromAmount && parseFloat(swapFromAmount) > 0) {
      const amountIn = parseFloat(swapFromAmount);
      const amountOut = calculateSwapOutput(amountIn, poolReserve1, poolReserve2);
      setSwapToAmount(amountOut.toFixed(2));
      
      const impact = calculatePriceImpact(amountIn, poolReserve1, poolReserve2);
      setPriceImpact(Math.abs(impact));
    } else {
      setSwapToAmount("");
      setPriceImpact(0);
    }
  }, [swapFromAmount, poolReserve1, poolReserve2, selectedCoin, chartCurrentPrice]);

  useEffect(() => {
    if (addToken1Amount && parseFloat(addToken1Amount) > 0) {
      const amount1 = parseFloat(addToken1Amount);
      const ratio = poolReserve2 / poolReserve1;
      const amount2 = amount1 * ratio;
      setAddToken2Amount(amount2.toFixed(2));
      
      const liquidityMinted = Math.sqrt(amount1 * amount2) * poolLiquidity / Math.sqrt(poolReserve1 * poolReserve2);
      setExpectedLPTokens(liquidityMinted);
    } else {
      setAddToken2Amount("");
      setExpectedLPTokens(0);
    }
  }, [addToken1Amount, poolReserve1, poolReserve2, poolLiquidity, selectedCoin, chartCurrentPrice]);

  useEffect(() => {
    if (lpTokenAmount && parseFloat(lpTokenAmount) > 0) {
      const lpAmount = parseFloat(lpTokenAmount);
      const share = lpAmount / poolLiquidity;
      setExpectedToken1(poolReserve1 * share);
      setExpectedToken2(poolReserve2 * share);
    } else {
      setExpectedToken1(0);
      setExpectedToken2(0);
    }
  }, [lpTokenAmount, poolLiquidity, poolReserve1, poolReserve2, selectedCoin, chartCurrentPrice]);

  const handleSwap = async () => {
    if (!connected || !publicKey) {
      showToast('⚠️ Please connect your wallet', 'warning');
      return;
    }

    if (!signTransaction) {
      showToast('⚠️ Wallet does not support signing', 'warning');
      return;
    }
    
    setSwapLoading(true);
    try {
      const transaction = new Transaction();
      
      const memoData = `PERColator v0 AMM Swap: ${swapFromAmount} ${getTokenSymbol()} → ${swapToAmount} ${swapToToken}`;
      const memoInstruction = new TransactionInstruction({
        keys: [],
        programId: new SolanaPublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr'),
        data: Buffer.from(memoData, 'utf-8'),
      });
      transaction.add(memoInstruction);
      
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
      transaction.recentBlockhash = blockhash;
      transaction.feePayer = publicKey;
      
      const signedTx = await signTransaction(transaction);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction({
        signature,
        blockhash,
        lastValidBlockHeight
      });
      
      showToast(
        `✅ AMM Swap Executed On-Chain!\n\n` +
        `${swapFromAmount} ${getTokenSymbol()} → ${swapToAmount} ${swapToToken}\n\n` +
        `⚠️ v0 Demo Mode: Visual only\n` +
        `(No real tokens swapped yet)\n\n` +
        `━━━━━━━━━━━━━━━━━━━━━━\n` +
        `Transaction Signature:\n` +
        `${signature}\n\n` +
        `📋 Click to select & copy\n` +
        `🔗 View on Solscan:\n` +
        `https://solscan.io/tx/${signature}?cluster=devnet`,
        'success'
      );
      
      setSwapFromAmount("");
      setSwapToAmount("");
    } catch (error: any) {
      const errorMsg = error?.message || 'Unknown error';
      
      if (errorMsg.includes('User rejected')) {
        showToast('❌ You cancelled the transaction', 'error');
      } else if (errorMsg.includes('insufficient')) {
        showToast('❌ Insufficient SOL for transaction fees', 'error');
      } else {
        showToast('❌ Swap failed\n\n' + errorMsg, 'error');
      }
    } finally {
      setSwapLoading(false);
    }
  };

  const handleAddLiquidity = async () => {
    if (!connected || !publicKey) {
      showToast('⚠️ Please connect your wallet', 'warning');
      return;
    }

    if (!signTransaction) {
      showToast('⚠️ Wallet does not support signing', 'warning');
      return;
    }
    
    setAddLiquidityLoading(true);
    try {
      const transaction = new Transaction();
      
      const memoData = `PERColator v0 AMM Add Liquidity: ${addToken1Amount} ${getTokenSymbol()} + ${addToken2Amount} USDC = ${expectedLPTokens.toFixed(2)} LP`;
      const memoInstruction = new TransactionInstruction({
        keys: [],
        programId: new SolanaPublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr'),
        data: Buffer.from(memoData, 'utf-8'),
      });
      transaction.add(memoInstruction);
      
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
      transaction.recentBlockhash = blockhash;
      transaction.feePayer = publicKey;
      
      const signedTx = await signTransaction(transaction);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction({
        signature,
        blockhash,
        lastValidBlockHeight
      });
      
      showToast(
        `✅ Liquidity Added On-Chain!\n\n` +
        `Received ${expectedLPTokens.toFixed(2)} LP tokens\n\n` +
        `⚠️ v0 Demo Mode: Visual only\n` +
        `(No real LP tokens minted yet)\n\n` +
        `━━━━━━━━━━━━━━━━━━━━━━\n` +
        `Transaction Signature:\n` +
        `${signature}\n\n` +
        `📋 Click to select & copy\n` +
        `🔗 View on Solscan:\n` +
        `https://solscan.io/tx/${signature}?cluster=devnet`,
        'success'
      );
      
      setAddToken1Amount("");
      setAddToken2Amount("");
    } catch (error: any) {
      const errorMsg = error?.message || 'Unknown error';
      
      if (errorMsg.includes('User rejected')) {
        showToast('❌ You cancelled the transaction', 'error');
      } else if (errorMsg.includes('insufficient')) {
        showToast('❌ Insufficient SOL for transaction fees', 'error');
      } else {
        showToast('❌ Add liquidity failed\n\n' + errorMsg, 'error');
      }
    } finally {
      setAddLiquidityLoading(false);
    }
  };

  const handleRemoveLiquidity = async () => {
    if (!connected || !publicKey) {
      showToast('⚠️ Please connect your wallet', 'warning');
      return;
    }

    if (!signTransaction) {
      showToast('⚠️ Wallet does not support signing', 'warning');
      return;
    }
    
    setRemoveLiquidityLoading(true);
    try {
      const transaction = new Transaction();
      
      const memoData = `PERColator v0 AMM Remove Liquidity: ${lpTokenAmount} LP → ${expectedToken1.toFixed(2)} ${getTokenSymbol()} + ${expectedToken2.toFixed(2)} USDC`;
      const memoInstruction = new TransactionInstruction({
        keys: [],
        programId: new SolanaPublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr'),
        data: Buffer.from(memoData, 'utf-8'),
      });
      transaction.add(memoInstruction);
      
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
      transaction.recentBlockhash = blockhash;
      transaction.feePayer = publicKey;
      
      const signedTx = await signTransaction(transaction);
      const signature = await connection.sendRawTransaction(signedTx.serialize());
      
      await connection.confirmTransaction({
        signature,
        blockhash,
        lastValidBlockHeight
      });
      
      showToast(
        `✅ Liquidity Removed On-Chain!\n\n` +
        `Received:\n` +
        `${expectedToken1.toFixed(2)} ${getTokenSymbol()}\n` +
        `${expectedToken2.toFixed(2)} USDC\n\n` +
        `⚠️ v0 Demo Mode: Visual only\n` +
        `(No real tokens withdrawn yet)\n\n` +
        `━━━━━━━━━━━━━━━━━━━━━━\n` +
        `Transaction Signature:\n` +
        `${signature}\n\n` +
        `📋 Click to select & copy\n` +
        `🔗 View on Solscan:\n` +
        `https://solscan.io/tx/${signature}?cluster=devnet`,
        'success'
      );
      
      setLpTokenAmount("");
    } catch (error: any) {
      const errorMsg = error?.message || 'Unknown error';
      
      if (errorMsg.includes('User rejected')) {
        showToast('❌ You cancelled the transaction', 'error');
      } else if (errorMsg.includes('insufficient')) {
        showToast('❌ Insufficient SOL for transaction fees', 'error');
      } else {
        showToast('❌ Remove liquidity failed\n\n' + errorMsg, 'error');
      }
    } finally {
      setRemoveLiquidityLoading(false);
    }
  };

  return (
    <div className="bg-gradient-to-br from-black/40 via-black/30 to-black/20 backdrop-blur-xl rounded-2xl border border-[#181825] overflow-hidden">
      <div className="bg-gradient-to-r from-green-500/10 via-emerald-500/5 to-transparent px-4 sm:px-6 py-4 border-b border-[#181825]">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h3 className="text-base sm:text-lg font-bold text-white flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-green-400 animate-pulse shadow-[0_0_10px_rgba(74,222,128,0.5)]"></div>
              AMM Liquidity Pool
            </h3>
            <p className="text-xs text-gray-400 mt-1 font-medium">{getTokenSymbol()}/USDC Pool</p>
          </div>
          <div className="text-left sm:text-right">
            <div className="text-[10px] sm:text-xs text-gray-400 uppercase tracking-wide">Total Liquidity</div>
            <div className="text-xl sm:text-2xl font-bold bg-gradient-to-r from-green-400 to-emerald-400 bg-clip-text text-transparent">
              ${(poolReserve2 * 2).toLocaleString(undefined, { maximumFractionDigits: 0 })}
            </div>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-2 sm:gap-3 p-4 sm:p-6 bg-black/20">
        <div className="bg-gradient-to-br from-blue-500/10 via-blue-600/5 to-transparent rounded-xl p-3 sm:p-4 border border-blue-500/20 hover:border-blue-500/40 transition-all duration-300 hover:shadow-lg hover:shadow-blue-500/10">
          <div className="text-[10px] sm:text-xs text-gray-400 mb-1 uppercase tracking-wide">{getTokenSymbol()} Reserve</div>
          <div className="text-sm sm:text-base font-bold text-white">{poolReserve1.toLocaleString()}</div>
        </div>
        <div className="bg-gradient-to-br from-green-500/10 via-green-600/5 to-transparent rounded-xl p-3 sm:p-4 border border-green-500/20 hover:border-green-500/40 transition-all duration-300 hover:shadow-lg hover:shadow-green-500/10">
          <div className="text-[10px] sm:text-xs text-gray-400 mb-1 uppercase tracking-wide">USDC Reserve</div>
          <div className="text-sm sm:text-base font-bold text-white">${poolReserve2.toLocaleString()}</div>
        </div>
        <div className="bg-gradient-to-br from-purple-500/10 via-purple-600/5 to-transparent rounded-xl p-3 sm:p-4 border border-purple-500/20 hover:border-purple-500/40 transition-all duration-300 hover:shadow-lg hover:shadow-purple-500/10">
          <div className="text-[10px] sm:text-xs text-gray-400 mb-1 uppercase tracking-wide">AMM Price</div>
          <div className="text-sm sm:text-base font-bold text-white">${getTokenPrice().toFixed(2)}</div>
        </div>
        <div className="bg-gradient-to-br from-orange-500/10 via-orange-600/5 to-transparent rounded-xl p-3 sm:p-4 border border-orange-500/20 hover:border-orange-500/40 transition-all duration-300 hover:shadow-lg hover:shadow-orange-500/10">
          <div className="text-[10px] sm:text-xs text-gray-400 mb-1 uppercase tracking-wide">Your Share</div>
          <div className="text-sm sm:text-base font-bold text-white">{userShare.toFixed(3)}%</div>
        </div>
      </div>

      <div className="p-4 sm:p-6">
        {mode === "swap" && (
          <div className="space-y-4 sm:space-y-5">
            <div className="flex items-center justify-between">
              <h4 className="text-sm sm:text-base font-bold text-white">Swap Tokens</h4>
              <div className="text-xs text-gray-500">Fee: 0.3%</div>
            </div>
            
            <div className="group bg-gradient-to-br from-black/60 via-black/40 to-black/30 rounded-2xl p-4 sm:p-5 border border-[#181825] hover:border-[#B8B8FF]/30 transition-all duration-300 hover:shadow-lg hover:shadow-[#B8B8FF]/5">
              <div className="flex items-center justify-between mb-3">
                <label className="text-xs sm:text-sm text-gray-400 font-medium uppercase tracking-wide">From</label>
                <span className="text-xs text-gray-500">Balance: <span className="text-white font-medium">0.00</span></span>
              </div>
              <div className="flex items-center gap-3 sm:gap-4">
                <input
                  type="number"
                  value={swapFromAmount}
                  onChange={(e) => setSwapFromAmount(e.target.value)}
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl sm:text-3xl font-bold text-white outline-none placeholder:text-gray-700"
                />
                <div className="flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-3 bg-gradient-to-br from-[#B8B8FF]/20 to-[#B8B8FF]/10 rounded-xl border border-[#B8B8FF]/40 shadow-lg shadow-[#B8B8FF]/10">
                  <span className="text-sm sm:text-base font-bold text-white">{getTokenSymbol()}</span>
                </div>
              </div>
            </div>

            <div className="flex justify-center -my-2 sm:-my-3 relative z-10">
              <div className="bg-gradient-to-br from-black/80 to-black/60 border-2 border-[#181825] rounded-full p-2 sm:p-2.5 hover:border-[#B8B8FF]/50 hover:shadow-lg hover:shadow-[#B8B8FF]/20 transition-all duration-300 cursor-pointer">
                <ArrowDown className="w-3.5 h-3.5 sm:w-4 sm:h-4 text-[#B8B8FF]" />
              </div>
            </div>

            <div className="group bg-gradient-to-br from-black/60 via-black/40 to-black/30 rounded-2xl p-4 sm:p-5 border border-[#181825] hover:border-green-500/30 transition-all duration-300 hover:shadow-lg hover:shadow-green-500/5">
              <div className="flex items-center justify-between mb-3">
                <label className="text-xs sm:text-sm text-gray-400 font-medium uppercase tracking-wide">To</label>
                <span className="text-xs text-gray-500">Balance: <span className="text-white font-medium">0.00</span></span>
              </div>
              <div className="flex items-center gap-3 sm:gap-4">
                <input
                  type="number"
                  value={swapToAmount}
                  readOnly
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl sm:text-3xl font-bold text-white outline-none placeholder:text-gray-700"
                />
                <div className="flex items-center gap-2 px-3 sm:px-4 py-2 sm:py-3 bg-gradient-to-br from-green-500/20 to-green-600/10 rounded-xl border border-green-500/40 shadow-lg shadow-green-500/10">
                  <span className="text-sm sm:text-base font-bold text-white">{swapToToken}</span>
                </div>
              </div>
            </div>

            {swapFromAmount && parseFloat(swapFromAmount) > 0 && (
              <div className="bg-gradient-to-br from-[#B8B8FF]/10 via-purple-500/5 to-transparent rounded-2xl p-4 sm:p-5 border border-[#B8B8FF]/30 space-y-3 backdrop-blur-sm">
                <div className="text-xs sm:text-sm font-semibold text-white mb-2">Transaction Details</div>
                <div className="flex justify-between items-center text-xs sm:text-sm">
                  <span className="text-gray-400">Exchange Rate</span>
                  <span className="text-white font-bold">1 {getTokenSymbol()} = {(parseFloat(swapToAmount) / parseFloat(swapFromAmount)).toFixed(2)} {swapToToken}</span>
                </div>
                <div className="flex justify-between items-center text-xs sm:text-sm">
                  <span className="text-gray-400">Price Impact</span>
                  <span className={cn(
                    "font-bold px-2 py-0.5 rounded-md",
                    priceImpact < 1 
                      ? "text-green-400 bg-green-500/10" 
                      : priceImpact < 3 
                      ? "text-yellow-400 bg-yellow-500/10" 
                      : "text-red-400 bg-red-500/10"
                  )}>
                    {priceImpact.toFixed(3)}%
                  </span>
                </div>
                <div className="flex justify-between items-center text-xs sm:text-sm">
                  <span className="text-gray-400">Min. Received (0.5% slippage)</span>
                  <span className="text-white font-bold">{(parseFloat(swapToAmount) * 0.995).toFixed(2)} {swapToToken}</span>
                </div>
                <div className="pt-2 border-t border-[#B8B8FF]/10">
                  <div className="flex justify-between items-center text-xs sm:text-sm">
                    <span className="text-gray-400">LP Fee (0.3%)</span>
                    <span className="text-purple-400 font-medium">{(parseFloat(swapFromAmount) * 0.003).toFixed(4)} {getTokenSymbol()}</span>
                  </div>
                </div>
                <div className="flex justify-between text-base font-bold pt-1.5 border-t border-[#181825]">
                  <span className="text-[#B8B8FF]">Total:</span>
                  <span className="text-white">{parseFloat(swapToAmount).toFixed(2)} {swapToToken}</span>
                </div>
              </div>
            )}

            <button
              onClick={handleSwap}
              disabled={!connected || swapLoading || !swapFromAmount || parseFloat(swapFromAmount) <= 0}
              className={cn(
                "w-full py-3 sm:py-4 rounded-xl font-bold text-base sm:text-lg transition-all duration-300 shadow-lg",
                !connected || swapLoading || !swapFromAmount || parseFloat(swapFromAmount) <= 0
                  ? "bg-gray-600/20 text-gray-500 cursor-not-allowed"
                  : "bg-gradient-to-r from-[#B8B8FF]/40 via-purple-500/40 to-[#B8B8FF]/40 hover:from-[#B8B8FF]/50 hover:via-purple-500/50 hover:to-[#B8B8FF]/50 border border-[#B8B8FF]/50 text-white hover:shadow-[#B8B8FF]/30"
              )}
            >
              {swapLoading ? (
                <div className="flex items-center justify-center gap-2">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  Swapping...
                </div>
              ) : !connected ? "Connect Wallet" : "Swap Tokens"}
            </button>
          </div>
        )}

        {mode === "add" && (
          <div className="space-y-4 sm:space-y-5">
            <div className="flex items-center justify-between">
              <h4 className="text-sm sm:text-base font-bold text-white">Add Liquidity</h4>
              <div className="text-xs text-gray-500">Earn fees from swaps</div>
            </div>
            
            <div className="group bg-gradient-to-br from-black/60 via-black/40 to-black/30 rounded-2xl p-4 sm:p-5 border border-[#181825] hover:border-blue-500/30 transition-all duration-300 hover:shadow-lg hover:shadow-blue-500/5">
              <div className="flex items-center justify-between mb-3">
                <label className="text-xs sm:text-sm text-gray-400 font-medium uppercase tracking-wide">{getTokenSymbol()}</label>
                <span className="text-xs text-gray-500">Balance: <span className="text-white font-medium">0.00</span></span>
              </div>
              <div className="flex items-center gap-3 sm:gap-4">
                <input
                  type="number"
                  value={addToken1Amount}
                  onChange={(e) => setAddToken1Amount(e.target.value)}
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl sm:text-3xl font-bold text-white outline-none placeholder:text-gray-700"
                />
                <div className="px-3 sm:px-4 py-2 sm:py-3 bg-gradient-to-br from-blue-500/20 to-blue-600/10 rounded-xl border border-blue-500/40">
                  <span className="text-sm sm:text-base font-bold text-white">{getTokenSymbol()}</span>
                </div>
              </div>
            </div>

            <div className="flex justify-center -my-3 relative z-10">
              <div className="bg-gradient-to-br from-green-500/20 to-emerald-500/20 border-2 border-green-500/30 rounded-full p-2.5 shadow-lg shadow-green-500/10">
                <div className="text-green-400 text-xl font-bold">+</div>
              </div>
            </div>

            <div className="group bg-gradient-to-br from-black/60 via-black/40 to-black/30 rounded-2xl p-4 sm:p-5 border border-[#181825] hover:border-green-500/30 transition-all duration-300 hover:shadow-lg hover:shadow-green-500/5">
              <div className="flex items-center justify-between mb-3">
                <label className="text-xs sm:text-sm text-gray-400 font-medium uppercase tracking-wide">USDC</label>
                <span className="text-xs text-gray-500">Balance: <span className="text-white font-medium">0.00</span></span>
              </div>
              <div className="flex items-center gap-3 sm:gap-4">
                <input
                  type="number"
                  value={addToken2Amount}
                  readOnly
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl sm:text-3xl font-bold text-gray-400 outline-none placeholder:text-gray-700"
                />
                <div className="px-3 sm:px-4 py-2 sm:py-3 bg-gradient-to-br from-green-500/20 to-green-600/10 rounded-xl border border-green-500/40">
                  <span className="text-sm sm:text-base font-bold text-white">USDC</span>
                </div>
              </div>
            </div>

            {addToken1Amount && parseFloat(addToken1Amount) > 0 && (
              <div className="bg-gradient-to-br from-green-500/15 via-emerald-500/10 to-transparent rounded-2xl p-4 sm:p-5 border border-green-500/30 space-y-3 backdrop-blur-sm">
                <div className="text-xs sm:text-sm font-semibold text-white mb-2">Expected Output</div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-300">LP Tokens</span>
                  <span className="text-xl sm:text-2xl font-bold text-green-400">{expectedLPTokens.toFixed(2)}</span>
                </div>
                <div className="pt-2 border-t border-green-500/20">
                  <div className="flex justify-between items-center text-xs sm:text-sm">
                    <span className="text-gray-400">Your Pool Share</span>
                    <span className="text-white font-bold">{((expectedLPTokens / (poolLiquidity + expectedLPTokens)) * 100).toFixed(4)}%</span>
                  </div>
                </div>
              </div>
            )}

            <button
              onClick={handleAddLiquidity}
              disabled={!connected || addLiquidityLoading || !addToken1Amount || parseFloat(addToken1Amount) <= 0}
              className={cn(
                "w-full py-3 sm:py-4 rounded-xl font-bold text-base sm:text-lg transition-all duration-300 shadow-lg",
                !connected || addLiquidityLoading || !addToken1Amount || parseFloat(addToken1Amount) <= 0
                  ? "bg-gray-600/20 text-gray-500 cursor-not-allowed"
                  : "bg-gradient-to-r from-green-500/40 via-emerald-500/40 to-green-500/40 hover:from-green-500/50 hover:via-emerald-500/50 hover:to-green-500/50 border border-green-500/50 text-white hover:shadow-green-500/30"
              )}
            >
              {addLiquidityLoading ? (
                <div className="flex items-center justify-center gap-2">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  Adding Liquidity...
                </div>
              ) : !connected ? "Connect Wallet" : "Add Liquidity"}
            </button>
          </div>
        )}

        {mode === "remove" && (
          <div className="space-y-4 sm:space-y-5">
            <div className="flex items-center justify-between">
              <h4 className="text-sm sm:text-base font-bold text-white">Remove Liquidity</h4>
              <div className="text-xs text-gray-500">Burn LP tokens</div>
            </div>
            
            <div className="group bg-gradient-to-br from-black/60 via-black/40 to-black/30 rounded-2xl p-4 sm:p-5 border border-[#181825] hover:border-orange-500/30 transition-all duration-300 hover:shadow-lg hover:shadow-orange-500/5">
              <div className="flex items-center justify-between mb-3">
                <label className="text-xs sm:text-sm text-gray-400 font-medium uppercase tracking-wide">LP Tokens</label>
                <span className="text-xs text-gray-500">Balance: <span className="text-white font-medium">{userLPBalance.toFixed(2)}</span></span>
              </div>
              <div className="flex items-center gap-3 sm:gap-4">
                <input
                  type="number"
                  value={lpTokenAmount}
                  onChange={(e) => setLpTokenAmount(e.target.value)}
                  placeholder="0.0"
                  className="flex-1 bg-transparent text-2xl sm:text-3xl font-bold text-white outline-none placeholder:text-gray-700"
                />
                <button 
                  onClick={() => setLpTokenAmount(userLPBalance.toString())}
                  className="px-3 sm:px-4 py-2 sm:py-2.5 bg-gradient-to-br from-orange-500/30 to-red-500/30 rounded-xl text-xs sm:text-sm font-bold text-white hover:from-orange-500/40 hover:to-red-500/40 transition-all duration-300 border border-orange-500/50"
                >
                  MAX
                </button>
              </div>
            </div>

            {lpTokenAmount && parseFloat(lpTokenAmount) > 0 && (
              <div className="bg-gradient-to-br from-orange-500/15 via-red-500/10 to-transparent rounded-2xl p-4 sm:p-5 border border-orange-500/30 space-y-3 backdrop-blur-sm">
                <div className="text-xs sm:text-sm font-semibold text-white mb-2">You will receive</div>
                <div className="flex justify-between items-center p-3 bg-black/30 rounded-xl border border-orange-500/20">
                  <span className="text-sm text-gray-300">{getTokenSymbol()}</span>
                  <span className="text-xl sm:text-2xl font-bold text-white">{expectedToken1.toFixed(4)}</span>
                </div>
                <div className="flex justify-between items-center p-3 bg-black/30 rounded-xl border border-orange-500/20">
                  <span className="text-sm text-gray-300">USDC</span>
                  <span className="text-xl sm:text-2xl font-bold text-white">{expectedToken2.toFixed(2)}</span>
                </div>
                <div className="pt-2 border-t border-orange-500/20">
                  <div className="flex justify-between items-center text-xs sm:text-sm">
                    <span className="text-gray-400">LP Tokens Burned</span>
                    <span className="text-red-400 font-bold">{lpTokenAmount}</span>
                  </div>
                </div>
              </div>
            )}

            <button
              onClick={handleRemoveLiquidity}
              disabled={!connected || removeLiquidityLoading || !lpTokenAmount || parseFloat(lpTokenAmount) <= 0}
              className={cn(
                "w-full py-3 sm:py-4 rounded-xl font-bold text-base sm:text-lg transition-all duration-300 shadow-lg",
                !connected || removeLiquidityLoading || !lpTokenAmount || parseFloat(lpTokenAmount) <= 0
                  ? "bg-gray-600/20 text-gray-500 cursor-not-allowed"
                  : "bg-gradient-to-r from-orange-500/40 via-red-500/40 to-orange-500/40 hover:from-orange-500/50 hover:via-red-500/50 hover:to-orange-500/50 border border-orange-500/50 text-white hover:shadow-orange-500/30"
              )}
            >
              {removeLiquidityLoading ? (
                <div className="flex items-center justify-center gap-2">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  Removing Liquidity...
                </div>
              ) : !connected ? "Connect Wallet" : "Remove Liquidity"}
            </button>
          </div>
        )}
      </div>

      <div className="px-4 sm:px-6 py-4 bg-black/30 border-t border-[#181825]">
        <div className="flex flex-col sm:flex-row items-center justify-between gap-3 text-xs sm:text-sm">
          <div className="flex items-center gap-2">
            <span className="text-gray-400">AMM Program:</span>
            <code className="text-green-400 font-mono bg-green-500/10 px-2 py-1 rounded border border-green-500/20">{formatAddress(PROGRAM_IDS.amm)}</code>
          </div>
          <div className="text-center text-gray-500">
            <span className="hidden sm:inline">Constant Product Formula: </span>
            <span className="font-mono text-[#B8B8FF]">x · y = k</span>
            <span className="mx-2 hidden sm:inline">|</span>
            <span className="sm:inline block mt-1 sm:mt-0">Fee: <span className="text-white font-bold">0.3%</span></span>
          </div>
        </div>
      </div>
    </div>
  );
};


