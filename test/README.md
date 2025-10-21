# Test Scripts

This directory contains test scripts for verifying various components of the Percolator system.

## Available Tests

### 1. `test-fetch.js`
Tests the fetch API availability and basic CoinGecko connection.

**Usage:**
```bash
node test/test-fetch.js
```

**What it tests:**
- ✅ Verifies `fetch` is available globally (Node.js 18+)
- ✅ Tests CoinGecko API connectivity
- ✅ Fetches Ethereum price data
- ✅ Displays formatted price, volume, and market cap data

**Expected Output:**
```
✅ fetch is available globally
✅ CoinGecko API Response:
{
  "ethereum": {
    "usd": 3859.02,
    "usd_24h_vol": 33820000000,
    "usd_24h_change": -4.95,
    "usd_market_cap": 464500000000
  }
}
```

---

### 2. `test-all-coins.js`
Tests fetching data for all three coins (BTC, ETH, SOL) from CoinGecko.

**Usage:**
```bash
node test/test-all-coins.js
```

**What it tests:**
- ✅ Fetches Bitcoin, Ethereum, and Solana data in one API call
- ✅ Displays raw JSON response from CoinGecko
- ✅ Validates data structure and availability for all three coins

**Expected Output:**
```
✅ CoinGecko response received

{
  "bitcoin": {
    "usd": 107827,
    "usd_market_cap": 2144408357836.2017,
    "usd_24h_vol": 60404213419.97344,
    "usd_24h_change": -3.0810231738217793
  },
  "ethereum": {
    "usd": 3869.48,
    "usd_market_cap": 466095520918.25006,
    "usd_24h_vol": 34159022560.936665,
    "usd_24h_change": -5.073972403498553
  },
  "solana": {
    "usd": 183.89,
    "usd_market_cap": 100439366083.66711,
    "usd_24h_vol": 5897971275.612155,
    "usd_24h_change": -4.762098730187461
  }
}

✅ All tests passed! All coins fetched successfully.
```

---

## Requirements

- **Node.js 18+** (for native `fetch` support)
- Internet connection (for CoinGecko API)

## Notes

- These tests use the free CoinGecko API (no API key required)
- Rate limits: 10-50 calls/minute (depending on endpoint)
- Data updates every ~60 seconds on CoinGecko

## Troubleshooting

### `fetch is not available`
**Solution:** Upgrade to Node.js 18 or higher
```bash
node --version  # Should be v18.0.0 or higher
```

### `HTTP Error 429`
**Solution:** You've hit the rate limit. Wait 60 seconds and try again.

### Connection timeout
**Solution:** Check your internet connection and firewall settings.

---

## Adding More Tests

To add more test scripts:

1. Create a new `.js` file in this directory
2. Use the same error handling pattern
3. Add documentation to this README
4. Make the script executable with `node test/your-test.js`

