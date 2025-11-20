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
      const ws = new WebSocket('ws://localhost:5001/ws/cli');

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
        const response = await fetch('http://localhost:5001/api/cli/status');
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
      const response = await fetch('http://localhost:5001/api/cli/execute', {
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
      const response = await fetch('http://localhost:5001/api/ai-assistant/suggest', {
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
      <header className="border-b border-gray-800 bg-gray-900">
        <div className="px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <Link href="/">
                <button className="px-3 py-2 rounded-lg bg-[#B8B8FF]/10 hover:bg-[#B8B8FF]/20 border border-[#B8B8FF]/30 hover:border-[#B8B8FF]/50 text-[#B8B8FF] text-sm font-bold transition-all flex items-center gap-1.5">
                  <Home className="w-4 h-4" />
                  <span>Home</span>
                </button>
              </Link>
              <h1 className="text-xl font-bold bg-gradient-to-r from-emerald-400 to-teal-500 bg-clip-text text-transparent flex items-center gap-2">
                <Terminal className="w-5 h-5 text-emerald-400" />
                Percolator CLI
              </h1>
              <span className="px-2 py-1 text-xs font-semibold bg-blue-600 rounded">
                DEVNET
              </span>
            </div>
            <WalletMultiButton />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex flex-col lg:flex-row gap-4 p-4">
        {/* Terminal */}
        <div className="flex-1 flex flex-col bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
          <div className="bg-gray-800 px-4 py-2 border-b border-gray-700 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className="flex gap-1.5">
                <div className="w-3 h-3 rounded-full bg-red-500"></div>
                <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                <div className="w-3 h-3 rounded-full bg-green-500"></div>
              </div>
              <span className="text-sm text-gray-400 ml-2">Terminal</span>
              <span className="text-xs px-2 py-0.5 bg-green-500/20 text-green-400 rounded">
                CLI Ready
              </span>
            </div>
            <span className="text-xs text-gray-500">Press ↑/↓ for command history</span>
          </div>
          
          <div 
            ref={terminalRef}
            className="flex-1 p-4 overflow-y-auto font-mono text-sm"
            onClick={() => inputRef.current?.focus()}
          >
            {/* Welcome Message */}
            <div className="text-emerald-400 mb-4">
              <p>Welcome to Percolator CLI Interface</p>
              <p className="text-gray-500 text-xs mt-1">Connected to: {publicKey ? publicKey.toBase58() : 'No wallet connected'}</p>
              <p className="text-gray-500 text-xs mt-1">Type commands below or use the examples on the right →</p>
            </div>

            {/* Command History */}
            {history.map((entry, idx) => (
              <div key={idx} className="mb-3">
                <div className="flex items-start gap-2">
                  <span className={entry.success ? "text-emerald-400" : "text-red-400"}>$</span>
                  <span className="text-gray-300">{entry.cmd}</span>
                  {entry.success ? (
                    <span className="text-green-500 text-xs">✓</span>
                  ) : (
                    <span className="text-red-500 text-xs">✗</span>
                  )}
                </div>
                <div className={`ml-4 whitespace-pre-wrap mt-1 ${entry.success ? 'text-gray-400' : 'text-red-400'}`}>
                  {entry.output}
                </div>
                <div className="ml-4 text-gray-600 text-xs mt-1">
                  {entry.timestamp.toLocaleTimeString()}
                </div>
              </div>
            ))}

            {/* Executing indicator */}
            {isExecuting && (
              <div className="flex items-center gap-2 text-yellow-400 mb-3">
                <span className="animate-pulse">⏳</span>
                <span className="text-sm">Executing command...</span>
              </div>
            )}

            {/* Input Line */}
            <div className="flex items-center gap-2">
              <span className="text-emerald-400">$</span>
              <input
                ref={inputRef}
                type="text"
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                onKeyDown={handleKeyDown}
                className="flex-1 bg-transparent border-none outline-none text-gray-300 disabled:opacity-50"
                placeholder="Enter command..."
                disabled={isExecuting}
                autoFocus
              />
            </div>
          </div>
        </div>

        {/* Examples Sidebar */}
        <div className="lg:w-96 space-y-4">
          {/* AI Assistant */}
          <div className="bg-gradient-to-br from-purple-900/30 to-blue-900/30 rounded-lg border border-purple-500/30 p-4">
            <h3 className="text-lg font-bold mb-3 flex items-center gap-2">
              <Sparkles className="w-5 h-5 text-purple-400" />
              AI Assistant
            </h3>
            
            <p className="text-xs text-gray-400 mb-3">
              Ask in plain English and I'll suggest the right command!
            </p>
            
            <div className="space-y-3">
              <div className="relative">
                <input
                  type="text"
                  value={aiQuestion}
                  onChange={(e) => setAiQuestion(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !aiLoading) {
                      askAI();
                    }
                  }}
                  placeholder="e.g., How do I create a new market?"
                  className="w-full px-3 py-2 pr-10 bg-gray-800 border border-gray-700 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:border-purple-500"
                  disabled={aiLoading}
                />
                <button
                  onClick={askAI}
                  disabled={aiLoading || !aiQuestion.trim()}
                  className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded bg-purple-500 hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
                >
                  <Send className="w-4 h-4" />
                </button>
              </div>
              
              {aiLoading && (
                <div className="p-3 bg-gray-800/50 rounded border border-gray-700 text-center">
                  <div className="animate-pulse text-purple-400 text-sm">
                    🤔 Thinking...
                  </div>
                </div>
              )}
              
              {aiResponse && !aiLoading && (
                <div className="p-3 bg-gray-800 rounded border border-purple-500/50 space-y-2">
                  <div className="flex items-start gap-2">
                    <Sparkles className="w-4 h-4 text-purple-400 mt-0.5 shrink-0" />
                    <div className="flex-1 space-y-2">
                      <p className="text-xs text-gray-400">{aiResponse.explanation}</p>
                      <div
                        onClick={() => {
                          setCommand(`-n localnet ${aiResponse.example}`);
                          inputRef.current?.focus();
                        }}
                        className="p-2 bg-gray-900 rounded border border-gray-700 hover:border-emerald-500 cursor-pointer transition-all group"
                      >
                        <code className="text-xs text-emerald-400 break-all group-hover:text-emerald-300">
                          {aiResponse.example}
                        </code>
                      </div>
                      <button
                        onClick={() => {
                          setCommand(`-n localnet ${aiResponse.example}`);
                          inputRef.current?.focus();
                        }}
                        className="w-full px-3 py-1.5 bg-purple-500 hover:bg-purple-600 rounded text-xs font-semibold transition-all"
                      >
                        Copy to Terminal
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Example Commands */}
          <div className="bg-gray-900 rounded-lg border border-gray-800 p-4">
            <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
              <Terminal className="w-5 h-5 text-emerald-400" />
              Example Commands
            </h3>
          
          <div className="space-y-3">
            {exampleCommands.map((example, idx) => (
              <div 
                key={idx}
                className="p-3 bg-gray-800 rounded-lg border border-gray-700 hover:border-emerald-500/50 transition-all cursor-pointer group"
                onClick={() => copyExample(example.cmd)}
              >
                <div className="flex items-start justify-between gap-2 mb-2">
                  <code className="text-xs text-emerald-400 break-all flex-1">
                    {example.cmd}
                  </code>
                  {copied ? (
                    <CheckCircle2 className="w-4 h-4 text-green-500 shrink-0" />
                  ) : (
                    <Copy className="w-4 h-4 text-gray-500 group-hover:text-emerald-400 shrink-0" />
                  )}
                </div>
                <p className="text-xs text-gray-400">{example.desc}</p>
              </div>
            ))}
          </div>

          <div className="mt-6 p-3 bg-blue-500/10 border border-blue-500/30 rounded-lg">
            <h4 className="text-sm font-semibold text-blue-400 mb-2">💡 Quick Start</h4>
            <p className="text-xs text-gray-400 mb-2">
              Use <code className="bg-gray-800 px-1 rounded">--help</code> on any command to see its options.
            </p>
            <div className="space-y-1 text-xs text-gray-500">
              <p>• View options: <code className="bg-gray-800 px-1 rounded">command --help</code></p>
              <p>• Initialize: <code className="bg-gray-800 px-1 rounded">init --name MyExchange</code></p>
              <p>• Run tests: <code className="bg-gray-800 px-1 rounded">test --crisis</code></p>
              <p>• Interactive mode: <code className="bg-gray-800 px-1 rounded">interactive</code></p>
            </div>
          </div>
          
          <div className="mt-4 p-3 bg-emerald-500/10 border border-emerald-500/30 rounded-lg">
            <h4 className="text-sm font-semibold text-emerald-400 mb-2">🚀 Local Binary</h4>
            <p className="text-xs text-gray-400 mb-1">
              For production, use the CLI directly:
            </p>
            <code className="text-xs text-gray-500 block bg-gray-800 p-2 rounded">
              ./target/release/percolator -n localnet status MyExchange
            </code>
          </div>
          </div>
        </div>
      </div>

      <StatusFooter />
    </div>
  );
}

