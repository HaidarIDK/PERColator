/**
 * Test script to verify fetch API availability and CoinGecko response
 * Run with: node test/test-fetch.js
 */

console.log('Testing fetch API and CoinGecko connection...\n');

// Check if fetch is available
if (typeof fetch === 'undefined') {
  console.error('fetch is not available globally');
  console.log('Make sure you are using Node.js 18+ or install node-fetch');
  process.exit(1);
} else {
  console.log(' fetch is available globally');
}

// Test CoinGecko API
async function testCoinGecko() {
  try {
    console.log('\nTesting CoinGecko API...');
    const coinId = 'ethereum';
    const url = `https://api.coingecko.com/api/v3/simple/price?ids=${coinId}&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true&include_market_cap=true`;
    
    console.log(`Fetching: ${url}\n`);
    
    const response = await fetch(url);
    
    if (!response.ok) {
      console.error(`HTTP Error: ${response.status} ${response.statusText}`);
      const text = await response.text();
      console.error('Response:', text);
      process.exit(1);
    }
    
    const data = await response.json();
    
    console.log('CoinGecko API Response:');
    console.log(JSON.stringify(data, null, 2));
    
    if (data[coinId]) {
      console.log('\n Data Structure:');
      console.log(`   Price: $${data[coinId].usd}`);
      console.log(`   24h Volume: $${(data[coinId].usd_24h_vol / 1e9).toFixed(2)}B`);
      console.log(`   24h Change: ${data[coinId].usd_24h_change.toFixed(2)}%`);
      console.log(`   Market Cap: $${(data[coinId].usd_market_cap / 1e9).toFixed(2)}B`);
    }
    
    console.log('\nAll tests passed!');
    
  } catch (error) {
    console.error(' Error:', error.message);
    console.error('Stack:', error.stack);
    process.exit(1);
  }
}

testCoinGecko();

