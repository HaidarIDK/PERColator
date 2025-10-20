#!/bin/bash
# Setup script for Percolator API server

echo "🚀 Setting up Percolator API server..."

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
  echo "📝 Creating .env file..."
  cat > .env << EOF
# Solana Configuration
SOLANA_RPC_URL=http://localhost:8899
SOLANA_NETWORK=localnet

# API Server
PORT=3000
HOST=0.0.0.0

# Program IDs (update after deployment)
SLAB_PROGRAM_ID=YourSlabProgramIdHere

# Keypair path for transaction signing (optional, for testing)
WALLET_PATH=~/.config/solana/id.json
EOF
  echo "✅ .env file created"
else
  echo "ℹ️  .env file already exists"
fi

# Install dependencies
echo "📦 Installing dependencies..."
npm install

echo ""
echo "✅ Setup complete!"
echo ""
echo "To start the server:"
echo "  npm run dev     # Development mode with auto-reload"
echo "  npm run build   # Build for production"
echo "  npm start       # Start production server"
echo ""
echo "⚠️  Don't forget to update SLAB_PROGRAM_ID in .env after deployment!"

