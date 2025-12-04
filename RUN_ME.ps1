# Quick script to run the interactive CLI
# This script will guide you through the process

Write-Host "=== Percolator Interactive CLI Launcher ===" -ForegroundColor Cyan
Write-Host ""

# Check if CLI is built
$cliPath = "target\release\percolator.exe"
if (Test-Path $cliPath) {
    Write-Host "✅ CLI is built!" -ForegroundColor Green
    Write-Host ""
    
    # Check balance
    Write-Host "Checking devnet SOL balance..." -ForegroundColor Yellow
    $balance = solana balance 2>&1
    Write-Host $balance -ForegroundColor Gray
    
    # Check if balance is low
    if ($balance -match "(\d+\.\d+) SOL") {
        $solBalance = [double]$matches[1]
        if ($solBalance -lt 1.0) {
            Write-Host ""
            Write-Host "⚠️  Low balance detected! Getting devnet SOL..." -ForegroundColor Yellow
            solana airdrop 2
            Write-Host ""
        }
    }
    
    Write-Host "🚀 Starting Interactive CLI..." -ForegroundColor Green
    Write-Host ""
    
    # Run the CLI
    & $cliPath -n devnet interactive
} else {
    Write-Host "⚠️  CLI is not built yet!" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Building options:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Option 1: Build on Windows (requires OpenSSL)" -ForegroundColor White
    Write-Host "  cargo build --release -p percolator-cli" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Option 2: Use WSL (easiest, no OpenSSL setup)" -ForegroundColor White
    Write-Host "  wsl" -ForegroundColor Gray
    Write-Host "  cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator" -ForegroundColor Gray
    Write-Host "  cargo build --release -p percolator-cli" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Option 3: Install OpenSSL via Chocolatey" -ForegroundColor White
    Write-Host "  choco install openssl" -ForegroundColor Gray
    Write-Host "  cargo build --release -p percolator-cli" -ForegroundColor Gray
    Write-Host ""
    Write-Host "See cli/RUN_CLI.md for detailed instructions" -ForegroundColor Yellow
    Write-Host ""
    
    # Ask if they want to try building
    $response = Read-Host "Try building now? (y/n)"
    if ($response -eq "y" -or $response -eq "Y") {
        Write-Host ""
        Write-Host "Building CLI..." -ForegroundColor Yellow
        cargo build --release -p percolator-cli
    }
}










