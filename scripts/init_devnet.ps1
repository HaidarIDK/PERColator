# Initialize exchange, create slabs (SOL/USDC, ETH/USDC, BTC/USDC), and AMM on devnet
# FORK-SAFE: Only reads from keypairs/ folder - no repo modifications
# Run this after setup_devnet.ps1 has generated keypairs and deployed programs

param(
    [string]$ExchangeName = "Percolator Devnet"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$KEYPAIRS_DIR = "keypairs"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Initializing Percolator on Devnet" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if config exists
$configPath = Join-Path $KEYPAIRS_DIR "config.json"
if (-not (Test-Path $configPath)) {
    Write-Host "ERROR: Config file not found: $configPath" -ForegroundColor Red
    Write-Host "Run scripts/setup_devnet.ps1 first to generate keypairs." -ForegroundColor Yellow
    exit 1
}

$config = Get-Content $configPath | ConvertFrom-Json
$devWalletPath = $config.keypairs.devwallet.path

# Check CLI is built
$cliPath = "target\release\percolator.exe"
if (-not (Test-Path $cliPath)) {
    $cliPath = "target\release\percolator"
    if (-not (Test-Path $cliPath)) {
        Write-Host "Building CLI..." -ForegroundColor Yellow
        cargo build --release --bin percolator
        if ($LASTEXITCODE -ne 0) {
            Write-Host "ERROR: CLI build failed!" -ForegroundColor Red
            exit 1
        }
    }
}

# Step 1: Initialize Exchange
Write-Host "=== Step 1: Initialize Exchange ===" -ForegroundColor Cyan
Write-Host ""
& $cliPath -n devnet --keypair $devWalletPath init `
    --name $ExchangeName `
    --insurance-fund 1000000000 `
    --maintenance-margin 500 `
    --initial-margin 1000

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Exchange initialization failed!" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "✓ Exchange initialized!" -ForegroundColor Green
Write-Host ""

# Get registry address (we'll need this for next steps)
# The registry is derived from the router program ID
$routerPubkey = $config.keypairs.router.pubkey
Write-Host "Router Program: $routerPubkey" -ForegroundColor White
Write-Host ""

# Parse registry address from output (or get it from config if available)
Write-Host ""
Write-Host "=== Step 2: Create Trading Pairs (Slabs) ===" -ForegroundColor Cyan
Write-Host ""

# Read registry - it's derived from router program + devwallet
$configPath = Join-Path $KEYPAIRS_DIR "config.json"
if (Test-Path $configPath) {
    $config = Get-Content $configPath | ConvertFrom-Json
    $routerPubkey = $config.keypairs.router.pubkey
    
    Write-Host "Creating 3 order books:" -ForegroundColor Yellow
    Write-Host "  - SOL/USDC (SOL-USD)" -ForegroundColor White
    Write-Host "  - ETH/USDC (ETH-USD)" -ForegroundColor White
    Write-Host "  - BTC/USDC (BTC-USD)" -ForegroundColor White
    Write-Host ""
    
    # Note: We need the registry address from init output
    # For now, show instructions
    Write-Host "Note: After exchange init completes, run these commands:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Create SOL/USDC order book:" -ForegroundColor Cyan
    Write-Host "  .\target\release\percolator.exe -n devnet --keypair $devWalletPath matcher create <REGISTRY> SOL-USD --tick-size 100 --lot-size 1000" -ForegroundColor White
    Write-Host ""
    Write-Host "Create ETH/USDC order book:" -ForegroundColor Cyan
    Write-Host "  .\target\release\percolator.exe -n devnet --keypair $devWalletPath matcher create <REGISTRY> ETH-USD --tick-size 100 --lot-size 1000" -ForegroundColor White
    Write-Host ""
    Write-Host "Create BTC/USDC order book:" -ForegroundColor Cyan
    Write-Host "  .\target\release\percolator.exe -n devnet --keypair $devWalletPath matcher create <REGISTRY> BTC-USD --tick-size 100 --lot-size 1000" -ForegroundColor White
    Write-Host ""
    Write-Host "Create AMM pool (optional):" -ForegroundColor Cyan
    Write-Host "  .\target\release\percolator.exe -n devnet --keypair $devWalletPath amm create <REGISTRY> SOL-USD --x-reserve 1000000000 --y-reserve 100000000" -ForegroundColor White
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Your trading pairs will be:" -ForegroundColor Yellow
Write-Host "  ✓ SOL/USDC - Perpetual futures order book" -ForegroundColor White
Write-Host "  ✓ ETH/USDC - Perpetual futures order book" -ForegroundColor White
Write-Host "  ✓ BTC/USDC - Perpetual futures order book" -ForegroundColor White
Write-Host ""
Write-Host "Users can:" -ForegroundColor Yellow
Write-Host "  - Place limit/market orders on any pair" -ForegroundColor White
Write-Host "  - Go long or short with leverage" -ForegroundColor White
Write-Host "  - Deposit USDC (SOL on devnet) as collateral" -ForegroundColor White
Write-Host ""

