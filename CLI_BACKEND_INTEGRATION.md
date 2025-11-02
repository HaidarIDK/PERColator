# CLI Backend Integration Guide

## Overview

The frontend CLI is now connected to **Toly's Rust CLI backend** via WebSocket! Users can execute real commands from the web interface.

---

## Architecture

```
┌─────────────────────────────────┐
│   Frontend CLI (Next.js)        │
│   - WebSocket Client            │
│   - Command Input               │
│   - Real-time Output Display    │
└───────────┬─────────────────────┘
            │
            │ WebSocket (ws://localhost:5001/ws/cli)
            │
┌───────────▼─────────────────────┐
│   Backend API (Express)         │
│   - WebSocket Server            │
│   - Command Validation          │
│   - CLI Process Management      │
└───────────┬─────────────────────┘
            │
            │ spawn()
            │
┌───────────▼─────────────────────┐
│   Rust CLI (percolator binary)  │
│   - Deploy commands             │
│   - Trading operations          │
│   - Portfolio management        │
│   - Full Solana integration     │
└─────────────────────────────────┘
```

---

## Setup

### 1. Build the Rust CLI

First, build the CLI binary:

```bash
cargo build --bin percolator
```

This creates: `target/debug/percolator` (or `percolator.exe` on Windows)

### 2. Start the Backend API

```bash
cd website/api
npm run dev
```

The WebSocket server will start on: `ws://localhost:5001/ws/cli`

### 3. Start the Frontend

```bash
cd website/frontend
npm run dev
```

Go to: http://localhost:3000/cli

---

## How It Works

### Frontend → Backend

When a user types a command:

1. **Frontend** sends JSON via WebSocket:
   ```json
   {
     "type": "command",
     "command": "help",
     "options": {
       "network": "devnet"
     }
   }
   ```

2. **Backend** validates the command for security

3. **Backend** spawns the Rust CLI process:
   ```bash
   percolator --network devnet --keypair keypairs/devwallet.json help
   ```

4. **Backend** streams stdout/stderr back to frontend in real-time

### Backend → Frontend

The backend sends messages:

```json
{
  "type": "output",
  "data": "Available commands:\n...",
  "timestamp": 1699123456789
}
```

Message types:
- `output` - Command output (stdout)
- `error` - Error messages (stderr)
- `complete` - Command finished
- `pong` - Ping response

---

## Available CLI Commands

The Rust CLI supports:

### Deployment
```bash
deploy --all
deploy --router
deploy --slab
deploy --amm
deploy --oracle
```

### Exchange Setup
```bash
init --name my-exchange --insurance-fund 1000000000
```

### Matcher/Slab
```bash
matcher create --exchange <addr> --symbol BTC-USD --tick-size 100 --lot-size 1
matcher list --exchange <addr>
matcher info --matcher <addr>
```

### Liquidity
```bash
liquidity add --matcher <addr> --amount 1000000
liquidity remove --matcher <addr> --amount 500000
liquidity list --matcher <addr>
```

### AMM
```bash
amm create --token0 <addr> --token1 <addr>
amm deposit --pool <addr> --amount0 1000 --amount1 2000
amm withdraw --pool <addr> --shares 500
```

### Trading
```bash
trade market --symbol BTC-USD --side buy --quantity 0.1
trade limit --symbol BTC-USD --side sell --price 45000 --quantity 0.05
trade cancel --order <id>
trade list
```

### Margin
```bash
margin deposit --portfolio <addr> --amount 1000000000
margin withdraw --portfolio <addr> --amount 500000000
margin balance
```

### Liquidation
```bash
liquidation trigger --user <addr>
liquidation list
```

### Insurance
```bash
insurance deposit --amount 1000000000
insurance withdraw --amount 500000000
insurance status
```

### Keeper
```bash
keeper start
keeper stop
```

### Testing
```bash
test --all
test --quick
test --margin
test --matching
test --liquidations
```

### Status
```bash
status --exchange <addr> --detailed
```

---

## Security

### Command Validation

The backend validates all commands before execution:

