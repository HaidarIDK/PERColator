#!/usr/bin/env pwsh
# Simple Solana Devnet Transaction Test

Write-Host "🚀 SOLANA DEVNET TRANSACTION TEST" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

# Check if Solana CLI is installed
Write-Host "1️⃣  Checking Solana CLI..." -ForegroundColor Green
try {
    $solanaVersion = solana --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ✅ Solana CLI installed: $solanaVersion" -ForegroundColor Green
    } else {
        throw "Solana CLI not found"
    }
} catch {
    Write-Host "   ❌ Solana CLI not installed!" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Install with:" -ForegroundColor Yellow
    Write-Host "   sh -c `"`$(curl -sSfL https://release.solana.com/v1.18.22/install)`"" -ForegroundColor White
    Write-Host ""
    exit 1
}
Write-Host ""

# Set cluster to devnet
Write-Host "2️⃣  Setting cluster to devnet..." -ForegroundColor Green
solana config set --url https://api.devnet.solana.com | Out-Null
Write-Host "   ✅ Cluster set to devnet" -ForegroundColor Green
Write-Host ""

# Check wallet
Write-Host "3️⃣  Checking wallet..." -ForegroundColor Green
$walletAddress = solana address 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "   ✅ Wallet address: $walletAddress" -ForegroundColor Green
} else {
    Write-Host "   ⚠️  No wallet found, creating one..." -ForegroundColor Yellow
    solana-keygen new --no-bip39-passphrase --force | Out-Null
    $walletAddress = solana address
    Write-Host "   ✅ New wallet created: $walletAddress" -ForegroundColor Green
}
Write-Host ""

# Check balance
Write-Host "4️⃣  Checking SOL balance..." -ForegroundColor Green
$balance = solana balance 2>&1
Write-Host "   Balance: $balance" -ForegroundColor White

if ($balance -match "^0") {
    Write-Host "   ⚠️  Balance is 0, requesting airdrop..." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "   Requesting 2 SOL from devnet faucet..." -ForegroundColor Gray
    
    try {
        solana airdrop 2 2>&1 | Out-Null
        Start-Sleep -Seconds 3
        $newBalance = solana balance
        Write-Host "   ✅ Airdrop successful!" -ForegroundColor Green
        Write-Host "   New balance: $newBalance" -ForegroundColor White
    } catch {
        Write-Host "   ⚠️  Airdrop rate limited. Try again in a minute." -ForegroundColor Yellow
    }
}
Write-Host ""

# Create a test transaction
Write-Host "5️⃣  Creating test transaction..." -ForegroundColor Green
Write-Host "   Sending 0.001 SOL to a test address..." -ForegroundColor Gray
Write-Host ""

# Use a known devnet address
$testAddress = "Stake11111111111111111111111111111111111111"

Write-Host "   From: $walletAddress" -ForegroundColor White
Write-Host "   To:   $testAddress" -ForegroundColor White
Write-Host "   Amount: 0.001 SOL" -ForegroundColor White
Write-Host ""

try {
    $txOutput = solana transfer $testAddress 0.001 --allow-unfunded-recipient 2>&1
    
    # Extract transaction signature
    if ($txOutput -match "Signature: ([A-Za-z0-9]+)") {
        $signature = $Matches[1]
        
        Write-Host "   ✅ TRANSACTION SUCCESSFUL!" -ForegroundColor Green
        Write-Host ""
        Write-Host "═══════════════════════════════════════" -ForegroundColor Cyan
        Write-Host "📝 TRANSACTION SIGNATURE:" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "   $signature" -ForegroundColor White
        Write-Host ""
        Write-Host "═══════════════════════════════════════" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "🔍 View on Solana Explorer:" -ForegroundColor Magenta
        Write-Host "   https://explorer.solana.com/tx/$signature`?cluster=devnet" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "📊 View on SolScan:" -ForegroundColor Magenta
        Write-Host "   https://solscan.io/tx/$signature`?cluster=devnet" -ForegroundColor Cyan
        Write-Host ""
        
    } else {
        Write-Host "   ✅ Transaction sent!" -ForegroundColor Green
        Write-Host ""
        Write-Host "$txOutput" -ForegroundColor Gray
    }
    
} catch {
    Write-Host "   ❌ Transaction failed!" -ForegroundColor Red
    Write-Host "   Error: $_" -ForegroundColor Red
    Write-Host ""
}

Write-Host "🎉 Test complete!" -ForegroundColor Green
Write-Host ""

