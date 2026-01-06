"use client"

import { useState, useRef, useEffect } from 'react';
import { Terminal, Home, Copy, CheckCircle2, Sparkles, Send } from 'lucide-react';
import Link from 'next/link';
import { TestnetBanner } from '@/components/TestnetBanner';
import { StatusFooter } from '@/components/StatusFooter';
import { useWallet, useConnection } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { Transaction, TransactionInstruction, PublicKey as SolanaPublicKey } from '@solana/web3.js';

export default function CLIPage() {
  const { publicKey, signTransaction } = useWallet();
  const { connection } = useConnection();
  const [command, setCommand] = useState('');
  const [history, setHistory] = useState<Array<{ cmd: string; output: string; timestamp: Date; success: boolean }>>([]);
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [copied, setCopied] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [cliStatus, setCliStatus] = useState<'checking' | 'ready' | 'unavailable'>('checking');
  const [aiQuestion, setAiQuestion] = useState('');
  const [aiLoading, setAiLoading] = useState(false);
  const [aiResponse, setAiResponse] = useState<{ command: string; explanation: string; example: string } | null>(null);
  const [useWebSocket, setUseWebSocket] = useState(true);
  const [wsConnected, setWsConnected] = useState(false);
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const wsRef = useRef<WebSocket | null>(null);

  // Auto-scroll to bottom when history updates
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [history]);

  // Load command history from localStorage on mount
  useEffect(() => {
    const savedHistory = localStorage.getItem('cli-command-history');
    if (savedHistory) {
      try {
        const parsed = JSON.parse(savedHistory);
        setCommandHistory(parsed);
      } catch (e) {
        console.error('Failed to load command history:', e);
      }
    }
  }, []);

  // Save command history to localStorage
  useEffect(() => {
    if (commandHistory.length > 0) {
      // Keep last 100 commands
      const toSave = commandHistory.slice(-100);
      localStorage.setItem('cli-command-history', JSON.stringify(toSave));
    }
  }, [commandHistory]);

  // WebSocket connection
  useEffect(() => {
    if (!useWebSocket) return;

    const connectWebSocket = () => {
      const wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:5001/ws';
      const ws = new WebSocket(wsUrl.replace('/ws', '/ws/cli'));

      ws.onopen = () => {
        console.log('✅ Connected to CLI WebSocket');
        setWsConnected(true);
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);

          switch (message.type) {
            case 'connected':
              console.log('CLI WebSocket ready:', message.message);
              break;

            case 'output':
              // Append streaming output to current command
              setHistory(prev => {
                if (prev.length === 0) return prev;
                const last = prev[prev.length - 1];
                return [
                  ...prev.slice(0, -1),
                  {
                    ...last,
                    output: (last.output || '') + message.data,
                  },
                ];
              });
              break;

            case 'complete':
              // Mark command as complete
              setHistory(prev => {
                if (prev.length === 0) return prev;
                const last = prev[prev.length - 1];
                return [
                  ...prev.slice(0, -1),
                  {
                    ...last,
                    success: message.success,
                    output: last.output || (message.success ? 'Command completed' : 'Command failed'),
                  },
                ];
              });
              setIsExecuting(false);
              setCommand('');
              break;

            case 'error':
              setHistory(prev => [...prev, {
                cmd: '',
                output: `Error: ${message.error}`,
                timestamp: new Date(),
                success: false,
              }]);
              setIsExecuting(false);
              break;

            case 'pong':
              // Heartbeat response
              break;
          }
        } catch (e) {
          console.error('Failed to parse WebSocket message:', e);
        }
      };

      ws.onclose = () => {
        console.log('❌ Disconnected from CLI WebSocket');
        setWsConnected(false);
        // Attempt to reconnect after 3 seconds
        setTimeout(connectWebSocket, 3000);
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        setWsConnected(false);
      };

      wsRef.current = ws;
    };

    connectWebSocket();

    // Cleanup on unmount
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [useWebSocket]);

  // Check CLI status on mount
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
        const response = await fetch(`${apiUrl}/api/cli/status`);
        const data = await response.json();
        // Always set to ready - backend will handle mock mode
        setCliStatus('ready');
      } catch (error) {
        console.error('Failed to check CLI status:', error);
        // Still allow commands even if status check fails
        setCliStatus('ready');
      }
    };
    checkStatus();
    inputRef.current?.focus();
  }, []);

  const executeCommand = async (cmd: string) => {
    if (!cmd.trim() || isExecuting) return;

    // Add to command history
    setCommandHistory(prev => [...prev, cmd]);
    setHistoryIndex(-1);
    setIsExecuting(true);

    // Handle clear command locally
    if (cmd.trim().toLowerCase() === 'clear' || cmd.trim().toLowerCase() === 'cls') {
      setHistory([]);
      setIsExecuting(false);
      setCommand('');
      return;
    }

    // Try WebSocket first if connected
    if (useWebSocket && wsConnected && wsRef.current?.readyState === WebSocket.OPEN) {
      // Add command to history with empty output (will be filled by streaming)
      setHistory(prev => [...prev, {
        cmd,
        output: '',
        timestamp: new Date(),
        success: true,
      }]);

      // Send command via WebSocket
      wsRef.current.send(JSON.stringify({
        type: 'execute',
        command: cmd,
        network: 'localnet',
      }));
      return;
    }

    // Fallback to HTTP API
    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
      const response = await fetch(`${apiUrl}/api/cli/execute`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          command: cmd,
          wallet: publicKey?.toBase58() || null,
          network: 'localnet',
        }),
      });

      const data = await response.json();

      // Check if transaction requires signature
      if (data.requiresSignature && data.transactionData) {
        try {
          // Build and execute real transaction
          if (!publicKey || !signTransaction) {
            setHistory(prev => [...prev, {
              cmd,
              output: '❌ Wallet not properly connected. Please connect Phantom wallet.',
              timestamp: new Date(),
              success: false,
            }]);
            setIsExecuting(false);
            return;
          }

          const txData = data.transactionData;
          let memoData = '';
          let successMessage = '';

          // Handle different transaction types
          switch (txData.type) {
            case 'swap':
              setHistory(prev => [...prev, {
                cmd,
                output: `🔄 Preparing swap: ${txData.amount} ${txData.from} → ${txData.to}...\n\n🔐 Please approve in Phantom...`,
                timestamp: new Date(),
                success: true,
              }]);
              memoData = `Percolator CLI AMM Swap: ${txData.amount} ${txData.from} → ${txData.to}`;
              successMessage = `✅ AMM Swap Executed!\n\n${txData.amount} ${txData.from} → ${txData.to}`;
              break;

            case 'order':
              setHistory(prev => [...prev, {
                cmd,
                output: `📊 Placing ${txData.orderType} ${txData.side} order...\n\nCoin: ${txData.coin}\nAmount: ${txData.amount}\n${txData.price ? `Price: $${txData.price}` : 'Market Price'}\n\n🔐 Please approve in Phantom...`,
                timestamp: new Date(),
                success: true,
              }]);
              memoData = `Percolator CLI Order: ${txData.side.toUpperCase()} ${txData.amount} ${txData.coin} @ ${txData.price || 'Market'}`;
              successMessage = `✅ ${txData.side.toUpperCase()} Order Placed!\n\n${txData.amount} ${txData.coin} ${txData.orderType === 'limit' && txData.price ? `@ $${txData.price}` : 'at market price'}`;
              break;

            case 'add-liquidity':
              setHistory(prev => [...prev, {
                cmd,
                output: `💧 Adding liquidity to ${txData.token1}-${txData.token2} pool...\n\n${txData.token1}: ${txData.amount1}\n${txData.token2}: ${txData.amount2}\n\n🔐 Please approve in Phantom...`,
                timestamp: new Date(),
                success: true,
              }]);
              memoData = `Percolator CLI Add Liquidity: ${txData.amount1} ${txData.token1} + ${txData.amount2} ${txData.token2}`;
              successMessage = `✅ Liquidity Added!\n\n${txData.amount1} ${txData.token1} + ${txData.amount2} ${txData.token2}\n\nLP tokens received: ~${Math.sqrt(txData.amount1 * txData.amount2).toFixed(2)}`;
              break;

            case 'remove-liquidity':
              setHistory(prev => [...prev, {
                cmd,
                output: `💧 Removing ${txData.lpAmount} LP tokens from ${txData.token1}-${txData.token2} pool...\n\n🔐 Please approve in Phantom...`,
                timestamp: new Date(),
                success: true,
              }]);
              memoData = `Percolator CLI Remove Liquidity: ${txData.lpAmount} LP → ${txData.token1}/${txData.token2}`;
              successMessage = `✅ Liquidity Removed!\n\n${txData.lpAmount} LP tokens burned\nReceived ${txData.token1} and ${txData.token2}`;
              break;

            default:
              memoData = `Percolator CLI Transaction`;
              successMessage = `✅ Transaction executed successfully!`;
          }

          const transaction = new Transaction();

          // Add memo instruction with transaction details
          const memoInstruction = new TransactionInstruction({
            keys: [],
            programId: new SolanaPublicKey('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr'),
            data: Buffer.from(memoData, 'utf-8'),
          });
          transaction.add(memoInstruction);

          // Get recent blockhash
          const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
          transaction.recentBlockhash = blockhash;
          transaction.feePayer = publicKey;

          // Sign transaction with Phantom
          const signedTx = await signTransaction(transaction);
          const signature = await connection.sendRawTransaction(signedTx.serialize());

          // Wait for confirmation
          await connection.confirmTransaction({
            signature,
            blockhash,
            lastValidBlockHeight
          });

          setHistory(prev => [...prev, {
            cmd: `[Transaction Confirmed]`,
            output: `${successMessage}\n\n━━━━━━━━━━━━━━━━━━━━━━\nSignature:\n${signature}\n\n🔗 View on Solana Explorer:\nhttps://explorer.solana.com/tx/${signature}?cluster=custom&customUrl=http://localhost:8899\n\n⚠️ v0 Demo Mode: Transaction sent on-chain\n(Program integration in development)`,
            timestamp: new Date(),
            success: true,
          }]);
          setIsExecuting(false);
          return;
        } catch (txError) {
          const errorMsg = txError instanceof Error ? txError.message : 'Unknown error';
          let userFriendlyError = errorMsg;

          if (errorMsg.includes('User rejected')) {
            userFriendlyError = 'You cancelled the transaction in Phantom wallet.';
          } else if (errorMsg.includes('insufficient')) {
            userFriendlyError = 'Insufficient SOL for transaction fees. Need ~0.000005 SOL for fees.';
          }

          setHistory(prev => [...prev, {
            cmd: '[Transaction Error]',
            output: `❌ Transaction failed: ${userFriendlyError}\n\nMake sure your wallet is connected and has SOL for transaction fees.`,
            timestamp: new Date(),
            success: false,
          }]);
          setIsExecuting(false);
          return;
        }
      } else {
        // Normal command output
        setHistory(prev => [...prev, {
          cmd,
          output: data.output || data.error || 'Command executed',
          timestamp: new Date(),
          success: data.success !== false,
        }]);
      }
    } catch (error) {
      setHistory(prev => [...prev, {
        cmd,
        output: `Error: ${error instanceof Error ? error.message : 'Failed to execute command'}`,
        timestamp: new Date(),
        success: false,
      }]);
    } finally {
      setIsExecuting(false);
      setCommand('');
    }
  };

  const askAI = async () => {
    if (!aiQuestion.trim() || aiLoading) return;

    setAiLoading(true);
    setAiResponse(null);

    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:5001';
      const response = await fetch(`${apiUrl}/api/ai-assistant/suggest`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          question: aiQuestion,
        }),
      });

      const data = await response.json();
      setAiResponse({
        command: data.suggestion,
        explanation: data.explanation,
        example: data.example,
      });
    } catch (error) {
      console.error('AI Assistant error:', error);
      setAiResponse({
        command: '--help',
        explanation: 'Sorry, I had trouble understanding that. Try asking about: initialize, create slab, place order, etc.',
        example: '--help',
      });
    } finally {
      setAiLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      executeCommand(command);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (commandHistory.length > 0) {
        const newIndex = historyIndex === -1 ? commandHistory.length - 1 : Math.max(0, historyIndex - 1);
        setHistoryIndex(newIndex);
        setCommand(commandHistory[newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex !== -1) {
        const newIndex = Math.min(commandHistory.length - 1, historyIndex + 1);
        if (newIndex === commandHistory.length - 1 && historyIndex === newIndex) {
          setHistoryIndex(-1);
          setCommand('');
        } else {
          setHistoryIndex(newIndex);
          setCommand(commandHistory[newIndex]);
        }
      }
    }
  };

  const copyExample = (cmd: string) => {
    setCommand(cmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    inputRef.current?.focus();
  };

  const exampleCommands = [
    { cmd: 'buy 10 SOL', desc: 'Buy 10 SOL at market price' },
    { cmd: 'sell 5 SOL limit 140', desc: 'Sell 5 SOL at $140 limit order' },
    { cmd: 'swap 10 SOL USDC', desc: 'Swap 10 SOL for USDC via AMM' },
    { cmd: 'add-lp 10 SOL 1400 USDC', desc: 'Add liquidity to SOL-USDC pool' },
    { cmd: 'remove-lp 100 SOL USDC', desc: 'Remove 100 LP tokens from pool' },
    { cmd: 'programs', desc: 'Show deployed program IDs' },
    { cmd: 'airdrop 10', desc: 'Airdrop 10 SOL (localnet only)' },
    { cmd: '--help', desc: 'Show all available commands' },
    { cmd: 'interactive', desc: 'Launch interactive menu' },
    { cmd: 'clear', desc: 'Clear the terminal screen' },
  ];

  return (
    <div className="min-h-screen bg-gray-950 text-white flex flex-col">
      <TestnetBanner />
      
      {/* Header */}
      <header className="border-b border-white/5 bg-zinc-950/80 backdrop-blur-md sticky top-0 z-50">
        <div className="px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Link href="/" className="group">
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 hover:bg-zinc-800 transition-all">
                  <Home className="w-4 h-4 text-zinc-400 group-hover:text-white" />
                  <span className="text-sm font-medium text-zinc-400 group-hover:text-white hidden sm:inline">Home</span>
                </div>
              </Link>
              
              <div className="h-6 w-[1px] bg-white/5 hidden sm:block" />
              
              <div className="flex items-center gap-3">
                <div className="p-1.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20">
                  <Terminal className="w-4 h-4 text-emerald-400" />
                </div>
                <h1 className="text-lg font-bold tracking-tight text-zinc-100">
                  CLI Interface
                </h1>
                <span className="px-2 py-0.5 text-[10px] font-bold tracking-wide bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded-full uppercase">
                  Devnet
                </span>
              </div>
            </div>
            <WalletMultiButton className="!bg-zinc-900 !border !border-white/10 !rounded-full !h-9 !px-4 !text-sm !font-medium hover:!bg-zinc-800 !transition-all" />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex flex-col lg:flex-row gap-6 p-6 max-w-[1800px] mx-auto w-full">
        {/* Terminal */}
        <div className="flex-1 flex flex-col bg-zinc-950 rounded-xl border border-white/5 overflow-hidden shadow-2xl shadow-black/50 ring-1 ring-white/5 relative group">
          {/* Terminal Header */}
          <div className="bg-zinc-900/50 backdrop-blur-md px-4 py-3 border-b border-white/5 flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="flex gap-2">
                <div className="w-3 h-3 rounded-full bg-[#ff5f56] border border-[#e0443e] shadow-inner"></div>
                <div className="w-3 h-3 rounded-full bg-[#ffbd2e] border border-[#dea123] shadow-inner"></div>
                <div className="w-3 h-3 rounded-full bg-[#27c93f] border border-[#1aab29] shadow-inner"></div>
              </div>
              <div className="h-4 w-[1px] bg-white/5" />
              <div className="flex items-center gap-2">
                <Terminal className="w-3.5 h-3.5 text-zinc-500" />
                <span className="text-xs font-medium text-zinc-400">percolator-cli — -n localnet</span>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <span className="text-[10px] text-zinc-500 font-mono hidden sm:inline-block">v0.1.0</span>
              <div className={`flex items-center gap-1.5 px-2 py-1 rounded-full border ${
                wsConnected 
                  ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-400' 
                  : 'bg-amber-500/10 border-amber-500/20 text-amber-400'
              }`}>
                <div className={`w-1.5 h-1.5 rounded-full ${wsConnected ? 'bg-emerald-500 animate-pulse' : 'bg-amber-500'}`} />
                <span className="text-[10px] font-bold tracking-wide uppercase">
                  {wsConnected ? 'Online' : 'Connecting'}
                </span>
              </div>
            </div>
          </div>
          
          {/* Terminal Body */}
          <div 
            ref={terminalRef}
            className="flex-1 p-4 overflow-y-auto font-mono text-sm bg-black/50 scrollbar-thin scrollbar-thumb-zinc-800 scrollbar-track-transparent"
            onClick={() => inputRef.current?.focus()}
          >
            {/* Welcome Message */}
            <div className="mb-6 pb-6 border-b border-white/5">
              <div className="text-emerald-400 font-bold mb-2 text-base">Welcome to Percolator CLI Interface</div>
              <div className="grid grid-cols-2 gap-x-8 gap-y-1 text-zinc-500 text-xs max-w-md">
                <div className="flex justify-between"><span>Environment:</span> <span className="text-zinc-400">WebAssembly / Browser</span></div>
                <div className="flex justify-between"><span>Network:</span> <span className="text-zinc-400">Devnet</span></div>
                <div className="flex justify-between"><span>Wallet:</span> <span className="text-zinc-400">{publicKey ? `${publicKey.toBase58().slice(0,8)}...${publicKey.toBase58().slice(-8)}` : 'Not Connected'}</span></div>
                <div className="flex justify-between"><span>Status:</span> <span className="text-zinc-400">Ready</span></div>
              </div>
              <div className="mt-4 text-xs text-zinc-600 italic">
                Type <span className="text-emerald-500 bg-emerald-500/10 px-1 rounded">--help</span> for a list of commands or try the AI Assistant.
              </div>
            </div>

            {/* Command History */}
            <div className="space-y-4">
              {history.map((entry, idx) => (
                <div key={idx} className="group/cmd">
                  <div className="flex items-start gap-3">
                    <span className="text-emerald-500 font-bold select-none mt-0.5">➜</span>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-3">
                        <span className="text-zinc-100 font-medium">{entry.cmd}</span>
                        <span className="text-[10px] text-zinc-600 select-none opacity-0 group-hover/cmd:opacity-100 transition-opacity">
                          {entry.timestamp.toLocaleTimeString()}
                        </span>
                      </div>
                      <div className={`mt-2 whitespace-pre-wrap leading-relaxed ${
                        entry.success ? 'text-zinc-400' : 'text-red-400 bg-red-500/5 p-2 rounded border border-red-500/10'
                      }`}>
                        {entry.output}
                      </div>
                    </div>
                  </div>
                </div>
              ))}

              {/* Executing indicator */}
              {isExecuting && (
                <div className="flex items-center gap-3">
                   <span className="text-emerald-500 font-bold select-none">➜</span>
                   <div className="flex items-center gap-2 text-amber-400">
                    <div className="w-2 h-2 bg-amber-400 rounded-full animate-ping" />
                    <span className="text-sm">Processing transaction...</span>
                  </div>
                </div>
              )}

              {/* Input Line */}
              <div className="flex items-center gap-3 pt-2 sticky bottom-0 bg-transparent">
                <span className="text-emerald-500 font-bold select-none text-base animate-pulse">➜</span>
                <input
                  ref={inputRef}
                  type="text"
                  value={command}
                  onChange={(e) => setCommand(e.target.value)}
                  onKeyDown={handleKeyDown}
                  className="flex-1 bg-transparent border-none outline-none text-zinc-100 placeholder-zinc-700 font-medium text-base disabled:opacity-50 caret-emerald-500"
                  placeholder="Enter command..."
                  disabled={isExecuting}
                  autoFocus
                  autoComplete="off"
                  spellCheck="false"
                />
              </div>
            </div>
          </div>
        </div>

        {/* Sidebar */}
        <div className="lg:w-[400px] space-y-6 shrink-0">
          {/* AI Assistant */}
          <div className="bg-zinc-900/50 rounded-xl border border-violet-500/20 p-5 relative overflow-hidden">
            <div className="absolute inset-0 bg-gradient-to-br from-violet-500/5 to-blue-500/5 pointer-events-none" />
            
            <div className="flex items-center gap-2 mb-4 relative z-10">
              <div className="p-1.5 bg-violet-500/10 rounded-lg border border-violet-500/20">
                <Sparkles className="w-4 h-4 text-violet-400" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-zinc-100">AI Assistant</h3>
                <p className="text-[10px] text-zinc-500">Powered by GPT-4</p>
              </div>
            </div>
            
            <div className="space-y-3 relative z-10">
              <div className="relative group">
                <input
                  type="text"
                  value={aiQuestion}
                  onChange={(e) => setAiQuestion(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !aiLoading) {
                      askAI();
                    }
                  }}
                  placeholder="How do I place a limit order?"
                  className="w-full px-4 py-3 pr-12 bg-zinc-950 border border-white/10 rounded-xl text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-violet-500/50 focus:ring-1 focus:ring-violet-500/20 transition-all"
                  disabled={aiLoading}
                />
                <button
                  onClick={askAI}
                  disabled={aiLoading || !aiQuestion.trim()}
                  className="absolute right-2 top-1/2 -translate-y-1/2 p-2 rounded-lg bg-violet-500 hover:bg-violet-600 text-white disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-lg shadow-violet-500/20"
                >
                  {aiLoading ? (
                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  ) : (
                    <Send className="w-3.5 h-3.5" />
                  )}
                </button>
              </div>
              
              {aiResponse && !aiLoading && (
                <div className="p-4 bg-zinc-950 rounded-xl border border-violet-500/30 space-y-3 animate-in fade-in slide-in-from-top-2">
                  <p className="text-xs text-zinc-400 leading-relaxed border-l-2 border-violet-500/30 pl-3">
                    {aiResponse.explanation}
                  </p>
                  
                  <div className="space-y-2">
                    <div
                      onClick={() => {
                        setCommand(aiResponse.example);
                        inputRef.current?.focus();
                      }}
                      className="p-3 bg-zinc-900 rounded-lg border border-white/5 hover:border-violet-500/50 cursor-pointer transition-all group"
                    >
                      <code className="text-xs text-emerald-400 font-mono break-all group-hover:text-emerald-300 flex gap-2">
                        <span className="text-zinc-600 shrink-0">$</span>
                        {aiResponse.example}
                      </code>
                    </div>
                    <button
                      onClick={() => {
                        setCommand(aiResponse.example);
                        inputRef.current?.focus();
                      }}
                      className="w-full py-2 bg-violet-500/10 hover:bg-violet-500/20 text-violet-300 text-xs font-medium rounded-lg border border-violet-500/20 transition-all flex items-center justify-center gap-2"
                    >
                      <Terminal className="w-3 h-3" />
                      Use Command
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Example Commands */}
          <div className="bg-zinc-900/50 rounded-xl border border-white/5 p-5">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-bold text-zinc-100 flex items-center gap-2">
                <Terminal className="w-4 h-4 text-zinc-500" />
                Quick Commands
              </h3>
              <span className="text-[10px] text-zinc-500 bg-zinc-900 px-2 py-0.5 rounded border border-white/5">Click to use</span>
            </div>
          
            <div className="grid gap-2">
              {exampleCommands.map((example, idx) => (
                <button
                  key={idx}
                  className="flex items-center justify-between p-3 bg-zinc-950 hover:bg-zinc-900 border border-white/5 hover:border-emerald-500/30 rounded-lg transition-all group text-left"
                  onClick={() => copyExample(example.cmd)}
                >
                  <div className="flex flex-col min-w-0">
                    <code className="text-xs text-emerald-500/80 group-hover:text-emerald-400 font-mono mb-0.5 truncate">
                      {example.cmd}
                    </code>
                    <span className="text-[10px] text-zinc-500 group-hover:text-zinc-400 truncate">
                      {example.desc}
                    </span>
                  </div>
                  {copied ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500 shrink-0" />
                  ) : (
                    <Copy className="w-3.5 h-3.5 text-zinc-700 group-hover:text-emerald-500/50 shrink-0 transition-colors" />
                  )}
                </button>
              ))}
            </div>

            <div className="mt-4 pt-4 border-t border-white/5">
              <div className="p-3 bg-blue-500/5 border border-blue-500/10 rounded-lg">
                <h4 className="text-xs font-semibold text-blue-400 mb-2 flex items-center gap-1.5">
                  <span className="w-1 h-1 rounded-full bg-blue-400" />
                  Quick Tips
                </h4>
                <div className="space-y-1.5">
                  <div className="flex items-baseline gap-2 text-[10px] text-zinc-400">
                    <code className="bg-blue-500/10 px-1 rounded text-blue-300 min-w-[45px] text-center">--help</code>
                    <span>View options for any command</span>
                  </div>
                  <div className="flex items-baseline gap-2 text-[10px] text-zinc-400">
                    <code className="bg-blue-500/10 px-1 rounded text-blue-300 min-w-[45px] text-center">init</code>
                    <span>Initialize a new exchange instance</span>
                  </div>
                  <div className="flex items-baseline gap-2 text-[10px] text-zinc-400">
                    <code className="bg-blue-500/10 px-1 rounded text-blue-300 min-w-[45px] text-center">↑ / ↓</code>
                    <span>Navigate command history</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="border-t border-white/5 bg-zinc-950 mt-auto">
        <StatusFooter />
      </div>
    </div>
  );
}

