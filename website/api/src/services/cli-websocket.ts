import { WebSocket } from 'ws';
import { cliExecutor, CLIOutput } from './cli-executor';

export interface CLIMessage {
  type: 'command' | 'ping';
  command?: string;
  options?: {
    keypair?: string;
    network?: string;
    rpcUrl?: string;
  };
}

export interface CLIResponse {
  type: 'output' | 'error' | 'complete' | 'pong';
  data?: string;
  timestamp: number;
  exitCode?: number;
}

export class CLIWebSocketHandler {
  private ws: WebSocket;
  private isExecuting: boolean = false;

  constructor(ws: WebSocket) {
    this.ws = ws;
    this.setupHandlers();
  }

  private setupHandlers() {
    this.ws.on('message', async (data: Buffer) => {
      try {
        const message: CLIMessage = JSON.parse(data.toString());
        await this.handleMessage(message);
      } catch (error: any) {
        this.sendError(`Failed to parse message: ${error.message}`);
      }
    });

    this.ws.on('close', () => {
      console.log('[CLI WS] Client disconnected');
    });

    this.ws.on('error', (error) => {
      console.error('[CLI WS] WebSocket error:', error);
    });

    // Send welcome message
    this.send({
      type: 'output',
      data: 'Connected to Percolator CLI backend.\nType a command or "help" to get started.',
      timestamp: Date.now(),
    });
  }

  private async handleMessage(message: CLIMessage) {
    if (message.type === 'ping') {
      this.send({
        type: 'pong',
        timestamp: Date.now(),
      });
      return;
    }

    if (message.type === 'command') {
      await this.executeCommand(message.command || '', message.options);
    }
  }

  private async executeCommand(command: string, options: any = {}) {
    // Prevent concurrent execution
    if (this.isExecuting) {
      this.sendError('A command is already executing. Please wait for it to complete.');
      return;
    }

    // Validate command
    const safetyCheck = cliExecutor.isCommandSafe(command);
    if (!safetyCheck.safe) {
      this.sendError(`Command rejected: ${safetyCheck.reason}`);
      return;
    }

    this.isExecuting = true;

    try {
      // Build CLI if needed
      const ready = await cliExecutor.buildIfNeeded();
      if (!ready) {
        this.sendError('CLI binary not available. Please build it with: cargo build --bin percolator');
        this.isExecuting = false;
        return;
      }

      // Execute command
      await cliExecutor.execute(
        command,
        options,
        (output: CLIOutput) => {
          this.handleCLIOutput(output);
        }
      );

    } catch (error: any) {
      this.sendError(`Command execution failed: ${error.message}`);
    } finally {
      this.isExecuting = false;
    }
  }

  private handleCLIOutput(output: CLIOutput) {
    if (output.type === 'exit') {
      this.send({
        type: 'complete',
        data: output.data,
        timestamp: output.timestamp,
        exitCode: output.exitCode,
      });
    } else if (output.type === 'error') {
      this.sendError(output.data);
    } else {
      // stdout or stderr
      this.send({
        type: 'output',
        data: output.data,
        timestamp: output.timestamp,
      });
    }
  }

  private send(response: CLIResponse) {
    if (this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(response));
    }
  }

  private sendError(message: string) {
    this.send({
      type: 'error',
      data: message,
      timestamp: Date.now(),
    });
  }
}

export function handleCLIWebSocket(ws: WebSocket) {
  new CLIWebSocketHandler(ws);
}