```typescript
// Blocked patterns
const dangerousPatterns = [
  '&&', '||', ';', '|', '>', '<', '`', '$(',
  'rm ', 'sudo ', 'chmod ', 'chown ',
];
```

If a command contains dangerous patterns, it's rejected immediately.

### Process Isolation

Each command runs in an isolated process:
- Cannot access parent process
- Timeout protection
- Resource limits

---

## Files Created

### Backend

1. **`website/api/src/services/cli-executor.ts`**
   - Spawns Rust CLI process
   - Streams stdout/stderr
   - Command validation
   - Auto-build if binary missing

2. **`website/api/src/services/cli-websocket.ts`**
   - WebSocket message handler
   - Command routing
   - Output formatting

3. **`website/api/src/services/websocket-server.ts`** (modified)
   - Added `/ws/cli` endpoint
   - Separate WebSocket for CLI

### Frontend

4. **`website/frontend/src/app/cli/page.tsx`** (modified)
   - WebSocket client
   - Real-time output display
   - Connection status indicators
   - Fallback to simulated commands

---

## Connection Status

The frontend shows connection status:

### Connected (Green)
```
percolator-cli [LIVE]    ✓ Backend Connected
```

Real Rust CLI commands are executed.

### Disconnected (Yellow)
```
percolator-cli           ⚠ Simulated
```

Falls back to simulated/demo commands.

---

## Error Handling

### Backend Not Running

If the backend is not running:
```
WebSocket connection error. Make sure the backend is running.
```

### CLI Binary Missing

If `target/debug/percolator` doesn't exist:
```
CLI binary not available. Please build it with: cargo build --bin percolator
```

The backend will attempt to auto-build it.

### Command Fails

If a command returns non-zero exit code:
```
Command failed with exit code 1
[error output]
```

---

## Testing

### Test the Integration

1. **Start backend:**
   ```bash
   cd website/api
   npm run dev
   ```

2. **Start frontend:**
   ```bash
   cd website/frontend
   npm run dev
   ```

3. **Open CLI:** http://localhost:3000/cli

4. **Try commands:**
   ```bash
   help
   version
   deploy --help
   test --quick
   ```

### Check WebSocket Connection

Open browser console:
```
[CLI WS] Connected
```

Should appear when connected.

---

## Troubleshooting

### "Backend not connected"

**Check:**
1. Is the backend API running? (`cd website/api && npm run dev`)
2. Is it on port 5001? Check `website/api/src/index.ts`
3. Firewall blocking WebSocket?

### "CLI binary not available"

**Fix:**
```bash
cargo build --bin percolator
```

### Commands hang

**Check:**
- CLI might be waiting for input
- Network connection to Solana RPC
- Keypair file exists (`keypairs/devwallet.json`)

### "Command rejected"

The command contains dangerous patterns. Review security validation in `cli-executor.ts`.

---

## Configuration

### Change Network

Edit `website/frontend/src/app/cli/page.tsx`:

```typescript
wsRef.current.send(JSON.stringify({
  type: 'command',
  command: trimmedCmd,
  options: {
    network: 'mainnet-beta', // Change here
    rpcUrl: 'https://api.mainnet-beta.solana.com',
  }
}));
```

### Change Keypair

Edit `website/api/src/services/cli-executor.ts`:

```typescript
this.defaultKeypairPath = path.join(
  this.workingDir,
  'keypairs',
  'your-keypair.json' // Change here
);
```

### Change WebSocket Port

Edit `website/api/src/index.ts`:

```typescript
const PORT = process.env.PORT || 5002; // Change port
```

Then update frontend:

```typescript
const ws = new WebSocket('ws://localhost:5002/ws/cli');
```

---

## Advanced Usage

### Stream Long-Running Commands

The WebSocket streams output in real-time:

```bash
test --all
```

You'll see each test result as it completes.

### Command History

The CLI tracks command history:
- Press ↑/↓ to navigate
- Recent commands shown in context

### AI Assistant Integration

The AI can now reference CLI output:

**User:** "Why did my deploy fail?"

**AI sees:** Last command was `deploy --router` with error "Insufficient funds"

**AI responds:** "Your deployment failed due to insufficient funds. Make sure your wallet has enough SOL for deployment costs..."

---

## Next Steps

### Add More Commands

Edit the CLI's `main.rs` to add new commands:

```rust
#[derive(Subcommand)]
enum Commands {
    YourNewCommand {
        #[arg(short, long)]
        option: String,
    },
}
```

Rebuild and it's instantly available in the web CLI!

### Add Command Autocomplete

Implement tab completion in the frontend using the CLI's available commands.

### Add Syntax Highlighting

Colorize commands and output for better readability.

### Add Command Templates

Provide quick-fill templates for common operations.

---

## Summary

| Feature | Status |
|---------|--------|
| WebSocket Connection | ✅ Working |
| Command Execution | ✅ Working |
| Real-time Streaming | ✅ Working |
| Error Handling | ✅ Working |
| Security Validation | ✅ Working |
| Connection Status | ✅ Working |
| Fallback to Simulation | ✅ Working |
| AI Context Awareness | ✅ Working |

**The frontend CLI is now fully connected to Toly's Rust CLI!** 🚀

Users can execute real commands from the browser with live output streaming.

