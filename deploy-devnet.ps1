# Deploy Percolator programs to Solana devnet (PowerShell version)

Write-Host "🚀 Deploying Percolator programs to devnet..." -ForegroundColor Cyan
Write-Host ""

# Check if solana CLI is installed
if (!(Get-Command solana -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Solana CLI not found!" -ForegroundColor Red
    Write-Host "Install it with:"
    Write-Host '  sh -c "$(curl -sSfL https://release.solana.com/stable/install)"'
    exit 1
}

# Configure for devnet
Write-Host "⚙️  Configuring Solana CLI for devnet..." -ForegroundColor Yellow
solana config set --url https://api.devnet.solana.com

# Check wallet balance
Write-Host ""
Write-Host "💰 Checking wallet balance..." -ForegroundColor Yellow
$BALANCE = (solana balance --lamports)
$MIN_BALANCE = 10000000000  # 10 SOL

if ([long]$BALANCE -lt $MIN_BALANCE) {
    Write-Host "⚠️  Low balance detected: $BALANCE lamports" -ForegroundColor Yellow
    Write-Host "📥 Requesting airdrop..." -ForegroundColor Yellow
    solana airdrop 2
    Start-Sleep -Seconds 2
}

# Build programs first
Write-Host ""
Write-Host "🔨 Building programs..." -ForegroundColor Yellow
.\build-bpf.ps1

# Deploy Router program
Write-Host ""
Write-Host "📤 Deploying Router program..." -ForegroundColor Yellow
$ROUTER_DEPLOY = solana program deploy target/deploy/percolator_router.so --output json | ConvertFrom-Json
$ROUTER_PROGRAM_ID = $ROUTER_DEPLOY.programId
Write-Host "✅ Router deployed: $ROUTER_PROGRAM_ID" -ForegroundColor Green

# Deploy Slab program
Write-Host ""
Write-Host "📤 Deploying Slab program..." -ForegroundColor Yellow
$SLAB_DEPLOY = solana program deploy target/deploy/percolator_slab.so --output json | ConvertFrom-Json
$SLAB_PROGRAM_ID = $SLAB_DEPLOY.programId
Write-Host "✅ Slab deployed: $SLAB_PROGRAM_ID" -ForegroundColor Green

# Save program IDs
Write-Host ""
Write-Host "💾 Saving program IDs to .env..." -ForegroundColor Yellow
$DATE = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$ENV_CONTENT = @"
# Percolator Devnet Program IDs
# Generated: $DATE

ROUTER_PROGRAM_ID=$ROUTER_PROGRAM_ID
SLAB_PROGRAM_ID=$SLAB_PROGRAM_ID
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_NETWORK=devnet
"@

$ENV_CONTENT | Out-File -FilePath api/.env.devnet -Encoding utf8

Write-Host ""
Write-Host "🎉 Deployment complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Program IDs saved to api/.env.devnet"
Write-Host "Verify on Solscan:"
Write-Host "  Router: https://solscan.io/account/$ROUTER_PROGRAM_ID`?cluster=devnet" -ForegroundColor Cyan
Write-Host "  Slab:   https://solscan.io/account/$SLAB_PROGRAM_ID`?cluster=devnet" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:"
Write-Host "1. Copy api/.env.devnet to api/.env"
Write-Host "2. Start API server: cd api && npm run dev"
Write-Host "3. Test with: curl http://localhost:3000/api/health"

