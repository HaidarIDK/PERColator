# Create all trading pairs (SOL/USDC, ETH/USDC, BTC/USDC) after exchange init
# FORK-SAFE: Only creates on-chain accounts - no repo modifications

param(
    [Parameter(Mandatory=$true)]
    [string]$RegistryAddress
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$KEYPAIRS_DIR = "keypairs"
$devWalletPath = Join-Path $KEYPAIRS_DIR "devwallet.json"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Creating Trading Pairs" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Registry: $RegistryAddress" -ForegroundColor White
Write-Host ""

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

# Trading pairs to create
$tradingPairs = @(
    @{ Symbol = "SOL-USD"; Name = "SOL/USDC"; TickSize = 100; LotSize = 1000 },
    @{ Symbol = "ETH-USD"; Name = "ETH/USDC"; TickSize = 100; LotSize = 1000 },
    @{ Symbol = "BTC-USD"; Name = "BTC/USDC"; TickSize = 100; LotSize = 1000 }
)

Write-Host "Creating $($tradingPairs.Count) order books..." -ForegroundColor Yellow
Write-Host ""

foreach ($pair in $tradingPairs) {
    Write-Host "Creating $($pair.Name) order book ($($pair.Symbol))..." -ForegroundColor Cyan
    
    & $cliPath -n devnet --keypair $devWalletPath matcher create `
        $RegistryAddress `
        $pair.Symbol `
        --tick-size $pair.TickSize `
        --lot-size $pair.LotSize
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✓ $($pair.Name) created successfully!" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ $($pair.Name) creation may have failed" -ForegroundColor Yellow
    }
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "All trading pairs created!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Your exchange now has:" -ForegroundColor Yellow
Write-Host "  ✓ SOL/USDC perpetual futures" -ForegroundColor White
Write-Host "  ✓ ETH/USDC perpetual futures" -ForegroundColor White
Write-Host "  ✓ BTC/USDC perpetual futures" -ForegroundColor White
Write-Host ""
Write-Host "Users can now trade these pairs with leverage!" -ForegroundColor Green
Write-Host ""

