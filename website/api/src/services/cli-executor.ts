import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

export interface CLIExecutorOptions {
  workingDir?: string;
  keypairPath?: string;
  network?: string;
  rpcUrl?: string;
}

export interface CLIOutput {
  type: 'stdout' | 'stderr' | 'exit' | 'error';
  data: string;
  timestamp: number;
  exitCode?: number;
}

export class CLIExecutor {
  private cliPath: string;
  private workingDir: string;
  private defaultKeypairPath: string;
  private defaultNetwork: string;
  private defaultRpcUrl: string;

  constructor() {
    // Path to the Rust CLI binary
    this.workingDir = path.resolve(__dirname, '../../../../');
    this.cliPath = path.join(this.workingDir, 'target', 'debug', 'percolator');
    this.defaultKeypairPath = path.join(this.workingDir, 'keypairs', 'devwallet.json');
    this.defaultNetwork = 'devnet';
    this.defaultRpcUrl = 'https://api.devnet.solana.com';

    // Check if CLI binary exists
    if (!fs.existsSync(this.cliPath) && !fs.existsSync(this.cliPath + '.exe')) {
      console.warn(`CLI binary not found at ${this.cliPath}. Please build it first with: cargo build --bin percolator`);
    }
  }

  /**
   * Execute a CLI command and stream output
   */
  async execute(
    command: string,
    options: CLIExecutorOptions = {},
    onOutput: (output: CLIOutput) => void
  ): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Parse command
        const args = this.parseCommand(command, options);

        console.log(`[CLI] Executing: ${this.cliPath} ${args.join(' ')}`);

        // Spawn process
        const proc = spawn(this.cliPath, args, {
          cwd: options.workingDir || this.workingDir,
          env: {
            ...process.env,
            RUST_BACKTRACE: '1',
          },
        });

        // Handle stdout
        proc.stdout.on('data', (data: Buffer) => {
          const output = data.toString('utf8');
          console.log('[CLI stdout]', output);
          onOutput({
            type: 'stdout',
            data: output,
            timestamp: Date.now(),
          });
        });

        // Handle stderr
        proc.stderr.on('data', (data: Buffer) => {
          const output = data.toString('utf8');
          console.error('[CLI stderr]', output);
          onOutput({
            type: 'stderr',
            data: output,
            timestamp: Date.now(),
          });
        });

        // Handle exit
        proc.on('exit', (code: number | null) => {
          console.log(`[CLI] Process exited with code ${code}`);
          onOutput({
            type: 'exit',
            data: code === 0 ? 'Command completed successfully' : `Command failed with exit code ${code}`,
            timestamp: Date.now(),
            exitCode: code || 0,
          });
          resolve();
        });

        // Handle errors
        proc.on('error', (err: Error) => {
          console.error('[CLI] Process error:', err);
          onOutput({
            type: 'error',
            data: `Error: ${err.message}`,
            timestamp: Date.now(),
          });
          reject(err);
        });

      } catch (error: any) {
        onOutput({
          type: 'error',
          data: `Failed to execute command: ${error.message}`,
          timestamp: Date.now(),
        });
        reject(error);
      }
    });
  }

  /**
   * Parse user command into CLI args
   */
  private parseCommand(command: string, options: CLIExecutorOptions): string[] {
    const args: string[] = [];

    // Add network
    if (options.network || this.defaultNetwork) {
      args.push('--network', options.network || this.defaultNetwork);
    }

    // Add RPC URL
    if (options.rpcUrl || this.defaultRpcUrl) {
      args.push('--url', options.rpcUrl || this.defaultRpcUrl);
    }

    // Add keypair
    if (options.keypairPath || this.defaultKeypairPath) {
      args.push('--keypair', options.keypairPath || this.defaultKeypairPath);
    }

    // Parse the actual command
    const trimmed = command.trim();
    
    // Handle simple commands
    if (trimmed === 'help' || trimmed === '--help' || trimmed === '-h') {
      args.push('--help');
      return args;
    }

    if (trimmed === 'version' || trimmed === '--version') {
      args.push('--version');
      return args;
    }

    // Split command into parts
    const parts = this.splitCommandLine(trimmed);
    args.push(...parts);

    return args;
  }

  /**
   * Split command line respecting quotes
   */
  private splitCommandLine(command: string): string[] {
    const args: string[] = [];
    let current = '';
    let inQuotes = false;
    let quoteChar = '';

    for (let i = 0; i < command.length; i++) {
      const char = command[i];

      if ((char === '"' || char === "'") && !inQuotes) {
        inQuotes = true;
        quoteChar = char;
      } else if (char === quoteChar && inQuotes) {
        inQuotes = false;
        quoteChar = '';
      } else if (char === ' ' && !inQuotes) {
        if (current) {
          args.push(current);
          current = '';
        }
      } else {
        current += char;
      }
    }

    if (current) {
      args.push(current);
    }

    return args;
  }

  /**
   * Validate command for security
   */
  isCommandSafe(command: string): { safe: boolean; reason?: string } {
    // Disallow shell operators
    const dangerousPatterns = [
      '&&', '||', ';', '|', '>', '<', '`', '$(',
      'rm ', 'sudo ', 'chmod ', 'chown ',
    ];

    for (const pattern of dangerousPatterns) {
      if (command.includes(pattern)) {
        return {
          safe: false,
          reason: `Command contains potentially dangerous pattern: ${pattern}`,
        };
      }
    }

    return { safe: true };
  }

  /**
   * Get available commands (for autocomplete)
   */
  getAvailableCommands(): string[] {
    return [
      'deploy --all',
      'deploy --router',
      'deploy --slab',
      'deploy --amm',
      'deploy --oracle',
      'init --name my-exchange',
      'matcher create',
      'matcher list',
      'matcher info',
      'liquidity add',
      'liquidity remove',
      'liquidity list',
      'amm create',
      'amm deposit',
      'amm withdraw',
      'trade market',
      'trade limit',
      'trade cancel',
      'trade list',
      'margin deposit',
      'margin withdraw',
      'margin balance',
      'liquidation trigger',
      'liquidation list',
      'insurance deposit',
      'insurance withdraw',
      'insurance status',
      'keeper start',
      'keeper stop',
      'test --all',
      'test --quick',
      'status',
      'help',
      'version',
    ];
  }

  /**
   * Build CLI binary if not exists
   */
  async buildIfNeeded(): Promise<boolean> {
    if (fs.existsSync(this.cliPath) || fs.existsSync(this.cliPath + '.exe')) {
      return true;
    }

    console.log('[CLI] Binary not found, attempting to build...');
    
    return new Promise((resolve) => {
      const build = spawn('cargo', ['build', '--bin', 'percolator'], {
        cwd: this.workingDir,
        shell: true,
      });

      build.on('exit', (code) => {
        if (code === 0) {
          console.log('[CLI] Build successful');
          resolve(true);
        } else {
          console.error('[CLI] Build failed with code', code);
          resolve(false);
        }
      });

      build.on('error', (err) => {
        console.error('[CLI] Build error:', err);
        resolve(false);
      });
    });
  }
}

export const cliExecutor = new CLIExecutor();

