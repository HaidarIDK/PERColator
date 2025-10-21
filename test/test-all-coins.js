/**
 * Test script to fetch data for all three porp coins (ETH, BTC, SOL) from CoinGecko
 * Run with: node test/test-all-coins.js
 * 
 * 
 */

console.log('Testing CoinGecko API for all coins (BTC, ETH, SOL)...\n');

async function fetchAllCoins() {
  try {
    const url = 'https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true&include_market_cap=true';
    
    console.log('Fetching data from CoinGecko...');
    console.log(`URL: ${url}\n`);
    
    const response = await fetch(url);
    
    if (!response.ok) {
      console.error(`HTTP Error: ${response.status} ${response.statusText}`);
      const text = await response.text();
      console.error('Response:', text);
      process.exit(1);
    }
    
    const data = await response.json();
    
    console.log('CoinGecko response received\n');
    console.log(JSON.stringify(data, null, 2));
    console.log('\n All tests passed! All coins fetched successfully.');
    
  } catch (error) {
    console.error('\n Error fetching data from CoinGecko:');
    console.error('Message:', error.message);
    console.error('Stack:', error.stack);
    process.exit(1);
  }
}

// Check if fetch is available
if (typeof fetch === 'undefined') {
  console.error(' fetch is not available globally');
  console.log('Make sure you are using Node.js 18+ or have node-fetch installed');
  process.exit(1);
}

fetchAllCoins();

