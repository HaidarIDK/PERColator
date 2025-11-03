#!/bin/bash
# Quick API endpoint tester

API_URL="http://localhost:3000"
SLAB_ADDR="11111111111111111111111111111111"
USER_ADDR="22222222222222222222222222222222"

echo "🧪 Testing Percolator API endpoints..."
echo ""

# Test 1: Health check
echo "1️⃣ Testing health endpoint..."
curl -s "$API_URL/api/health" | jq '.'
echo ""

# Test 2: Get instruments
echo "2️⃣ Testing instruments endpoint..."
curl -s "$API_URL/api/market/instruments?slab=$SLAB_ADDR" | jq '.'
echo ""

# Test 3: Get orderbook
echo "3️⃣ Testing orderbook endpoint..."
curl -s "$API_URL/api/market/orderbook?slab=$SLAB_ADDR&instrument=0" | jq '.'
echo ""

# Test 4: Get recent trades
echo "4️⃣ Testing trades endpoint..."
curl -s "$API_URL/api/market/trades?slab=$SLAB_ADDR&instrument=0&limit=5" | jq '.'
echo ""

# Test 5: Get user balance
echo "5️⃣ Testing user balance endpoint..."
curl -s "$API_URL/api/user/balance?slab=$SLAB_ADDR&user=$USER_ADDR" | jq '.'
echo ""

# Test 6: Get user portfolio
echo "6️⃣ Testing portfolio endpoint..."
curl -s "$API_URL/api/user/portfolio?slab=$SLAB_ADDR&user=$USER_ADDR" | jq '.'
echo ""

echo "✅ All tests complete!"
echo ""
echo "💡 To test WebSocket, use:"
echo "   wscat -c ws://localhost:3000/ws"
echo "   Then send: {\"type\":\"subscribe\",\"channel\":\"orderbook:BTC/USDC\"}"

