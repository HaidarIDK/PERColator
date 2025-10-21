#!/usr/bin/env pwsh
# Test Devnet Trade - Execute a real transaction on Solana devnet

Write-Host "🚀 PERCOLATOR DEVNET TRADE TEST" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

# Configuration
$API_URL = "http://localhost:3000"
$WALLET = "BWiQzXy8tDWNMFct1pYNW8hCh5QK9d3g2Ujvq9CheXRx"  # Your Phantom wallet

Write-Host "📋 Configuration:" -ForegroundColor Yellow
Write-Host "   API URL: $API_URL"
Write-Host "   Wallet:  $WALLET"
Write-Host ""

# Step 1: Check if backend is running
Write-Host "1️⃣  Checking backend status..." -ForegroundColor Green
try {
    $health = Invoke-RestMethod -Uri "$API_URL/health" -Method Get -TimeoutSec 5
    Write-Host "   ✅ Backend is running!" -ForegroundColor Green
    Write-Host "   Status: $($health.status)" -ForegroundColor Gray
} catch {
    Write-Host "   ❌ Backend is not running!" -ForegroundColor Red
    Write-Host "   Please start: cd api && npm run dev" -ForegroundColor Yellow
    exit 1
}
Write-Host ""

# Step 2: Create a Reserve transaction
Write-Host "2️⃣  Creating RESERVE transaction..." -ForegroundColor Green
Write-Host "   Market: ETH/USDC" -ForegroundColor Gray
Write-Host "   Side: BUY" -ForegroundColor Gray
Write-Host "   Quantity: 0.5 ETH" -ForegroundColor Gray
Write-Host "   Price: $3,900" -ForegroundColor Gray
Write-Host ""

$reservePayload = @{
    wallet = $WALLET
    market = "ETHUSDC"
    side = "buy"
    quantity = 0.5
    price = 3900
    orderType = "limit"
} | ConvertTo-Json

Write-Host "   Sending reserve request..." -ForegroundColor Gray
try {
    $reserveResponse = Invoke-RestMethod -Uri "$API_URL/api/trade/reserve" -Method Post -Body $reservePayload -ContentType "application/json"
    
    Write-Host "   ✅ RESERVE SUCCESSFUL!" -ForegroundColor Green
    Write-Host ""
    Write-Host "   📊 Reserve Details:" -ForegroundColor Cyan
    Write-Host "   Hold ID:     $($reserveResponse.holdId)" -ForegroundColor White
    Write-Host "   VWAP Price:  `$$($reserveResponse.vwapPrice)" -ForegroundColor White
    Write-Host "   Filled Qty:  $($reserveResponse.filledQty) ETH" -ForegroundColor White
    Write-Host "   Max Charge:  `$$($reserveResponse.maxCharge)" -ForegroundColor White
    Write-Host "   Expires:     $(Get-Date -UnixTimeMilliseconds $($reserveResponse.expiryMs) -Format 'HH:mm:ss')" -ForegroundColor White
    
    if ($reserveResponse.transaction) {
        Write-Host ""
        Write-Host "   🔗 Transaction (serialized):" -ForegroundColor Yellow
        Write-Host "   $($reserveResponse.transaction.Substring(0, 60))..." -ForegroundColor Gray
    }
    
    $holdId = $reserveResponse.holdId
    
} catch {
    Write-Host "   ❌ Reserve failed!" -ForegroundColor Red
    Write-Host "   Error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Response: $($_.ErrorDetails.Message)" -ForegroundColor Gray
    exit 1
}
Write-Host ""

# Step 3: Wait a moment
Write-Host "3️⃣  Waiting 2 seconds..." -ForegroundColor Green
Start-Sleep -Seconds 2
Write-Host ""

# Step 4: Commit the reservation
Write-Host "4️⃣  Creating COMMIT transaction..." -ForegroundColor Green
Write-Host "   Hold ID: $holdId" -ForegroundColor Gray
Write-Host ""

$commitPayload = @{
    wallet = $WALLET
    holdId = $holdId
} | ConvertTo-Json

Write-Host "   Sending commit request..." -ForegroundColor Gray
try {
    $commitResponse = Invoke-RestMethod -Uri "$API_URL/api/trade/commit" -Method Post -Body $commitPayload -ContentType "application/json"
    
    Write-Host "   ✅ COMMIT SUCCESSFUL!" -ForegroundColor Green
    Write-Host ""
    Write-Host "   📊 Commit Details:" -ForegroundColor Cyan
    Write-Host "   Filled Qty:  $($commitResponse.filledQty) ETH" -ForegroundColor White
    Write-Host "   Avg Price:   `$$($commitResponse.avgPrice)" -ForegroundColor White
    Write-Host "   Total Fee:   `$$($commitResponse.totalFee)" -ForegroundColor White
    Write-Host "   Total Cost:  `$$($commitResponse.totalDebit)" -ForegroundColor White
    
    if ($commitResponse.transaction) {
        Write-Host ""
        Write-Host "   🔗 Transaction (serialized):" -ForegroundColor Yellow
        Write-Host "   $($commitResponse.transaction.Substring(0, 60))..." -ForegroundColor Gray
    }
    
} catch {
    Write-Host "   ❌ Commit failed!" -ForegroundColor Red
    Write-Host "   Error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Response: $($_.ErrorDetails.Message)" -ForegroundColor Gray
    exit 1
}
Write-Host ""

# Step 5: Summary
Write-Host "═══════════════════════════════════════" -ForegroundColor Cyan
Write-Host "✅ TRADE COMPLETED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "═══════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "📈 Trade Summary:" -ForegroundColor Yellow
Write-Host "   Market:      ETH/USDC" -ForegroundColor White
Write-Host "   Side:        BUY" -ForegroundColor Green
Write-Host "   Quantity:    0.5 ETH" -ForegroundColor White
Write-Host "   Avg Price:   `$$($commitResponse.avgPrice)" -ForegroundColor White
Write-Host "   Total Cost:  `$$($commitResponse.totalDebit)" -ForegroundColor White
Write-Host ""
Write-Host "🎉 Your transactions have been created!" -ForegroundColor Magenta
Write-Host ""
Write-Host "⚠️  Note: These transactions were CREATED by the backend" -ForegroundColor Yellow
Write-Host "   but need to be SIGNED by Phantom wallet to execute on-chain." -ForegroundColor Yellow
Write-Host ""
Write-Host "💡 To get real on-chain signatures:" -ForegroundColor Cyan
Write-Host "   1. Go to http://localhost:3001/dashboard" -ForegroundColor White
Write-Host "   2. Click 'BUY' button" -ForegroundColor White
Write-Host "   3. Approve transaction in Phantom" -ForegroundColor White
Write-Host "   4. Check Solana Explorer for tx signature!" -ForegroundColor White
Write-Host ""

