// Quick endpoint tester for PERColator API
// Usage: node test-endpoints.js

const BASE_URL = 'http://localhost:3000';

async function testEndpoint(name, url, options = {}) {
  try {
    const response = await fetch(url, options);
    const data = await response.json();
    console.log(`✓ ${name}`);
    console.log(`  Status: ${response.status}`);
    console.log(`  Response:`, JSON.stringify(data).substring(0, 100) + '...');
    console.log('');
    return data;
  } catch (error) {
    console.log(`✗ ${name}`);
    console.log(`  Error: ${error.message}`);
    console.log('');
  }
}

async function runTests() {
  console.log('Testing PERColator API Endpoints\n');
  console.log('='.repeat(50));
  console.log('');

  // Root
  await testEndpoint('Root', `${BASE_URL}/`);

  // Health
  await testEndpoint('Health Check', `${BASE_URL}/api/health`);

  // Market Data
  await testEndpoint(
    'Instruments',
    `${BASE_URL}/api/market/instruments?slab=test`
  );

  await testEndpoint(
    'Orderbook',
    `${BASE_URL}/api/market/orderbook?slab=test&instrument=0`
  );

  await testEndpoint(
    'Recent Trades',
    `${BASE_URL}/api/market/trades?slab=test&instrument=0&limit=5`
  );

  await testEndpoint(
    'Market Stats',
    `${BASE_URL}/api/market/stats?slab=test`
  );

  // User Data
  await testEndpoint(
    'User Balance',
    `${BASE_URL}/api/user/balance?slab=test&user=testuser`
  );

  await testEndpoint(
    'User Positions',
    `${BASE_URL}/api/user/positions?slab=test&user=testuser`
  );

  await testEndpoint(
    'User Orders',
    `${BASE_URL}/api/user/orders?slab=test&user=testuser`
  );

  await testEndpoint(
    'User Portfolio',
    `${BASE_URL}/api/user/portfolio?slab=test&user=testuser`
  );

  // Router
  await testEndpoint(
    'Router Slabs',
    `${BASE_URL}/api/router/slabs`
  );

  await testEndpoint(
    'Router Portfolio',
    `${BASE_URL}/api/router/portfolio/testuser`
  );

  await testEndpoint(
    'Vault Info',
    `${BASE_URL}/api/router/vault/USDC`
  );

  // Trading Operations (POST)
  await testEndpoint(
    'Place Order',
    `${BASE_URL}/api/trade/order`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        slab: 'test',
        user: 'testuser',
        instrument: 0,
        side: 'buy',
        price: 65000,
        qty: 1.0
      })
    }
  );

  await testEndpoint(
    'Reserve',
    `${BASE_URL}/api/trade/reserve`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        slab: 'test',
        user: 'testuser',
        instrument: 0,
        side: 'buy',
        qty: 1.0,
        limit_px: 65000
      })
    }
  );

  await testEndpoint(
    'Deposit',
    `${BASE_URL}/api/router/deposit`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        user: 'testuser',
        mint: 'USDC',
        amount: 1000
      })
    }
  );

  console.log('='.repeat(50));
  console.log('\nAll tests complete!');
  console.log('\nWebSocket: ws://localhost:3000/ws');
  console.log('Full docs: api/ENDPOINTS.md\n');
}

runTests().catch(console.error);

