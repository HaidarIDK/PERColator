# Setup script for Percolator devnet deployment with vanity addresses
# FORK-SAFE: Only creates keypairs/ folder (gitignored) - safe to sync with upstream
# This script generates vanity keypairs and deploys all programs to devnet

param(
    [switch]$SkipKeypairs,
    [switch]$SkipBuild,
    [switch]$SkipDeploy,
    [switch]$SkipInit
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# FORK-SAFE: All outputs go to keypairs/ (gitignored) - no repo modifications
$KEYPAIRS_DIR = "keypairs"
$NETWORK = "devnet"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Percolator Devnet Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if Solana CLI is installed
if (-not (Get-Command solana -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: solana CLI not found. Please install Solana CLI first." -ForegroundColor Red
    Write-Host "Visit: https://docs.solana.com/cli/install-solana-cli-tools" -ForegroundColor Yellow
    exit 1
}

# Check if solana-keygen grind is available (for vanity addresses)
$keygenVersion = solana-keygen --version 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: solana-keygen not found. Please install Solana CLI tools." -ForegroundColor Red
    exit 1
}

# Create keypairs directory
if (-not (Test-Path $KEYPAIRS_DIR)) {
    New-Item -ItemType Directory -Path $KEYPAIRS_DIR | Out-Null
    Write-Host "Created keypairs directory: $KEYPAIRS_DIR" -ForegroundColor Green
}

# Function to generate vanity keypair
function Generate-VanityKeypair {
    param(
        [string]$Name,
        [string]$Prefix,
        [int]$MaxAttempts = 1000000
    )
    
    Write-Host "Generating vanity keypair for $Name (prefix: $Prefix)..." -ForegroundColor Yellow
    Write-Host "  This may take a few minutes..." -ForegroundColor Gray
    
    $keypairPath = Join-Path $KEYPAIRS_DIR "$Name.json"
    
    # Convert prefix to lowercase for Solana (addresses are case-insensitive)
    $prefixLower = $Prefix.ToLower()
    
    # Generate vanity keypair with count (e.g., "d:1" means 1 character prefix)
    # solana-keygen grind creates a file with the pubkey as filename
    Write-Host "  Generating (this may take ~10 seconds)..." -ForegroundColor Gray
    $currentDir = Get-Location
    $grindOutput = solana-keygen grind --starts-with "${prefixLower}:1" 2>&1
    
    if ($LASTEXITCODE -eq 0) {
        # Find the generated keypair file (starts with the prefix)
        $generatedFile = Get-ChildItem -Path $currentDir -Filter "${prefixLower}*.json" | Select-Object -First 1
        if ($generatedFile -and $generatedFile.Exists) {
            # Move to our keypairs directory
            Move-Item -Path $generatedFile.FullName -Destination $keypairPath -Force
            $pubkey = (solana-keygen pubkey $keypairPath).Trim()
            Write-Host "  ✓ Generated: $pubkey" -ForegroundColor Green
            Write-Host "  ✓ Saved to: $keypairPath" -ForegroundColor Green
            return $pubkey
        }
    }
    
    # Fallback: Create regular keypair if vanity generation fails
    Write-Host "  ⚠ Vanity generation failed, creating regular keypair..." -ForegroundColor Yellow
    solana-keygen new --no-bip39-passphrase --outfile $keypairPath --force
    $pubkey = (solana-keygen pubkey $keypairPath).Trim()
    Write-Host "  ✓ Generated: $pubkey" -ForegroundColor Green
    return $pubkey
}

# Generate keypairs
if (-not $SkipKeypairs) {
    Write-Host "=== Generating Vanity Keypairs ===" -ForegroundColor Cyan
    Write-Host ""
    
    # Set Solana config to devnet
    solana config set --url devnet | Out-Null
    
    # Generate keypairs
    # Trading pairs: SOL/USDC, ETH/USDC, BTC/USDC (all use USD as quote)
    # Note: Slabs are accounts under the slab program, not separate programs
    # We'll create slab accounts during init (one per trading pair)
    # Using single-char prefixes for fast generation (~10 seconds each)
    $devWalletPubkey = Generate-VanityKeypair -Name "devwallet" -Prefix "d"
    $routerPubkey = Generate-VanityKeypair -Name "router" -Prefix "r"
    $slabProgramPubkey = Generate-VanityKeypair -Name "slab" -Prefix "s"
    $ammPubkey = Generate-VanityKeypair -Name "amm" -Prefix "a"
    $oraclePubkey = Generate-VanityKeypair -Name "oracle" -Prefix "o"
    
    Write-Host ""
    Write-Host "=== Keypair Summary ===" -ForegroundColor Cyan
    Write-Host "Dev Wallet:  $devWalletPubkey" -ForegroundColor White
    Write-Host "Router:      $routerPubkey" -ForegroundColor White
    Write-Host "Slab Program: $slabProgramPubkey (used for all trading pairs)" -ForegroundColor White
    Write-Host "AMM:        $ammPubkey" -ForegroundColor White
    Write-Host "Oracle:     $oraclePubkey" -ForegroundColor White
    Write-Host ""
    Write-Host "Trading pairs (will be created during init):" -ForegroundColor Yellow
    Write-Host "  - SOL/USDC (SOL-USD)" -ForegroundColor White
    Write-Host "  - ETH/USDC (ETH-USD)" -ForegroundColor White
    Write-Host "  - BTC/USDC (BTC-USD)" -ForegroundColor White
    Write-Host ""
    
    # Save pubkeys to a config file (fork-safe: only in keypairs/)
    $configPath = Join-Path $KEYPAIRS_DIR "config.json"
    if (Test-Path $configPath) {
        Write-Host "  ⚠ Config file already exists, updating..." -ForegroundColor Yellow
    }
    
    # Build JSON config
    $configJson = @{
        network = $NETWORK
        created_at = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ')
        note = "FORK-SAFE: This config is gitignored and wont conflict with upstream"
        keypairs = @{
            devwallet = @{
                pubkey = $devWalletPubkey
                path = "$KEYPAIRS_DIR/devwallet.json"
            }
            router = @{
                pubkey = $routerPubkey
                path = "$KEYPAIRS_DIR/router.json"
            }
            slab = @{
                pubkey = $slabProgramPubkey
                path = "$KEYPAIRS_DIR/slab.json"
                note = "Slab program - individual trading pair slabs will be created as accounts"
            }
            trading_pairs = @{
                "SOL-USD" = @{
                    symbol = "SOL-USD"
                    description = "SOL/USDC perpetual order book"
                }
                "ETH-USD" = @{
                    symbol = "ETH-USD"
                    description = "ETH/USDC perpetual order book"
                }
                "BTC-USD" = @{
                    symbol = "BTC-USD"
                    description = "BTC/USDC perpetual order book"
                }
            }
            amm = @{
                pubkey = $ammPubkey
                path = "$KEYPAIRS_DIR/amm.json"
            }
            oracle = @{
                pubkey = $oraclePubkey
                path = "$KEYPAIRS_DIR/oracle.json"
            }
        }
    }
    
    $configJson | ConvertTo-Json -Depth 10 | Out-File -FilePath (Join-Path $KEYPAIRS_DIR "config.json") -Encoding UTF8
    Write-Host "Config saved to: keypairs/config.json" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "Skipping keypair generation (SkipKeypairs specified)" -ForegroundColor Yellow
    Write-Host ""
}

# Build programs
if (-not $SkipBuild) {
    Write-Host "=== Building Programs ===" -ForegroundColor Cyan
    Write-Host ""
    
    cargo build-sbf
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Build failed!" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✓ Build successful!" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "Skipping build (SkipBuild specified)" -ForegroundColor Yellow
    Write-Host ""
}

# Check if we have the dev wallet keypair
$devWalletPath = Join-Path $KEYPAIRS_DIR "devwallet.json"
if (-not (Test-Path $devWalletPath)) {
    Write-Host "ERROR: Dev wallet keypair not found: $devWalletPath" -ForegroundColor Red
    Write-Host "Run without SkipKeypairs to generate it first." -ForegroundColor Yellow
    exit 1
}

# Airdrop SOL to dev wallet
Write-Host "=== Requesting SOL Airdrop ===" -ForegroundColor Cyan
$devWalletPubkey = (solana-keygen pubkey $devWalletPath).Trim()
Write-Host "Dev Wallet: $devWalletPubkey" -ForegroundColor White

Write-Host "Requesting 2 SOL airdrop..." -ForegroundColor Yellow
solana airdrop 2 $devWalletPubkey --url devnet | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Airdrop successful!" -ForegroundColor Green
} else {
    Write-Host "⚠ Airdrop may have failed (rate limit?) - continuing anyway" -ForegroundColor Yellow
}

