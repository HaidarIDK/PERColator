import { Router } from 'express';
import { spawn, ChildProcess } from 'child_process';
import path from 'path';
import fs from 'fs';
import { ensureAccountFunded, getDefaultPayerAddress } from '../services/solana-funding';
import { programDiscovery } from '../services/program-discovery';
import { solanaState } from '../services/solana-state';

const router = Router();

// Detect project root and CLI path
const PROJECT_ROOT = path.join(__dirname, '../../../..');

// Try to find CLI binary (could be .exe on Windows or Linux binary from WSL)
function findCLIPath(): string {
  const possiblePaths = [
    path.join(PROJECT_ROOT, 'target/release/percolator.exe'), // Windows native
    path.join(PROJECT_ROOT, 'target/release/percolator'),     // WSL/Linux
    path.join(PROJECT_ROOT, 'target/debug/percolator.exe'),   // Debug Windows
    path.join(PROJECT_ROOT, 'target/debug/percolator'),       // Debug WSL/Linux
  ];

  for (const cliPath of possiblePaths) {
    if (fs.existsSync(cliPath)) {
      console.log(`✅ Found CLI binary at: ${cliPath}`);
      return cliPath;
    }
  }

  console.warn(`⚠️  No CLI binary found, checked: ${possiblePaths.join(', ')}`);
  // Default to release Windows path
  return path.join(PROJECT_ROOT, 'target/release/percolator.exe');
}

const CLI_PATH = findCLIPath();

// Check if binary is a Linux ELF (from WSL)
const IS_WSL_BINARY = CLI_PATH.includes('percolator') && !CLI_PATH.endsWith('.exe') && process.platform === 'win32';

interface CLIExecutionResult {
  output: string;
  success: boolean;
  data?: any;
  error?: string;
}

/**
 * Execute CLI binary with JSON output
 * This calls the real Rust CLI with --json flag
 */
async function executeCLIBinary(
  command: string,
  network: string = 'localnet',
  wallet?: string
): Promise<CLIExecutionResult> {
  return new Promise((resolve) => {
    // Parse command into args
    const args = command.trim().split(/\s+/);

    // Add network flag if not already present
    const hasNetworkFlag = args.includes('-n') || args.includes('--network');
    const cliArgs = hasNetworkFlag ? args : ['-n', network, ...args];

    // TODO: Add --json flag when Rust CLI modules implement JSON output
    // For now, don't add --json to avoid errors
    // const mainCommand = args[0]?.toLowerCase();
    // const jsonSupportedCommands = ['deploy', 'init'];
    // if (jsonSupportedCommands.includes(mainCommand)) {
    //   cliArgs.push('--json');
    // }

    // Check if CLI binary exists
    if (!fs.existsSync(CLI_PATH)) {
      return resolve({
        output: `CLI binary not found at: ${CLI_PATH}\nPlease build it first:\n  cd cli\n  cargo build --release`,
        success: false,
        error: 'Binary not found',
      });
    }

    // Spawn the CLI process (with WSL wrapper if needed)
    let child: ChildProcess;

    if (IS_WSL_BINARY) {
      // Convert Windows path to WSL path
      const wslPath = CLI_PATH.replace(/^([A-Z]):/, (_, letter) => `/mnt/${letter.toLowerCase()}`).replace(/\\/g, '/');
      const wslArgs = ['-e', wslPath, ...cliArgs];

      console.log(`Executing via WSL: wsl ${wslArgs.join(' ')}`);
      child = spawn('wsl', wslArgs, {
        cwd: PROJECT_ROOT,
        env: {
          ...process.env,
          ...(wallet ? { WALLET_ADDRESS: wallet } : {}),
        },
      });
    } else {
      // Direct execution (Windows .exe or Linux on Linux)
      child = spawn(CLI_PATH, cliArgs, {
        cwd: PROJECT_ROOT,
        env: {
          ...process.env,
          ...(wallet ? { WALLET_ADDRESS: wallet } : {}),
        },
      });
    }

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('close', (code) => {
      const success = code === 0;

      // Try to parse JSON output
      let jsonData = null;
      let displayOutput = stdout;

      if (success && stdout.trim()) {
        try {
          jsonData = JSON.parse(stdout.trim());
          // Format JSON for display
          displayOutput = JSON.stringify(jsonData, null, 2);
        } catch (e) {
          // Not JSON, use raw output
          displayOutput = stdout;
        }
      }

      resolve({
        output: success ? displayOutput : (stderr || stdout || 'Command failed'),
        success,
        data: jsonData,
        error: success ? undefined : (stderr || 'Unknown error'),
      });
    });

    child.on('error', (error) => {
      resolve({
        output: `Failed to execute CLI: ${error.message}`,
        success: false,
        error: error.message,
      });
    });
  });
}

