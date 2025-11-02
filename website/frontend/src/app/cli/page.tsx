"use client"

import { useState, useRef, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import Link from 'next/link';
import { ArrowLeft, Terminal, Send, Trash2, Copy, Check, Bot, Sparkles, Zap } from 'lucide-react';

export default function CLIPage() {
  const { publicKey } = useWallet();
  const [command, setCommand] = useState('');
  const [output, setOutput] = useState<Array<{type: 'command' | 'output' | 'error', text: string}>>([
    { type: 'output', text: 'Percolator CLI - Web Interface' },
    { type: 'output', text: 'Type "help" to see available commands' },
  ]);
  const [isExecuting, setIsExecuting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [wsConnected, setWsConnected] = useState(false);
  const [useRealCLI, setUseRealCLI] = useState(false); // Disabled by default - simulated mode
  const outputRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  
  // AI Assistant state
  const [aiQuestion, setAiQuestion] = useState('');
  const [aiChat, setAiChat] = useState<Array<{role: 'user' | 'assistant', text: string}>>([
    { role: 'assistant', text: 'Hi! I\'m your Percolator AI Assistant. I can help you with CLI commands, trading operations, and understanding the platform. Ask me anything!' },
  ]);
  const [isAiThinking, setIsAiThinking] = useState(false);
  const aiChatRef = useRef<HTMLDivElement>(null);

  // Connect to CLI WebSocket
  useEffect(() => {
    if (!useRealCLI) return;

    const ws = new WebSocket('ws://localhost:5001/ws/cli');
    wsRef.current = ws;

    ws.onopen = () => {
      console.log('[CLI WS] Connected');
      setWsConnected(true);
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        handleWSMessage(message);
      } catch (error) {
        console.error('[CLI WS] Failed to parse message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('[CLI WS] Error:', error);
      setWsConnected(false);
      setOutput(prev => [...prev, {
        type: 'error',
        text: 'Backend not connected. Using simulated commands. (Start backend: cd website/api && npm run dev)'
      }]);
    };

    ws.onclose = () => {
      console.log('[CLI WS] Disconnected');
      setWsConnected(false);
      setOutput(prev => [...prev, {
        type: 'error',
        text: 'Disconnected from CLI backend. Using simulated commands instead.'
      }]);
    };

    return () => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    };
  }, [useRealCLI]);

  // Auto-scroll to bottom when output changes
  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [output]);

  // Auto-scroll AI chat when messages change
  useEffect(() => {
    if (aiChatRef.current) {
      aiChatRef.current.scrollTop = aiChatRef.current.scrollHeight;
    }
  }, [aiChat]);

  const availableCommands = {
    help: 'Show available commands',
    portfolio: 'Show portfolio status',
    balance: 'Check SOL balance',
    markets: 'List available markets',
    orders: 'Show open orders',
    history: 'Show trade history',
    deposit: 'Deposit funds',
    withdraw: 'Withdraw funds',
    trade: 'Place a trade',
    lp: 'LP operations',
    clear: 'Clear terminal',
  };

  const handleWSMessage = (message: any) => {
    if (message.type === 'output') {
      setOutput(prev => [...prev, {
        type: 'output',
        text: message.data
      }]);
    } else if (message.type === 'error') {
      setOutput(prev => [...prev, {
        type: 'error',
        text: message.data
      }]);
    } else if (message.type === 'complete') {
      setIsExecuting(false);
      setOutput(prev => [...prev, {
        type: 'output',
        text: `\n${message.data}`
      }]);
    }
  };

  const handleCommand = async (cmd: string) => {
    const trimmedCmd = cmd.trim();
    
    // Add command to output
    setOutput(prev => [...prev, { type: 'command', text: `$ ${cmd}` }]);
    setIsExecuting(true);

    // Handle 'clear' locally
    if (trimmedCmd.toLowerCase() === 'clear') {
      setOutput([]);
      setIsExecuting(false);
      setCommand('');
      return;
    }

    // If using real CLI and connected, send to WebSocket
    if (useRealCLI && wsConnected && wsRef.current) {
      try {
        wsRef.current.send(JSON.stringify({
          type: 'command',
          command: trimmedCmd,
          options: {
            network: 'devnet',
            // keypair: publicKey ? 'user-wallet' : undefined
          }
        }));
        setCommand('');
        // Don't set isExecuting to false here - wait for complete message
        return;
      } catch (error: any) {
        setOutput(prev => [...prev, {
          type: 'error',
          text: `Failed to send command: ${error.message}`
        }]);
        setIsExecuting(false);
        setCommand('');
        return;
      }
    }

    // Fallback to simulated commands if not connected
    await new Promise(resolve => setTimeout(resolve, 300));

    let response: string[] = [];

    switch (trimmedCmd.toLowerCase()) {
      case 'version':
        response = [
          'Percolator CLI v1.0.0',
          'Web Interface for Percolator Protocol',
          '',
          'Solana Network: Devnet',
          'Mode: ' + (wsConnected ? 'Live Backend' : 'Simulated'),
          '',
          'Built with Next.js + Rust',
        ];
        break;

      case 'help':
        response = [
          'Percolator CLI - Available Commands',
          '═══════════════════════════════════════',
          '',
          'Basic:',
          '  help         - Show this help message',
          '  clear        - Clear terminal',
          '  version      - Show CLI version',
          '',
          'Portfolio:',
          '  portfolio    - Show portfolio status',
          '  balance      - Check SOL balance',
          '  deposit      - Deposit funds',
          '  withdraw     - Withdraw funds',
          '',
          'Trading:',
          '  markets      - List available markets',
          '  trade        - Place a trade',
          '  orders       - Show open orders',
          '  history      - Show trade history',
          '',
          'Liquidity:',
          '  lp           - LP operations',
          '',
          'Status:',
          wsConnected ? '✓ Connected to Rust CLI backend' : '⚠ Simulated mode (demo commands)',
          '',
          'For full CLI: cargo build --bin percolator',
        ];
        break;

      case 'portfolio':
        if (!publicKey) {
          response = ['Error: Please connect your wallet first'];
          setOutput(prev => [...prev, { type: 'error', text: response.join('\n') }]);
        } else {
          response = [
            'Portfolio Status:',
            '─────────────────────────────',
            `Wallet: ${publicKey.toBase58().slice(0, 8)}...${publicKey.toBase58().slice(-8)}`,
            'Balance: Loading...',
            'Open Positions: 0',
            'Total PnL: $0.00',
            '',
            'Note: Full integration coming soon',
          ];
          setOutput(prev => [...prev, { type: 'output', text: response.join('\n') }]);
        }
        setIsExecuting(false);
        setCommand('');
        return;

      case 'balance':
        if (!publicKey) {
          response = ['Error: Please connect your wallet first'];
          setOutput(prev => [...prev, { type: 'error', text: response.join('\n') }]);
        } else {
          response = [
            'Checking balance...',
            '',
            `Wallet: ${publicKey.toBase58()}`,
            'SOL Balance: Loading...',
            '',
            'Note: Connect to backend for real data',
          ];
          setOutput(prev => [...prev, { type: 'output', text: response.join('\n') }]);
        }
        setIsExecuting(false);
        setCommand('');
        return;

      case 'markets':
        response = [
          'Available Markets:',
          '─────────────────────────────',
          '1. SOL/USDC  - $186.00  24h: +2.5%',
          '2. ETH/USDC  - $2,800   24h: +1.8%',
          '3. BTC/USDC  - $43,000  24h: +3.2%',
          '',
          'Use "trade <market>" to start trading',
        ];
        break;

      case 'orders':
        response = [
          'Open Orders:',
          '─────────────────────────────',
          'No open orders',
          '',
          'Use "trade" to place an order',
        ];
        break;

      case 'history':
        response = [
          'Trade History:',
          '─────────────────────────────',
          'No trades yet',
          '',
          'Start trading to see your history',
        ];
        break;

      case '':
        setIsExecuting(false);
        setCommand('');
        return;

      default:
        response = [
          `Command not found: ${cmd}`,
          'Type "help" to see available commands',
        ];
        setOutput(prev => [...prev, { type: 'error', text: response.join('\n') }]);
        setIsExecuting(false);
        setCommand('');
        return;
    }

    setOutput(prev => [...prev, { type: 'output', text: response.join('\n') }]);
    setIsExecuting(false);
    setCommand('');
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (command.trim() && !isExecuting) {
      handleCommand(command);
    }
  };

  const handleCopyOutput = () => {
    const text = output.map(o => {
      if (o.type === 'command') return o.text;
      return o.text;
    }).join('\n');
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleClear = () => {
    setOutput([
      { type: 'output', text: 'Percolator CLI - Web Interface' },
      { type: 'output', text: 'Type "help" to see available commands' },
      { type: 'output', text: '─────────────────────────────────────────' },
    ]);
  };

  const handleAiQuestion = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!aiQuestion.trim() || isAiThinking) return;

    const userQuestion = aiQuestion.trim();
    setAiChat(prev => [...prev, { role: 'user', text: userQuestion }]);
    setAiQuestion('');
    setIsAiThinking(true);

    // Gather context for the AI
    const context = {
      wallet: {
        connected: !!publicKey,
        address: publicKey?.toBase58(),
      },
      cliHistory: output.slice(-10).map(o => o.text), // Last 10 commands/outputs
      availableCommands: Object.keys(availableCommands),
      timestamp: new Date().toISOString(),
    };

    try {
      // Option 1: Call your backend AI endpoint
      // const response = await fetch('/api/ai/chat', {
      //   method: 'POST',
      //   headers: { 'Content-Type': 'application/json' },
      //   body: JSON.stringify({
      //     question: userQuestion,
      //     context: context,
      //     chatHistory: aiChat.slice(-5), // Last 5 messages for conversation context
      //   }),
      // });
      // const data = await response.json();
      // const aiResponse = data.response;

      // Option 2: Use this smart rule-based system (current implementation)
      const aiResponse = await generateAiResponse(userQuestion, context);
      
      setAiChat(prev => [...prev, { role: 'assistant', text: aiResponse }]);
    } catch (error) {
      console.error('AI error:', error);
      setAiChat(prev => [...prev, { 
        role: 'assistant', 
        text: 'Sorry, I encountered an error. Please try again or rephrase your question.' 
      }]);
    } finally {
      setIsAiThinking(false);
    }
  };

  const generateAiResponse = async (userQuestion: string, context: any): Promise<string> => {
    // Simulate AI thinking delay
    await new Promise(resolve => setTimeout(resolve, 800));

    let aiResponse = '';
    const lowerQ = userQuestion.toLowerCase();

    // Context-aware responses
    const walletStatus = context.wallet.connected 
      ? `✓ Your wallet is connected (${context.wallet.address?.slice(0, 8)}...)`
      : '✗ Your wallet is NOT connected. Please connect your wallet first.';

    if (lowerQ.includes('help') || lowerQ.includes('how')) {
      if (lowerQ.includes('portfolio')) {
        aiResponse = `To check your portfolio, use the "portfolio" command.\n\n${walletStatus}\n\nThis command will show:\n• Your wallet address\n• SOL balance\n• Open positions\n• Total equity\n\nExample: Just type "portfolio" in the terminal above.`;
      } else if (lowerQ.includes('trade') || lowerQ.includes('order')) {
        aiResponse = `To place a trade:\n\n${walletStatus}\n\nUse: "trade buy SOL 0.1" or "trade sell ETH 0.5"\n\nOr use the dashboard trading panel for a visual interface.\n\nMake sure:\n✓ Portfolio is initialized\n✓ Sufficient balance\n✓ Amount aligns with lot size`;
      } else if (lowerQ.includes('balance')) {
        aiResponse = `${walletStatus}\n\nTo check your balance:\n• "balance" - Shows SOL balance\n• "portfolio" - Shows full portfolio details\n\nYou can also check the dashboard for a visual overview.`;
      } else if (lowerQ.includes('market')) {
        aiResponse = 'Use "markets" to list all available trading pairs.\n\nCurrently supported:\n• SOL/USDC\n• ETH/USDC\n• BTC/USDC\n\nEach market has its own orderbook, tick size, and lot size.';
      } else {
        aiResponse = `${walletStatus}\n\nI can help you with:\n• Portfolio management\n• Trading operations\n• Balance checks\n• Market info\n• Deposits/withdrawals\n• LP operations\n\nTry typing "help" in the terminal above for all commands.`;
      }
    } else if (lowerQ.includes('wallet') || lowerQ.includes('connect')) {
      aiResponse = context.wallet.connected
        ? `Your wallet is already connected!\n\nAddress: ${context.wallet.address}\n\nYou can now use commands like:\n• portfolio\n• balance\n• trade\n• deposit`
        : 'To connect your wallet:\n\n1. Click the wallet button in the top right\n2. Select your wallet (Phantom, Solflare, etc.)\n3. Approve the connection\n\nOnce connected, you can use all CLI commands.';
    } else if (lowerQ.includes('command')) {
      aiResponse = `Available commands:\n${context.availableCommands.join(', ')}\n\nType "help" in the terminal for detailed descriptions.\n\n${walletStatus}`;
    } else if (lowerQ.includes('what is') || lowerQ.includes('explain')) {
      if (lowerQ.includes('slab') || lowerQ.includes('orderbook')) {
        aiResponse = 'A slab is an on-chain orderbook where:\n\n• Liquidity providers POST limit orders\n• Takers EXECUTE against those orders\n• Each market has its own slab\n• Tick size = minimum price increment\n• Lot size = minimum quantity increment\n\nAll trades are recorded on-chain and transparent!';
      } else if (lowerQ.includes('tick size')) {
        aiResponse = 'Tick size = minimum price increment\n\nExample:\nIf tick size is $0.01, you can only place orders at:\n• $100.00 ✓\n• $100.01 ✓\n• $100.015 ✗ (invalid)\n\nThis ensures price consistency across the orderbook.';
      } else if (lowerQ.includes('lot size')) {
        aiResponse = 'Lot size = minimum quantity increment\n\nExample:\nIf lot size is 0.01 SOL, you can trade:\n• 0.01 SOL ✓\n• 0.05 SOL ✓\n• 0.015 SOL ✗ (invalid)\n\nThis ensures quantity consistency across the orderbook.';
      } else if (lowerQ.includes('percolator')) {
        aiResponse = 'Percolator is a decentralized perpetual futures DEX on Solana.\n\nKey features:\n• On-chain orderbooks (slabs)\n• AMM for instant liquidity\n• Real-time oracle prices\n• Fully transparent trades\n• Low-latency execution\n\nAll operations are on-chain and verifiable!';
      } else {
        aiResponse = 'I can explain:\n• Slabs & orderbooks\n• Tick size & lot size\n• LP operations\n• Trading mechanics\n• Percolator architecture\n\nWhat would you like to know more about?';
      }
    } else if (lowerQ.includes('error') || lowerQ.includes('fail') || lowerQ.includes('problem')) {
      const recentErrors = context.cliHistory.filter((h: string) => h.toLowerCase().includes('error') || h.toLowerCase().includes('fail'));
      const troubleshootingSteps = `Troubleshooting checklist:\n\n${walletStatus}\n\n✓ Check: Enough SOL for gas fees?\n✓ Check: Portfolio initialized?\n✓ Check: Valid amounts (tick/lot size)?\n✓ Check: Backend services running?\n\nRun "portfolio" to see your current status.`;
      
      if (recentErrors.length > 0) {
        aiResponse = `I see you had recent errors:\n\n${recentErrors[0]}\n\n${troubleshootingSteps}`;
      } else {
        aiResponse = troubleshootingSteps;
      }
    } else if (lowerQ.includes('price') || lowerQ.includes('market')) {
      aiResponse = 'To check prices:\n\n• "markets" - See all trading pairs & prices\n• Dashboard - View live charts\n\nPrices update in real-time from our oracle.\n\nSupported pairs:\n• SOL/USDC\n• ETH/USDC\n• BTC/USDC';
    } else if (lowerQ.includes('history')) {
      const recentCommands = context.cliHistory.filter((h: string) => h.startsWith('$'));
      if (recentCommands.length > 0) {
        aiResponse = `Your recent commands:\n\n${recentCommands.slice(-5).join('\n')}\n\nType "history" to see your full trade history.`;
      } else {
        aiResponse = 'No command history yet.\n\nType "history" to see your trade history once you start trading.';
      }
    } else {
      aiResponse = `${walletStatus}\n\nI'm your Percolator AI assistant! I can help with:\n\n• CLI commands & usage\n• Trading concepts\n• Troubleshooting issues\n• Platform features\n\nTry asking:\n• "How do I check my portfolio?"\n• "What is a slab?"\n• "Help me place a trade"\n• "Explain tick size"`;
    }

    return aiResponse;
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-black via-gray-900 to-black text-white">
      {/* Header */}
      <header className="border-b border-gray-800 bg-black/50 backdrop-blur-sm sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 py-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link href="/dashboard">
              <button className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 transition-colors">
                <ArrowLeft className="w-4 h-4" />
                <span className="text-sm font-medium">Back to Dashboard</span>
              </button>
            </Link>
            <div className="flex items-center gap-3">
              <Terminal className="w-6 h-6 text-green-400" />
              <h1 className="text-xl font-bold">CLI Terminal</h1>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleCopyOutput}
              className="px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 transition-colors flex items-center gap-2"
            >
              {copied ? <Check className="w-4 h-4 text-green-400" /> : <Copy className="w-4 h-4" />}
              <span className="text-sm">{copied ? 'Copied!' : 'Copy'}</span>
            </button>
            <button
              onClick={handleClear}
              className="px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 transition-colors flex items-center gap-2"
            >
              <Trash2 className="w-4 h-4" />
              <span className="text-sm">Clear</span>
            </button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 py-8">
        {/* Terminal Window */}
        <div className="bg-black border border-gray-700 rounded-lg shadow-2xl overflow-hidden">
          {/* Terminal Header */}
          <div className="bg-gray-800 px-4 py-3 flex items-center justify-between border-b border-gray-700">
            <div className="flex items-center gap-2">
              <div className="flex gap-2">
                <div className="w-3 h-3 rounded-full bg-red-500"></div>
                <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                <div className="w-3 h-3 rounded-full bg-green-500"></div>
              </div>
              <span className="ml-4 text-sm text-gray-400 font-mono">percolator-cli</span>
              {wsConnected && (
                <div className="ml-2 flex items-center gap-1 px-2 py-0.5 bg-green-500/20 border border-green-500/30 rounded text-xs text-green-400">
                  <Zap className="w-3 h-3" />
                  <span>LIVE</span>
                </div>
              )}
            </div>
            <div className="flex items-center gap-4">
              {publicKey && (
                <div className="text-xs text-gray-400 font-mono">
                  Wallet: {publicKey.toBase58().slice(0, 8)}...
                </div>
              )}
              <div className={`text-xs font-mono px-2 py-1 rounded ${wsConnected ? 'bg-green-500/20 text-green-400' : 'bg-gray-500/20 text-gray-400'}`}>
                {wsConnected ? '✓ Backend Connected' : 'Simulated Mode'}
              </div>
              <button
                onClick={() => setUseRealCLI(!useRealCLI)}
                className="text-xs px-3 py-1 rounded bg-blue-600 hover:bg-blue-700 transition-colors"
                title={useRealCLI ? 'Disable backend connection' : 'Enable backend connection'}
              >
                {useRealCLI ? 'Backend: ON' : 'Backend: OFF'}
              </button>
            </div>
          </div>

          {/* Terminal Output */}
          <div 
            ref={outputRef}
            className="bg-black p-6 font-mono text-sm h-[500px] overflow-y-auto"
            style={{
              scrollbarWidth: 'thin',
              scrollbarColor: '#374151 #000000',
            }}
          >
            {output.map((line, idx) => (
              <div key={idx} className="mb-2">
                {line.type === 'command' && (
                  <div className="text-green-400">{line.text}</div>
                )}
                {line.type === 'output' && (
                  <div className="text-gray-300 whitespace-pre-wrap">{line.text}</div>
                )}
                {line.type === 'error' && (
                  <div className="text-red-400 whitespace-pre-wrap">{line.text}</div>
                )}
              </div>
            ))}
            {isExecuting && (
              <div className="text-gray-500 flex items-center gap-2">
                <span>Processing</span>
                <span className="animate-pulse">...</span>
              </div>
            )}
          </div>

          {/* Input Area */}
          <form onSubmit={handleSubmit} className="bg-gray-900 border-t border-gray-700 p-4">
            <div className="flex items-center gap-3">
              <span className="text-green-400 font-mono">$</span>
              <input
                type="text"
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder="Type a command... (try 'help')"
                className="flex-1 bg-transparent text-white font-mono outline-none placeholder-gray-500"
                disabled={isExecuting}
                autoFocus
              />
              <button
                type="submit"
                disabled={isExecuting || !command.trim()}
                className="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 disabled:cursor-not-allowed rounded-lg transition-colors flex items-center gap-2"
              >
                <Send className="w-4 h-4" />
                <span className="text-sm font-medium">Execute</span>
              </button>
            </div>
          </form>
        </div>

        {/* Info Cards */}
        <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-gradient-to-br from-gray-900 to-gray-800 border border-gray-700 rounded-lg p-4">
            <h3 className="text-sm font-bold text-gray-400 mb-2">Quick Commands</h3>
            <div className="space-y-1 text-xs text-gray-300 font-mono">
              <div>help - Show all commands</div>
              <div>portfolio - View portfolio</div>
              <div>markets - List markets</div>
              <div>clear - Clear terminal</div>
            </div>
          </div>

          <div className="bg-gradient-to-br from-gray-900 to-gray-800 border border-gray-700 rounded-lg p-4">
            <h3 className="text-sm font-bold text-gray-400 mb-2">Status</h3>
            <div className="space-y-2 text-xs">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-green-500"></div>
                <span className="text-gray-300">CLI Ready</span>
              </div>
              <div className="flex items-center gap-2">
                <div className={`w-2 h-2 rounded-full ${publicKey ? 'bg-green-500' : 'bg-yellow-500'}`}></div>
                <span className="text-gray-300">{publicKey ? 'Wallet Connected' : 'Wallet Not Connected'}</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-yellow-500"></div>
                <span className="text-gray-300">Backend: Coming Soon</span>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-br from-blue-900/20 to-gray-800 border border-blue-800/50 rounded-lg p-4">
            <h3 className="text-sm font-bold text-blue-400 mb-2">Coming Soon</h3>
            <div className="space-y-1 text-xs text-gray-300">
              <div>Real-time CLI execution</div>
              <div>WebSocket integration</div>
              <div>Command history</div>
              <div>Advanced trading ops</div>
            </div>
          </div>
        </div>

        {/* Note */}
        <div className="mt-6 bg-yellow-900/20 border border-yellow-800/50 rounded-lg p-4">
          <p className="text-sm text-yellow-200">
            <span className="font-bold">Note:</span> This is a web-based CLI interface. For full CLI functionality,
            use the native Rust CLI: <code className="bg-black/50 px-2 py-1 rounded text-xs">cargo run --bin cli -- --help</code>
          </p>
        </div>

        {/* AI Assistant */}
        <div className="mt-8 bg-gradient-to-br from-purple-900/30 to-gray-800 border border-purple-700/50 rounded-lg overflow-hidden">
          {/* AI Header */}
          <div className="bg-purple-900/40 border-b border-purple-700/50 px-4 py-3 flex items-center gap-3">
            <div className="flex items-center justify-center w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-pink-500">
              <Sparkles className="w-5 h-5 text-white" />
            </div>
            <div className="flex-1">
              <h3 className="text-lg font-bold bg-gradient-to-r from-purple-400 to-pink-400 bg-clip-text text-transparent">
                AI Assistant
              </h3>
              <p className="text-xs text-gray-400">Ask me anything about Percolator</p>
            </div>
            <div className={`px-3 py-1 rounded-full text-xs font-medium ${isAiThinking ? 'bg-yellow-500/20 text-yellow-300' : 'bg-green-500/20 text-green-300'}`}>
              {isAiThinking ? 'Thinking...' : 'Ready'}
            </div>
          </div>

          {/* AI Chat Area */}
          <div 
            ref={aiChatRef}
            className="bg-black/30 p-4 h-[300px] overflow-y-auto"
            style={{
              scrollbarWidth: 'thin',
              scrollbarColor: '#7c3aed #000000',
            }}
          >
            {aiChat.map((message, idx) => (
              <div key={idx} className={`mb-4 flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                <div className={`max-w-[80%] ${message.role === 'user' ? 'order-2' : 'order-1'}`}>
                  <div className={`rounded-lg px-4 py-2 ${
                    message.role === 'user' 
                      ? 'bg-blue-600 text-white' 
                      : 'bg-purple-900/40 text-gray-200 border border-purple-700/50'
                  }`}>
                    {message.role === 'assistant' && (
                      <div className="flex items-center gap-2 mb-1">
                        <Bot className="w-4 h-4 text-purple-400" />
                        <span className="text-xs font-semibold text-purple-400">AI Assistant</span>
                      </div>
                    )}
                    <p className="text-sm whitespace-pre-wrap">{message.text}</p>
                  </div>
                </div>
              </div>
            ))}
            {isAiThinking && (
              <div className="flex items-start gap-2">
                <div className="bg-purple-900/40 border border-purple-700/50 rounded-lg px-4 py-2">
                  <div className="flex items-center gap-2">
                    <Bot className="w-4 h-4 text-purple-400 animate-pulse" />
                    <span className="text-sm text-gray-300">Thinking</span>
                    <span className="text-purple-400 animate-pulse">...</span>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* AI Input Area */}
          <form onSubmit={handleAiQuestion} className="bg-gray-900/50 border-t border-purple-700/50 p-4">
            <div className="flex items-center gap-3">
              <div className="flex-1 bg-black/30 rounded-lg px-4 py-2 border border-purple-700/30 focus-within:border-purple-500/50 transition-colors">
                <input
                  type="text"
                  value={aiQuestion}
                  onChange={(e) => setAiQuestion(e.target.value)}
                  placeholder="Ask the AI about commands, trading, or anything..."
                  className="w-full bg-transparent text-white text-sm outline-none placeholder-gray-500"
                  disabled={isAiThinking}
                />
              </div>
              <button
                type="submit"
                disabled={isAiThinking || !aiQuestion.trim()}
                className="px-5 py-2 bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 disabled:from-gray-700 disabled:to-gray-700 disabled:cursor-not-allowed rounded-lg transition-all flex items-center gap-2 shadow-lg"
              >
                <Send className="w-4 h-4" />
                <span className="text-sm font-medium">Ask</span>
              </button>
            </div>
            <p className="text-xs text-gray-400 mt-2">
              Try asking: "How do I check my portfolio?" or "What is a slab?" or "Help me place a trade"
            </p>
          </form>
        </div>
      </main>
    </div>
  );
}

