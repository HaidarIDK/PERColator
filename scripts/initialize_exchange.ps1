# Initialize Percolator Exchange on Devnet
# Creates the router registry and 3 trading pair slabs

param(
    [switch]$SkipInit,
    [switch]$SkipSlabs
)

$ErrorActionPreference = "Stop"

# Configuration
$NETWORK = "devnet"
$KEYPAIRS_DIR = "keypairs"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Initializing Percolator Exchange" -ForegroundColor Cyan  
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Load config
$config = Get-Content "$KEYPAIRS_DIR/config.json" | ConvertFrom-Json

$devWallet = "$KEYPAIRS_DIR/devwallet.json"
$routerProgramId = $config.programs.router.program_id
$slabProgramId = $config.programs.slab.program_id
$ammProgramId = $config.programs.amm.program_id

Write-Host "Router Program: $routerProgramId" -ForegroundColor Green
Write-Host "Slab Program: $slabProgramId" -ForegroundColor Green
Write-Host "AMM Program: $ammProgramId" -ForegroundColor Green
Write-Host ""

# Set Solana to devnet
solana config set --url devnet | Out-Null

# Check balance
$balance = solana balance $devWallet
Write-Host "Dev Wallet Balance: $balance" -ForegroundColor Yellow
Write-Host ""

if (-not $SkipInit) {
    Write-Host "=== Step 1: Initialize Exchange (Router Registry) ===" -ForegroundColor Cyan
    Write-Host "This creates the global registry account for the exchange." -ForegroundColor Gray
    Write-Host ""
    
    # Note: The actual initialization requires specific instruction format
    # This is a placeholder - the CLI would normally handle this
    Write-Host "⚠ Router registry initialization requires CLI tool" -ForegroundColor Yellow
    Write-Host "  The registry will be auto-created on first use" -ForegroundColor Gray
    Write-Host ""
} else {
    Write-Host "Skipping exchange initialization" -ForegroundColor Yellow
}

if (-not $SkipSlabs) {
    Write-Host "=== Step 2: Create Trading Pair Slabs ===" -ForegroundColor Cyan
    Write-Host "Creating 3 order book accounts:" -ForegroundColor Gray
    Write-Host ""
    
    # Trading pairs to create
    $tradingPairs = @(
        @{
            symbol = "SOL-USD"
            tickSize = 1
            lotSize = 1000
            markPrice = 15000  # $150.00 in cents
        },
        @{
            symbol = "ETH-USD"
            tickSize = 1
            lotSize = 1000
            markPrice = 250000  # $2500.00 in cents
        },
        @{
            symbol = "BTC-USD"
            tickSize = 1
            lotSize = 100
            markPrice = 4500000  # $45000.00 in cents
        }
    )
    
    foreach ($pair in $tradingPairs) {
        Write-Host "Creating $($pair.symbol) slab..." -ForegroundColor Green
        Write-Host "  Tick Size: $($pair.tickSize)" -ForegroundColor Gray
        Write-Host "  Lot Size: $($pair.lotSize)" -ForegroundColor Gray
        Write-Host "  Initial Mark Price: `$$($pair.markPrice / 100)" -ForegroundColor Gray
        
        # Note: Creating slabs requires specific instruction format
        # This requires the CLI tool or manual transaction building
        Write-Host "  ⚠ Slab creation requires CLI tool or manual transaction" -ForegroundColor Yellow
        Write-Host ""
    }
    
    Write-Host "⚠ To create slabs, you need to:" -ForegroundColor Yellow
    Write-Host "  1. Build CLI in WSL (no OpenSSL issues there)" -ForegroundColor Gray
    Write-Host "  2. Or use the test scripts to create them" -ForegroundColor Gray
    Write-Host ""
} else {
    Write-Host "Skipping slab creation" -ForegroundColor Yellow
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Next Steps" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "To complete initialization, run in WSL:" -ForegroundColor Yellow
Write-Host ""
Write-Host "cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator-v2" -ForegroundColor White
Write-Host "cargo build --release -p percolator-cli" -ForegroundColor White
Write-Host ""
Write-Host "# Initialize exchange" -ForegroundColor White
Write-Host "./target/release/percolator -n devnet -k keypairs/devwallet.json init --name 'Percolator DEX'" -ForegroundColor White
Write-Host ""
Write-Host "# Create SOL/USD slab" -ForegroundColor White
Write-Host "./target/release/percolator -n devnet -k keypairs/devwallet.json matcher create --symbol SOL-USD --tick-size 1 --lot-size 1000" -ForegroundColor White
Write-Host ""
Write-Host "# Create ETH/USD slab" -ForegroundColor White
Write-Host "./target/release/percolator -n devnet -k keypairs/devwallet.json matcher create --symbol ETH-USD --tick-size 1 --lot-size 1000" -ForegroundColor White
Write-Host ""
Write-Host "# Create BTC/USD slab" -ForegroundColor White
Write-Host "./target/release/percolator -n devnet -k keypairs/devwallet.json matcher create --symbol BTC-USD --tick-size 1 --lot-size 100" -ForegroundColor White
Write-Host ""