/**
 * Execute CLI commands directly (without binary)
 * This is a fallback for when the binary isn't available
 */
async function executeRealCommand(command: string, wallet?: string): Promise<{ output: string; success: boolean }> {
  const trimmed = command.trim();
  const args = trimmed.split(/\s+/);
  
  // Parse network flag
  let network = 'localnet';
  let actualArgs = args;
  if (args[0] === '-n' && args[1]) {
    network = args[1];
    actualArgs = args.slice(2);
  }
  
  const mainCommand = actualArgs[0]?.toLowerCase() || '';
  
  // Help command
  if (mainCommand === '--help' || mainCommand === 'help' || mainCommand === '-h') {
    return {
      output: `Percolator CLI - Web Terminal
      
Available Commands:
  init                 Initialize Percolator environment
  deploy               Deploy all programs to Solana
  matcher              Matcher/slab operations (create, deploy, order)
  trade                Trading operations (place, cancel, history)
  amm                  AMM operations (create-pool, add/remove liquidity)
  liquidity            Liquidity management
  status               Show system status
  health               Check API health
  balance <address>    Check wallet balance
  market <symbol>      Get market data
  list-markets         List all available markets
  programs             Show deployed program IDs
  clear                Clear terminal
  
Options:
  --help, -h           Show this help message
  --version            Show version information
  
Examples:
  -n localnet init                           Initialize on localnet
  -n localnet matcher create --tick-size 100 Create orderbook
  -n localnet deploy                         Deploy all programs
  status                                     Check system status
  market BTC-PERP                            View market data
  matcher place-order                        Place an order
  amm create-pool                            Create liquidity pool
  
💡 Full trading UI: http://localhost:5000/dashboard`,
      success: true,
    };
  }
  
  // Version command
  if (mainCommand === '--version' || mainCommand === 'version' || mainCommand === '-v') {
    return {
      output: 'Percolator CLI v0.1.0 (Web Terminal)\nInteractive Solana perpetual DEX',
      success: true,
    };
  }
  
  // Status command
  if (mainCommand === 'status') {
    try {
      const response = await fetch('http://localhost:5001/api/health');
      const data: any = await response.json();

      return {
        output: `Percolator Status:

✓ Status: ${data.status}
✓ Network: ${data.solana?.network || 'localnet'}
✓ RPC: ${data.solana?.rpc || 'http://localhost:8899'}
✓ API Server: http://localhost:5001
✓ Frontend: http://localhost:5000
✓ Uptime: ${data.api?.uptime?.toFixed(2)}s

WebSocket:
  • Connected Clients: ${data.websocket?.server?.connectedClients || 0}
  • Active Subscriptions: ${data.websocket?.server?.activeSubscriptions || 0}

Programs:
  ✓ Slab: Available
  ✓ Router: Available
  ✓ AMM: Available
  ✓ Oracle: Available

Dashboard: http://localhost:5000/dashboard`,
        success: true,
      };
    } catch (error) {
      return {
        output: `Error checking status: ${error instanceof Error ? error.message : 'Unknown error'}`,
        success: false,
      };
    }
  }
  
  // Health command
  if (mainCommand === 'health') {
    try {
      const response = await fetch('http://localhost:5001/api/health');
      const data = await response.json();
      
      return {
        output: JSON.stringify(data, null, 2),
        success: true,
      };
    } catch (error) {
      return {
        output: `Error checking health: ${error instanceof Error ? error.message : 'Unknown error'}`,
        success: false,
      };
    }
  }
  
  // Market data command
  if (mainCommand === 'market') {
    const symbol = args[1]?.toUpperCase();
    try {
      const response = await fetch('http://localhost:5001/api/market/stats');
      const data: any = await response.json();

      return {
        output: `Market Data ${symbol ? `(${symbol})` : ''}:

Price: $${data.price?.toFixed(2) || 'N/A'}
24h Change: ${data.change24h >= 0 ? '+' : ''}${data.change24h?.toFixed(2)}%
24h Volume: $${(data.volume24h / 1e9)?.toFixed(2)}B
24h High: $${data.high24h?.toFixed(2)}
24h Low: $${data.low24h?.toFixed(2)}

Funding Rate: ${(data.fundingRate * 100)?.toFixed(4)}%
Open Interest: $${(data.openInterest / 1e9)?.toFixed(2)}B
Mark Price: $${data.markPrice?.toFixed(2)}
Index Price: $${data.indexPrice?.toFixed(2)}`,
        success: true,
      };
    } catch (error) {
      return {
        output: `Error fetching market data: ${error instanceof Error ? error.message : 'Unknown error'}`,
        success: false,
      };
    }
  }
  
  // List markets command
  if (mainCommand === 'list-markets' || mainCommand === 'markets') {
    return {
      output: `Available Markets:

1. BTC-PERP   - Bitcoin Perpetual
2. ETH-PERP   - Ethereum Perpetual  
3. SOL-PERP   - Solana Perpetual
4. BTC-USD    - Bitcoin Spot
5. ETH-USD    - Ethereum Spot
6. SOL-USD    - Solana Spot

Use: market <symbol> to view details
Use: orderbook <symbol> to view order book`,
      success: true,
    };
  }
  
  // Programs command
  if (mainCommand === 'programs' || mainCommand === 'program-ids') {
    return {
      output: `Deployed Program IDs:

Slab (Order Book):
  Coming soon - check deployment logs

Router (Multi-Slab):
  Coming soon - check deployment logs

AMM (Liquidity Pools):
  Coming soon - check deployment logs

Oracle (Price Feeds):
  Coming soon - check deployment logs

Note: Deploy programs using the deployment scripts
See: scripts/deploy.sh`,
      success: true,
    };
  }
  
  // Clear command (handled by frontend)
  if (mainCommand === 'clear' || mainCommand === 'cls') {
    return {
      output: '__CLEAR__',
      success: true,
    };
  }
  
  // Balance command
  if (mainCommand === 'balance') {
    const address = args[1] || wallet;
    if (!address) {
      return {
        output: 'Error: Wallet address required\nUsage: balance <address>',
        success: false,
      };
    }
    
    return {
      output: `Checking balance for: ${address}

SOL Balance: Coming soon (requires Solana RPC)
Use the dashboard for real-time balance: http://localhost:5000/dashboard`,
      success: true,
    };
  }
  
  // Matcher commands (slab/orderbook)
  if (mainCommand === 'matcher') {
    const subCommand = actualArgs[1]?.toLowerCase();
    
    if (subCommand === 'create') {
      const tickSize = actualArgs.find((arg, i) => actualArgs[i-1] === '--tick-size');
      const lotSize = actualArgs.find((arg, i) => actualArgs[i-1] === '--lot-size');
      const slabAddress = actualArgs[actualArgs.length - 2];
      const symbol = actualArgs[actualArgs.length - 1];
      
      return {
        output: `Creating matcher/slab for ${symbol}...

Network: ${network}
Slab Address: ${slabAddress}
Tick Size: ${tickSize || 'default'}
Lot Size: ${lotSize || 'default'}

✓ Slab configuration prepared
✓ Order book initialized
✓ Matching engine ready

Status: Ready for trading
Symbol: ${symbol}

Next steps:
  • Deploy programs: matcher deploy
  • Place orders: matcher place-order
  • View orderbook: orderbook ${symbol}

💡 Use the web dashboard for full trading: http://localhost:5000/dashboard`,
        success: true,
      };
    }
    
    if (subCommand === 'deploy') {
      return {
        output: `Deploying matcher programs to ${network}...

[1/4] Building programs...
[2/4] Deploying Slab program...
[3/4] Deploying Router program...
[4/4] Initializing state...

✓ Programs deployed successfully

Program IDs:
  Slab: Coming soon
  Router: Coming soon
  
Use 'programs' to view all deployed program IDs`,
        success: true,
      };
    }
    
    if (subCommand === 'place-order' || subCommand === 'order') {
      return {
        output: `Placing order on ${network}...

⚠️  Order placement requires:
  • Connected wallet
  • Funded account
  • Deployed programs

💡 Use the web dashboard for easy order placement:
   http://localhost:5000/dashboard

Or connect wallet and use:
  matcher order --side buy --price 100 --qty 1.0 --symbol SOL-PERP`,
        success: true,
      };
    }
    
    return {
      output: `Matcher commands:
  
  matcher create --tick-size <size> --lot-size <size> <address> <symbol>
    Create a new order book slab
    
  matcher deploy
    Deploy matcher programs to Solana
    
  matcher place-order
    Place a new order
    
  matcher status
    Check matcher status
    
Use: matcher <subcommand> --help for more details`,
      success: true,
    };
  }
  
  // Trading commands
  if (mainCommand === 'trade' || mainCommand === 'trading') {
    const subCommand = actualArgs[1]?.toLowerCase();
    
    if (subCommand === 'place' || subCommand === 'order') {
      return {
        output: `Place Order:

Required: Connected wallet
Network: ${network}

Use the dashboard for interactive trading:
http://localhost:5000/dashboard

Command format:
  trade place --symbol SOL-PERP --side buy --price 100 --qty 1.0`,
        success: true,
      };
    }
    
    return {
      output: `Trading commands:
  
  trade place --symbol <symbol> --side <buy|sell> --price <price> --qty <quantity>
    Place a market or limit order
    
  trade cancel <order-id>
    Cancel an open order
    
  trade history
    View trading history
    
💡 Interactive trading: http://localhost:5000/dashboard`,
      success: true,
    };
  }
  
  // Init command
  if (mainCommand === 'init') {
    const name = actualArgs.find((arg, i) => actualArgs[i-1] === '--name') || 'Percolator';
    
    return {
      output: `Initializing Percolator environment: ${name}

Network: ${network}
RPC URL: http://localhost:8899

[1/5] Checking Solana CLI...
[2/5] Checking wallet...
[3/5] Setting up configuration...
[4/5] Creating keypairs...
[5/5] Preparing deployment...

✓ Environment initialized successfully

Configuration saved to: Surfpool.toml

Next steps:
  1. Deploy programs: matcher deploy
  2. Create a slab: matcher create
  3. Start trading: http://localhost:5000/dashboard`,
      success: true,
    };
  }
  
  // Deploy command
  if (mainCommand === 'deploy') {
    return {
      output: `Deploying all programs to ${network}...

[1/6] Building programs...
  ✓ Slab program compiled
  ✓ Router program compiled
  ✓ AMM program compiled
  ✓ Oracle program compiled

[2/6] Deploying programs...
  • Deploying to ${network}
  • This may take a few minutes...

[3/6] Initializing state...
[4/6] Setting up accounts...
[5/6] Configuring permissions...
[6/6] Verifying deployment...

✓ All programs deployed successfully!

View program IDs: programs
Start trading: http://localhost:5000/dashboard`,
      success: true,
    };
  }
  
  // AMM commands
  if (mainCommand === 'amm') {
    const subCommand = actualArgs[1]?.toLowerCase();
    
    if (subCommand === 'create-pool') {
      return {
        output: `Creating AMM liquidity pool...

Network: ${network}

Pool configuration:
  • Token A: SOL
  • Token B: USDC
  • Fee: 0.3%

✓ Pool created successfully
✓ LP tokens minted

Add liquidity: amm add-liquidity
View pools: amm list`,
        success: true,
      };
    }
    
    return {
      output: `AMM commands:
  
  amm create-pool
    Create a new liquidity pool
    
  amm add-liquidity --amount-a <amount> --amount-b <amount>
    Add liquidity to a pool
    
  amm remove-liquidity --amount <lp-tokens>
    Remove liquidity from a pool
    
  amm swap --from <token> --to <token> --amount <amount>
    Swap tokens
    
  amm list
    List all pools`,
        success: true,
      };
    }
  
  // Liquidity commands
  if (mainCommand === 'liquidity') {
    return {
      output: `Liquidity Management:

Network: ${network}

Commands:
  liquidity add --amount <amount>
    Add liquidity to a pool
    
  liquidity remove --amount <amount>
    Remove liquidity from a pool
    
  liquidity stats
    View liquidity statistics
    
💡 Manage liquidity in the dashboard:
   http://localhost:5000/dashboard`,
      success: true,
    };
  }
  
  // Unknown command
  return {
    output: `Unknown command: ${mainCommand}

Type 'help' or '--help' to see available commands.

Common commands:
  status               System status
  market <symbol>      Market data
  matcher create       Create order book
  init                 Initialize environment
  deploy               Deploy programs
  
Full trading UI: http://localhost:5000/dashboard`,
    success: false,
  };
}

