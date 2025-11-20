import WebSocket from 'ws';
import { spawn, ChildProcess } from 'child_process';
import path from 'path';
import fs from 'fs';

// Detect CLI path
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
      return cliPath;
    }
  }

  return path.join(PROJECT_ROOT, 'target/release/percolator.exe');
}

const CLI_PATH = findCLIPath();
const IS_WSL_BINARY = CLI_PATH.includes('percolator') && !CLI_PATH.endsWith('.exe') && process.platform === 'win32';

interface CLISession {
  process?: ChildProcess;
  command: string;
  network: string;
  startTime: number;
}

/**
 * CLI WebSocket Handler
 * Manages real-time streaming of CLI command output
 */
export class CLIWebSocketHandler {
  private sessions: Map<WebSocket, CLISession> = new Map();

  /**
   * Handle incoming messages from CLI WebSocket clients
   */
  handleMessage(ws: WebSocket, data: Buffer): void {
    try {
      const message = JSON.parse(data.toString());

      switch (message.type) {
        case 'execute':
          this.executeCommand(ws, message.command, message.network || 'localnet');
          break;
        case 'cancel':
          this.cancelCommand(ws);
          break;
        case 'ping':
          this.sendMessage(ws, { type: 'pong', timestamp: Date.now() });
          break;
        default:
          this.sendMessage(ws, {
            type: 'error',
            error: `Unknown message type: ${message.type}`,
          });
      }
    } catch (error) {
      this.sendMessage(ws, {
        type: 'error',
        error: error instanceof Error ? error.message : 'Invalid message format',
      });
    }
  }

  /**
   * Execute a CLI command with real-time streaming
   */
  private executeCommand(ws: WebSocket, command: string, network: string): void {
    // Check if there's already a running command
    const existingSession = this.sessions.get(ws);
    if (existingSession?.process) {
      this.sendMessage(ws, {
        type: 'error',
        error: 'A command is already running. Cancel it first.',
      });
      return;
    }

    // Check if CLI binary exists
    if (!fs.existsSync(CLI_PATH)) {
      this.sendMessage(ws, {
        type: 'error',
        error: `CLI binary not found at: ${CLI_PATH}`,
      });
      this.sendMessage(ws, {
        type: 'output',
        data: 'Please build the CLI first:\n  cd cli\n  cargo build --release',
      });
      this.sendMessage(ws, {
        type: 'complete',
        success: false,
        exitCode: 1,
      });
      return;
    }

    // Parse command into args
    const args = command.trim().split(/\s+/);

    // Add network flag if not already present
    const hasNetworkFlag = args.includes('-n') || args.includes('--network');
    const cliArgs = hasNetworkFlag ? args : ['-n', network, ...args];

    // Don't add --json for streaming (we want live output)
    // --json is for /api/cli/execute endpoint only

    // Spawn the CLI process (with WSL wrapper if needed)
    let child: ChildProcess;

    if (IS_WSL_BINARY) {
      // Convert Windows path to WSL path
      const wslPath = CLI_PATH.replace(/^([A-Z]):/, (_, letter) => `/mnt/${letter.toLowerCase()}`).replace(/\\/g, '/');
      const wslArgs = ['-e', wslPath, ...cliArgs];

      child = spawn('wsl', wslArgs, {
        cwd: PROJECT_ROOT,
        env: {
          ...process.env,
          RUST_LOG: 'info',
        },
      });
    } else {
      child = spawn(CLI_PATH, cliArgs, {
        cwd: PROJECT_ROOT,
        env: {
          ...process.env,
          RUST_LOG: 'info',
        },
      });
    }

    // Store session
    const session: CLISession = {
      process: child,
      command,
      network,
      startTime: Date.now(),
    };
    this.sessions.set(ws, session);

    // Send start message
    this.sendMessage(ws, {
      type: 'start',
      command,
      network,
      timestamp: session.startTime,
    });

    // Stream stdout
    child.stdout.on('data', (data) => {
      this.sendMessage(ws, {
        type: 'output',
        stream: 'stdout',
        data: data.toString(),
      });
    });

    // Stream stderr
    child.stderr.on('data', (data) => {
      this.sendMessage(ws, {
        type: 'output',
        stream: 'stderr',
        data: data.toString(),
      });
    });

    // Handle completion
    child.on('close', (code) => {
      const duration = Date.now() - session.startTime;

      this.sendMessage(ws, {
        type: 'complete',
        success: code === 0,
        exitCode: code,
        duration,
      });

      // Clean up session
      const currentSession = this.sessions.get(ws);
      if (currentSession) {
        delete currentSession.process;
      }
    });

    // Handle errors
    child.on('error', (error) => {
      this.sendMessage(ws, {
        type: 'error',
        error: error.message,
      });

      this.sendMessage(ws, {
        type: 'complete',
        success: false,
        exitCode: 1,
        duration: Date.now() - session.startTime,
      });

      // Clean up session
      const currentSession = this.sessions.get(ws);
      if (currentSession) {
        delete currentSession.process;
      }
    });
  }

  /**
   * Cancel a running command
   */
  private cancelCommand(ws: WebSocket): void {
    const session = this.sessions.get(ws);

    if (!session?.process) {
      this.sendMessage(ws, {
        type: 'error',
        error: 'No running command to cancel',
      });
      return;
    }

    // Kill the process
    session.process.kill('SIGTERM');

    this.sendMessage(ws, {
      type: 'cancelled',
      message: 'Command cancelled',
    });
  }

  /**
   * Handle client disconnect
   */
  handleDisconnect(ws: WebSocket): void {
    const session = this.sessions.get(ws);

    // Kill any running process
    if (session?.process) {
      session.process.kill('SIGTERM');
    }

    // Clean up session
    this.sessions.delete(ws);
  }

  /**
   * Send a message to the client
   */
  private sendMessage(ws: WebSocket, message: any): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(message));
    }
  }

  /**
   * Get active session count
   */
  getActiveSessionCount(): number {
    return this.sessions.size;
  }

  /**
   * Get running process count
   */
  getRunningProcessCount(): number {
    let count = 0;
    for (const session of this.sessions.values()) {
      if (session.process) {
        count++;
      }
    }
    return count;
  }
}

// Singleton instance
export const cliWebSocketHandler = new CLIWebSocketHandler();
