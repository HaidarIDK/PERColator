# Run Interactive CLI in WSL (Easiest Method)

Since you have WSL installed, this is the **easiest way** to run the CLI - no OpenSSL setup needed!

## Quick Start

### Step 1: Open WSL Terminal

```powershell
# In PowerShell, just type:
wsl
```

### Step 2: Navigate to Project

```bash
# In WSL terminal
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator
```

### Step 3: Build the CLI

```bash
# Build in release mode
cargo build --release -p percolator-cli

# This will compile everything (first time takes a few minutes)
```

### Step 4: Get Devnet SOL

```bash
# Make sure Solana CLI is available in WSL
# If not installed in WSL, you can use the Windows one:
solana config set --url devnet
solana airdrop 2
```

### Step 5: Run the Interactive CLI

```bash
# Run the interactive CLI
./target/release/percolator -n devnet interactive

# Or use cargo run
cargo run --release -p percolator-cli -- -n devnet interactive
```

## That's It! 🎉

The CLI will start and show you the interactive menu. OpenSSL works automatically in WSL.

## Alternative: One-Line Command

If you're in PowerShell and want to quickly run in WSL:

```powershell
wsl bash -c "cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator && cargo run --release -p percolator-cli -- -n devnet interactive"
```

## Troubleshooting

### "cargo: command not found" in WSL

Install Rust in WSL:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### "solana: command not found" in WSL

Either:
1. Install Solana CLI in WSL (recommended for WSL development)
2. Or use Windows Solana CLI from WSL (more complex)

To install in WSL:
```bash
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### Build Errors

If you get build errors, make sure you're in the right directory:

```bash
pwd
# Should show: /mnt/c/Users/7haid/OneDrive/Desktop/percolator
```

## Benefits of WSL

✅ No OpenSSL setup needed  
✅ Linux-native build environment  
✅ Faster builds (usually)  
✅ Better terminal experience  
✅ No Windows path issues  

## Next Steps

Once the CLI is running:

1. **Initialize Portfolio** (Menu 1 → Option 2)
2. **Create Slab** (Menu 2 → Option 1)
3. **Deposit Collateral** (Menu 4 → Option 2)
4. **Place Orders** (Menu 3 → Option 1)
5. **View Status** (Menu 7 → Option 2)

Enjoy testing! 🚀