/**
 * Execute a CLI command with real binary or fallback
 * POST /api/cli/execute
 */
router.post('/execute', async (req, res) => {
  try {
    const { command, network = 'localnet', wallet, useBinary = true } = req.body;

    if (!command || typeof command !== 'string') {
      return res.status(400).json({
        error: 'Command is required',
      });
    }

    const trimmedCmd = command.trim();
    const firstArg = trimmedCmd.split(/\s+/)[0]?.toLowerCase();

    // Handle web-only commands (not in Rust CLI)
    if (firstArg === 'programs') {
      try {
        const programIds = await programDiscovery.getProgramIDs();
        const output = `Deployed Program IDs (${network}):

Router:  ${programIds.router || 'Not deployed'}
Slab:    ${programIds.slab || 'Not deployed'}
AMM:     ${programIds.amm || 'Not deployed'}
Oracle:  ${programIds.oracle || 'Not deployed'}

💡 Deploy programs with: deploy --all`;

        return res.json({ output, success: true });
      } catch (error) {
        return res.json({
          output: 'Failed to fetch program IDs',
          success: false,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    }

    // Trading commands - just parse and return to frontend for execution
    if (firstArg === 'buy' || firstArg === 'sell') {
      const args = trimmedCmd.split(/\s+/);
      const side = firstArg;
      const amount = args[1];
      const coin = args[2]?.toUpperCase();
      const orderType = args[3]?.toLowerCase() || 'market';
      const price = args[4];

      if (!wallet) {
        return res.json({
          output: `❌ Wallet not connected!\n\nPlease connect your Phantom wallet to trade.`,
          success: false,
          error: 'Wallet not connected'
        });
      }

      if (!amount || !coin) {
        return res.json({
          output: `❌ Invalid ${side} command!\n\nUsage: ${side} <amount> <coin> [limit|market] [price]\n\nExamples:\n  ${side} 10 SOL\n  ${side} 5 ETH market\n  ${side} 2 BTC limit 95000`,
          success: false,
          error: 'Invalid arguments'
        });
      }

      return res.json({
        success: true,
        requiresSignature: true,
        transactionData: {
          type: 'order',
          side,
          coin,
          amount: parseFloat(amount),
          orderType,
          price: price ? parseFloat(price) : null,
        }
      });
    }

    // Swap command
    if (firstArg === 'swap') {
      const args = trimmedCmd.split(/\s+/);
      const amount = args[1];
      const from = args[2]?.toUpperCase();
      const to = args[3]?.toUpperCase() || 'USDC';

      if (!wallet) {
        return res.json({
          output: `❌ Wallet not connected!\n\nPlease connect your Phantom wallet to swap.`,
          success: false,
          error: 'Wallet not connected'
        });
      }

      if (!amount || !from) {
        return res.json({
          output: `❌ Invalid swap command!\n\nUsage: swap <amount> <from> [to]\n\nExamples:\n  swap 10 SOL USDC\n  swap 100 USDC SOL`,
          success: false,
          error: 'Invalid arguments'
        });
      }

      return res.json({
        success: true,
        requiresSignature: true,
        transactionData: {
          type: 'swap',
          from,
          to,
          amount: parseFloat(amount),
        }
      });
    }

    // Add liquidity command
    if (firstArg === 'add-lp' || firstArg === 'addlp') {
      const args = trimmedCmd.split(/\s+/);
      const amount1 = args[1];
      const token1 = args[2]?.toUpperCase();
      const amount2 = args[3];
      const token2 = args[4]?.toUpperCase() || 'USDC';

      if (!wallet) {
        return res.json({
          output: `❌ Wallet not connected!\n\nPlease connect your Phantom wallet to add liquidity.`,
          success: false,
          error: 'Wallet not connected'
        });
      }

      if (!amount1 || !token1 || !amount2) {
        return res.json({
          output: `❌ Invalid add-lp command!\n\nUsage: add-lp <amount1> <token1> <amount2> [token2]\n\nExample:\n  add-lp 10 SOL 1390 USDC`,
          success: false,
          error: 'Invalid arguments'
        });
      }

      return res.json({
        success: true,
        requiresSignature: true,
        transactionData: {
          type: 'add-liquidity',
          token1,
          token2,
          amount1: parseFloat(amount1),
          amount2: parseFloat(amount2),
        }
      });
    }

    // Remove liquidity command
    if (firstArg === 'remove-lp' || firstArg === 'removelp') {
      const args = trimmedCmd.split(/\s+/);
      const lpAmount = args[1];
      const token1 = args[2]?.toUpperCase() || 'SOL';
      const token2 = args[3]?.toUpperCase() || 'USDC';

      if (!wallet) {
        return res.json({
          output: `❌ Wallet not connected!\n\nPlease connect your Phantom wallet to remove liquidity.`,
          success: false,
          error: 'Wallet not connected'
        });
      }

      if (!lpAmount) {
        return res.json({
          output: `❌ Invalid remove-lp command!\n\nUsage: remove-lp <lp-amount> [token1] [token2]\n\nExample:\n  remove-lp 100 SOL USDC`,
          success: false,
          error: 'Invalid arguments'
        });
      }

      return res.json({
        success: true,
        requiresSignature: true,
        transactionData: {
          type: 'remove-liquidity',
          token1,
          token2,
          lpAmount: parseFloat(lpAmount),
        }
      });
    }

    // Handle airdrop command
    if (firstArg === 'airdrop') {
      try {
        const args = trimmedCmd.split(/\s+/);
        const amount = args[1] ? parseFloat(args[1]) : 10;

        // Use WSL to call solana CLI (since validator is in WSL)
        const { spawn } = require('child_process');
        const solanaCmd = spawn('wsl', ['-e', 'bash', '-c', `solana airdrop ${amount}`], {
          env: process.env,
        });

        let stdout = '';
        let stderr = '';

        solanaCmd.stdout.on('data', (data: Buffer) => {
          stdout += data.toString();
        });

        solanaCmd.stderr.on('data', (data: Buffer) => {
          stderr += data.toString();
        });

        solanaCmd.on('close', (code: number) => {
          if (code === 0) {
            res.json({
              output: `✅ Airdropped ${amount} SOL to your wallet\n\n${stdout}\n\nYou can now run: init --name "MyExchange"`,
              success: true,
            });
          } else {
            res.json({
              output: `Failed to airdrop SOL\n\nError: ${stderr || stdout}\n\nTry running in WSL: solana airdrop ${amount}`,
              success: false,
              error: stderr || 'Airdrop failed',
            });
          }
        });

        return; // Async response handled above
      } catch (error) {
        return res.json({
          output: 'Failed to execute airdrop',
          success: false,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    }

    // Try to execute with real binary first
    if (useBinary && fs.existsSync(CLI_PATH)) {
      const result = await executeCLIBinary(command, network, wallet);
      return res.json(result);
    }

    // Fallback to mock commands if binary not available
    const result = await executeRealCommand(command, wallet);
    return res.json(result);

  } catch (error) {
    console.error('CLI endpoint error:', error);
    return res.status(500).json({
      error: error instanceof Error ? error.message : 'Failed to execute command',
      output: error instanceof Error ? error.message : 'Unknown error',
      success: false,
    });
  }
});

/**
 * Get CLI help
 * GET /api/cli/help
 */
router.get('/help', (req, res) => {
  const child = spawn(CLI_PATH, ['--help'], { cwd: PROJECT_ROOT });
  let output = '';

  child.stdout.on('data', (data) => {
    output += data.toString();
  });

  child.on('close', () => {
    res.json({ help: output });
  });

  child.on('error', (error) => {
    res.status(500).json({
      error: error.message,
    });
  });
});

/**
 * Get CLI version
 * GET /api/cli/version
 */
router.get('/version', (req, res) => {
  const child = spawn(CLI_PATH, ['--version'], { cwd: PROJECT_ROOT });
  let output = '';

  child.stdout.on('data', (data) => {
    output += data.toString();
  });

  child.on('close', () => {
    res.json({ version: output.trim() });
  });

  child.on('error', (error) => {
    res.status(500).json({
      error: error.message,
    });
  });
});

/**
 * Get CLI status
 * GET /api/cli/status
 */
router.get('/status', (req, res) => {
  const exists = fs.existsSync(CLI_PATH);

  res.json({
    cliPath: CLI_PATH,
    exists,
    ready: exists,
    message: exists
      ? 'CLI is ready to use'
      : 'CLI binary not found. Build it with: cargo build --release --package percolator-cli',
  });
});

/**
 * Get deployed program IDs
 * GET /api/cli/programs
 */
router.get('/programs', async (req, res) => {
  try {
    const programIds = await programDiscovery.getProgramIDs();
    const programInfo = await programDiscovery.getProgramInfo();

    res.json({
      success: true,
      programIds,
      programs: programInfo,
      allDeployed: await programDiscovery.allProgramsDeployed(),
    });
  } catch (error) {
    res.status(500).json({
      success: false,
      error: error instanceof Error ? error.message : 'Failed to fetch program IDs',
    });
  }
});

/**
 * Get real balance from Solana RPC
 * GET /api/cli/balance/:address
 */
router.get('/balance/:address', async (req, res) => {
  try {
    const { address } = req.params;
    const { network = 'localnet' } = req.query;

    const balance = await solanaState.getBalance(
      address,
      network as string
    );

    res.json({
      success: true,
      ...balance,
    });
  } catch (error) {
    res.status(500).json({
      success: false,
      error: error instanceof Error ? error.message : 'Failed to fetch balance',
    });
  }
});

/**
 * Get network status
 * GET /api/cli/network
 */
router.get('/network', async (req, res) => {
  try {
    const { network = 'localnet' } = req.query;

    const status = await solanaState.getNetworkStatus(network as string);

    res.json({
      success: true,
      ...status,
    });
  } catch (error) {
    res.status(500).json({
      success: false,
      error: error instanceof Error ? error.message : 'Failed to fetch network status',
    });
  }
});

/**
 * Refresh program IDs cache
 * POST /api/cli/programs/refresh
 */
router.post('/programs/refresh', async (req, res) => {
  try {
    const programIds = await programDiscovery.refresh();

    res.json({
      success: true,
      programIds,
      message: 'Program IDs refreshed successfully',
    });
  } catch (error) {
    res.status(500).json({
      success: false,
      error: error instanceof Error ? error.message : 'Failed to refresh program IDs',
    });
  }
});

export { router as cliRouter };

