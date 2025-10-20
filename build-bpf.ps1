# Build Percolator programs for Solana BPF target (PowerShell version)

Write-Host "🔨 Building Percolator programs for Solana BPF..." -ForegroundColor Cyan
Write-Host ""

# Check if cargo-build-sbf is installed
if (!(Get-Command cargo-build-sbf -ErrorAction SilentlyContinue)) {
    Write-Host "❌ cargo-build-sbf not found!" -ForegroundColor Red
    Write-Host "Install Solana CLI tools first:"
    Write-Host '  sh -c "$(curl -sSfL https://release.solana.com/stable/install)"'
    exit 1
}

# Build Router program
Write-Host "📦 Building Router program..." -ForegroundColor Yellow
cargo-build-sbf --manifest-path programs/router/Cargo.toml

# Build Slab program
Write-Host "📦 Building Slab program..." -ForegroundColor Yellow
cargo-build-sbf --manifest-path programs/slab/Cargo.toml

Write-Host ""
Write-Host "✅ Build complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Program binaries:"
Write-Host "  Router: target/deploy/percolator_router.so" -ForegroundColor White
Write-Host "  Slab:   target/deploy/percolator_slab.so" -ForegroundColor White
Write-Host ""
Write-Host "To deploy to devnet:"
Write-Host "  .\deploy-devnet.ps1" -ForegroundColor Cyan

