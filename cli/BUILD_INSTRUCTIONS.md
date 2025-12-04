# Build Instructions for Interactive CLI

## OpenSSL Setup on Windows

The Solana CLI requires OpenSSL. On Windows, you need to install OpenSSL first.

### Option 1: Install OpenSSL via vcpkg (Recommended)

1. Install vcpkg if you don't have it:
   ```powershell
   git clone https://github.com/Microsoft/vcpkg.git
   cd vcpkg
   .\bootstrap-vcpkg.bat
   ```

2. Install OpenSSL:
   ```powershell
   .\vcpkg install openssl:x64-windows
   ```

3. Set environment variables:
   ```powershell
   $env:OPENSSL_DIR = "C:\path\to\vcpkg\installed\x64-windows"
   $env:OPENSSL_LIBS = "ssl;crypto"
   ```

### Option 2: Install OpenSSL manually

1. Download OpenSSL for Windows from: https://slproweb.com/products/Win32OpenSSL.html
2. Install it to `C:\Program Files\OpenSSL-Win64`
3. Add `C:\Program Files\OpenSSL-Win64\bin` to your PATH

### Option 3: Use WSL (Windows Subsystem for Linux)

If you have WSL installed, you can build and run the CLI in Linux:

```bash
# In WSL
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator
cargo build --release -p percolator-cli
./target/release/percolator -n devnet interactive
```

### Option 4: Use Pre-built Solana Tools

The Solana CLI tools include OpenSSL. If you have Solana CLI installed, you might be able to use its OpenSSL:

```powershell
# Check if Solana CLI is installed
solana --version

# The Solana installation might include OpenSSL
```

## Building the CLI

Once OpenSSL is set up:

```powershell
# Build in debug mode (faster)
cargo build -p percolator-cli

# Build in release mode (optimized)
cargo build --release -p percolator-cli

# Run the interactive CLI
cargo run --release -p percolator-cli -- -n devnet interactive
```

## Troubleshooting

### "OpenSSL libdir does not contain the required files"

This means OpenSSL is not properly installed or not found. Try:

1. Verify OpenSSL installation:
   ```powershell
   # Check if OpenSSL is in PATH
   openssl version
   ```

2. Set OPENSSL_DIR environment variable:
   ```powershell
   $env:OPENSSL_DIR = "C:\Program Files\OpenSSL-Win64"
   ```

3. Ensure OpenSSL libraries are in the lib directory:
   - `libssl.lib` or `libssl.a`
   - `libcrypto.lib` or `libcrypto.a`

### "Command 'perl' not found"

This happens when trying to use vendored OpenSSL. Either:
- Install Perl (for vendored OpenSSL)
- Use system OpenSSL (recommended)
- Use WSL/Linux

### Alternative: Check Code Without Building

If you just want to verify the code compiles (syntax check):

```powershell
# This will check syntax but may fail on linking
cargo check -p percolator-cli --message-format=short 2>&1 | Select-String "error.*interactive"
```

## Quick Test Without Full Build

You can verify the interactive CLI code is correct by checking for syntax errors:

```powershell
# Check if the file has any obvious issues
rustc --edition 2021 --crate-type lib cli/src/interactive.rs --extern anyhow --extern colored 2>&1
```

## Recommended Setup

For the easiest experience on Windows:

1. **Use WSL2** (if available) - Linux environment, no OpenSSL issues
2. **Install OpenSSL via vcpkg** - Most reliable for Windows native builds
3. **Use Dev Container** - VS Code Dev Container with pre-configured environment

## Next Steps

Once built successfully:

```powershell
# Run on devnet
cargo run --release -p percolator-cli -- -n devnet interactive

# Or run the binary directly
.\target\release\percolator.exe -n devnet interactive
```

## Need Help?

If you continue to have OpenSSL issues:
1. Check the Solana documentation for Windows setup
2. Consider using WSL for development
3. Check if your Solana CLI installation includes OpenSSL