# Wait a bit for confirmation
Start-Sleep -Seconds 2

# Check balance
$balance = (solana balance $devWalletPubkey --url devnet | Out-String).Trim()
Write-Host "Balance: $balance" -ForegroundColor White
Write-Host ""

# Deploy programs
if (-not $SkipDeploy) {
    Write-Host "=== Deploying Programs to Devnet ===" -ForegroundColor Cyan
    Write-Host ""
    
    # Check if programs are built
    $routerSo = "target\deploy\percolator_router.so"
    $slabSo = "target\deploy\percolator_slab.so"
    $ammSo = "target\deploy\percolator_amm.so"
    $oracleSo = "target\deploy\percolator_oracle.so"
    
    if (-not (Test-Path $routerSo) -or -not (Test-Path $slabSo) -or -not (Test-Path $ammSo) -or -not (Test-Path $oracleSo)) {
        Write-Host "ERROR: Programs not built. Run cargo build-sbf first." -ForegroundColor Red
        exit 1
    }
    
    # Deploy Router
    Write-Host "Deploying Router..." -ForegroundColor Yellow
    $routerKeypair = Join-Path $KEYPAIRS_DIR "router.json"
    $routerPubkey = (solana-keygen pubkey $routerKeypair).Trim()
    
    # Check if router account exists, if not create it
    $routerAccountInfo = solana account $routerPubkey --url devnet 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Creating router program account..." -ForegroundColor Gray
        solana program show $routerPubkey --url devnet | Out-Null
        if ($LASTEXITCODE -ne 0) {
            # Create program account
            solana program write-buffer $routerSo --url devnet --keypair $devWalletPath | Out-Null
            $bufferOutput = solana program show --buffers --url devnet
            $bufferLine = $bufferOutput | Select-String "Buffer"
            if ($bufferLine) {
                $bufferParts = $bufferLine.ToString() -split " "
                $bufferPubkey = $bufferParts[2]
            } else {
                Write-Host "  ⚠ Could not find buffer, trying direct deploy..." -ForegroundColor Yellow
                $bufferPubkey = $null
            }
            if ($bufferPubkey) {
                solana program deploy --buffer $bufferPubkey --program-id $routerKeypair --keypair $devWalletPath --url devnet | Out-Null
            }
        }
    }
    
    solana program deploy $routerSo --program-id $routerKeypair --keypair $devWalletPath --url devnet --upgrade-authority $devWalletPath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Router deployed: $routerPubkey" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ Router deployment may have failed or program already exists" -ForegroundColor Yellow
    }
    Write-Host ""
    
    # Deploy Slab Program
    Write-Host "Deploying Slab Program..." -ForegroundColor Yellow
    $slabKeypair = Join-Path $KEYPAIRS_DIR "slab.json"
    $slabPubkey = (solana-keygen pubkey $slabKeypair).Trim()
    
    solana program deploy $slabSo --program-id $slabKeypair --keypair $devWalletPath --url devnet --upgrade-authority $devWalletPath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Slab program deployed: $slabPubkey" -ForegroundColor Green
        Write-Host "  Note: Individual trading pair slabs will be created as accounts during init" -ForegroundColor Gray
    } else {
        Write-Host "  ⚠ Slab deployment may have failed or program already exists" -ForegroundColor Yellow
    }
    Write-Host ""
    
    # Deploy AMM
    Write-Host "Deploying AMM..." -ForegroundColor Yellow
    $ammKeypair = Join-Path $KEYPAIRS_DIR "amm.json"
    $ammPubkey = (solana-keygen pubkey $ammKeypair).Trim()
    
    solana program deploy $ammSo --program-id $ammKeypair --keypair $devWalletPath --url devnet --upgrade-authority $devWalletPath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ AMM deployed: $ammPubkey" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ AMM deployment may have failed or program already exists" -ForegroundColor Yellow
    }
    Write-Host ""
    
    # Deploy Oracle
    Write-Host "Deploying Oracle..." -ForegroundColor Yellow
    $oracleKeypair = Join-Path $KEYPAIRS_DIR "oracle.json"
    $oraclePubkey = (solana-keygen pubkey $oracleKeypair).Trim()
    
    solana program deploy $oracleSo --program-id $oracleKeypair --keypair $devWalletPath --url devnet --upgrade-authority $devWalletPath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ Oracle deployed: $oraclePubkey" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ Oracle deployment may have failed or program already exists" -ForegroundColor Yellow
    }
    Write-Host ""
    
    Write-Host "✓ Deployment complete!" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "Skipping deployment (SkipDeploy specified)" -ForegroundColor Yellow
    Write-Host ""
}

# Initialize exchange
if (-not $SkipInit) {
    Write-Host "=== Initializing Exchange ===" -ForegroundColor Cyan
    Write-Host ""
    
    $cliPath = "target\release\percolator.exe"
    if (-not (Test-Path $cliPath)) {
        $cliPath = "target\release\percolator"
    }
    
    Write-Host "Initializing exchange..." -ForegroundColor Yellow
    & $cliPath -n devnet --keypair $devWalletPath init --name PercolatorDevnetDEX --insurance-fund 1000000000 --maintenance-margin 500 --initial-margin 1000
    
    Write-Host ""
    Write-Host "✓ Setup complete!" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "Skipping initialization (SkipInit specified)" -ForegroundColor Yellow
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "All done" -ForegroundColor Green  
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps" -ForegroundColor Yellow
Write-Host "Check keypairs folder for all generated keypairs" -ForegroundColor White

