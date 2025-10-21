# Start both Backend API and Frontend servers
Write-Host "🚀 Starting Percolator DEX Servers..." -ForegroundColor Cyan
Write-Host ""

# Start Backend in new window
Write-Host "📡 Starting Backend API Server..." -ForegroundColor Yellow
Start-Process pwsh -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot\api'; Write-Host '=== BACKEND API SERVER ===' -ForegroundColor Green; npm run dev"

# Wait a bit for backend to start
Start-Sleep -Seconds 3

# Start Frontend in new window
Write-Host "🎨 Starting Frontend Dev Server..." -ForegroundColor Yellow
Start-Process pwsh -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot\frontend'; Write-Host '=== FRONTEND DEV SERVER ===' -ForegroundColor Blue; npm run dev"

Write-Host ""
Write-Host "✅ Both servers starting in separate windows!" -ForegroundColor Green
Write-Host ""
Write-Host "📍 Backend API:  http://localhost:3000" -ForegroundColor Cyan
Write-Host "📍 Frontend UI:  http://localhost:3001" -ForegroundColor Cyan
Write-Host "📍 Dashboard:    http://localhost:3001/dashboard" -ForegroundColor Magenta
Write-Host ""
Write-Host "Press Ctrl+C in each window to stop the servers" -ForegroundColor Yellow

