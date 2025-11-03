import { Router } from 'express';

export const dashboardRouter = Router();

// Real-time market data from CoinGecko API
// Initialize with fallback data to prevent 503 errors on startup
let ethData: any = { price: 3931.15, priceChangePercent24h: -2.14, volume24h: 34626730527, marketCap: 474592579277, high24h: 4082.02, low24h: 3926.31 };
let solData: any = { price: 186.12, priceChangePercent24h: -2.27, volume24h: 5808295794, marketCap: 101723519950, high24h: 194.13, low24h: 185.96 };
let btcData: any = { price: 98765.43, priceChangePercent24h: 1.50, volume24h: 45000000000, marketCap: 1950000000000, high24h: 99500, low24h: 97000 };

// Fetch real market data from CoinGecko
async function fetchCoinGeckoData() {
  try {
    console.log('Fetching data from CoinGecko...');
    
    // Check if fetch is available
    if (typeof fetch === 'undefined') {
      console.error('Fetch is not available in this Node.js version');
      // Use fallback data
      ethData = { price: 3931.15, priceChangePercent24h: -2.14, volume24h: 34626730527, marketCap: 474592579277, high24h: 4082.02, low24h: 3926.31 };
      solData = { price: 186.12, priceChangePercent24h: -2.27, volume24h: 5808295794, marketCap: 101723519950, high24h: 194.13, low24h: 185.96 };
      btcData = { price: 98765.43, priceChangePercent24h: 1.50, volume24h: 45000000000, marketCap: 1950000000000, high24h: 99500, low24h: 97000 };
      console.log('Using fallback data - install node-fetch or upgrade Node.js');
      return;
    }
    
    // Fetch ETH, BTC, and SOL data in one call
    const response = await fetch(
      'https://api.coingecko.com/api/v3/coins/markets?vs_currency=usd&ids=bitcoin,ethereum,solana&order=market_cap_desc&per_page=3&page=1&sparkline=false&price_change_percentage=24h'
    );
    
    if (!response.ok) {
      console.error(`CoinGecko API error: ${response.status} ${response.statusText}`);
      const text = await response.text();
      console.error('Response:', text);
      console.log('CoinGecko API failed - continuing with fallback data');
      return;
    }
    
    const data: any = await response.json();
    console.log('CoinGecko response received:', data.length, 'coins');
    
    if (data && Array.isArray(data)) {
      // Parse each coin's data
      for (const coin of data) {
        if (coin.id === 'ethereum') {
          ethData = {
            price: coin.current_price || 0,
            priceChange24h: coin.price_change_24h || 0,
            priceChangePercent24h: coin.price_change_percentage_24h || 0,
            volume24h: coin.total_volume || 0, // In USD
            marketCap: coin.market_cap || 0,
            high24h: coin.high_24h || 0,
            low24h: coin.low_24h || 0,
            circulatingSupply: coin.circulating_supply || 0,
            ath: coin.ath || 0,
            atl: coin.atl || 0
          };
          console.log(`ETH: $${ethData.price.toFixed(2)}, Vol: $${(ethData.volume24h / 1e9).toFixed(2)}B, Change: ${ethData.priceChangePercent24h.toFixed(2)}%`);
        } else if (coin.id === 'bitcoin') {
          btcData = {
            price: coin.current_price || 0,
            priceChange24h: coin.price_change_24h || 0,
            priceChangePercent24h: coin.price_change_percentage_24h || 0,
            volume24h: coin.total_volume || 0, // In USD
            marketCap: coin.market_cap || 0,
            high24h: coin.high_24h || 0,
            low24h: coin.low_24h || 0,
            circulatingSupply: coin.circulating_supply || 0,
            ath: coin.ath || 0,
            atl: coin.atl || 0
          };
          console.log(`BTC: $${btcData.price.toFixed(2)}, Vol: $${(btcData.volume24h / 1e9).toFixed(2)}B, Change: ${btcData.priceChangePercent24h.toFixed(2)}%`);
        } else if (coin.id === 'solana') {
          solData = {
            price: coin.current_price || 0,
            priceChange24h: coin.price_change_24h || 0,
            priceChangePercent24h: coin.price_change_percentage_24h || 0,
            volume24h: coin.total_volume || 0, // In USD
            marketCap: coin.market_cap || 0,
            high24h: coin.high_24h || 0,
            low24h: coin.low_24h || 0,
            circulatingSupply: coin.circulating_supply || 0,
            ath: coin.ath || 0,
            atl: coin.atl || 0
          };
          console.log(`SOL: $${solData.price.toFixed(2)}, Vol: $${(solData.volume24h / 1e9).toFixed(2)}B, Change: ${solData.priceChangePercent24h.toFixed(2)}%`);
        }
      }
      
      // Calculate and log ratios
      if (ethData && solData) {
        const ethSolRatio = ethData.price / solData.price;
        console.log(`ETH/SOL Ratio: ${ethSolRatio.toFixed(4)}`);
      }
      if (btcData && solData) {
        const btcSolRatio = btcData.price / solData.price;
        console.log(`BTC/SOL Ratio: ${btcSolRatio.toFixed(4)}`);
      }
    } else {
      console.error('CoinGecko returned non-array data:', data);
    }
  } catch (error) {
    console.error('Failed to fetch data from CoinGecko:', error);
    console.log('Continuing with fallback/existing market data');
  }
}

// Wrapper to prevent unhandled errors
function safeFetchCoinGeckoData() {
  fetchCoinGeckoData().catch(err => {
    console.error('Unhandled error in fetchCoinGeckoData:', err);
  });
}

// Update prices every 60 seconds (CoinGecko free tier limit)
setInterval(safeFetchCoinGeckoData, 60000);

// Initialize with real data
safeFetchCoinGeckoData();

// ============================================
// HYPERLIQUID API HELPERS
// ============================================

/**
 * Map our symbol names to Hyperliquid coin names
 */
function mapSymbolToHyperliquidCoin(symbol: string): string {
  const symbolMap: { [key: string]: string } = {
    'BTC-PERP': 'BTC',
    'BTCUSDC': 'BTC',
    'ETH-PERP': 'ETH',
    'ETHUSDC': 'ETH',
    'SOL-PERP': 'SOL',
    'SOLUSDC': 'SOL',
    'ETHSOL': 'ETH', // For ratios, we'll use the base asset
    'ETH-SOL': 'ETH',
    'BTCSOL': 'BTC',
    'BTC-SOL': 'BTC'
  };
  
  return symbolMap[symbol] || 'BTC'; // Default to BTC
}

/**
 * Map timeframe to Hyperliquid interval format
 */
function mapTimeframeToHyperliquidInterval(timeframe: string): string {
  // Remove any trailing 'm', 'h', 'd' characters
  const numericTimeframe = timeframe.replace(/[mhd]$/i, '');
  
  const intervalMap: { [key: string]: string } = {
    '1': '1m',
    '5': '5m',
    '15': '15m',
    '30': '30m',
    '60': '1h',
    '240': '4h',
    '1440': '1d'
  };
  
  return intervalMap[numericTimeframe] || '15m'; // Default to 15m
}

/**
 * Fetch candlestick data from Hyperliquid API
 */
async function fetchHyperliquidCandles(coin: string, interval: string, limit: number, from?: number, to?: number): Promise<any[]> {
  try {
    const now = Date.now();
    let startTime: number;
    let endTime: number;
    
    if (from !== undefined && to !== undefined) {
      startTime = from;
      endTime = to;
    } else {
      // Calculate based on interval
      const intervalMinutes = parseInt(interval.replace(/[^\d]/g, ''));
      const intervalMs = intervalMinutes * 60 * 1000;
      startTime = now - (limit * intervalMs);
      endTime = now;
    }
    
    const payload = {
      type: "candleSnapshot",
      req: {
        coin: coin,
        interval: interval,
        startTime: startTime,
        endTime: endTime
      }
    };
    
    console.log(`📊 Fetching Hyperliquid candles for ${coin} ${interval}`);
    console.log(`   Time range: ${new Date(startTime).toISOString()} -> ${new Date(endTime).toISOString()}`);
    console.log(`   Payload:`, JSON.stringify(payload));
    
    const response = await fetch('https://api.hyperliquid.xyz/info', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload)
    });
    
    if (!response.ok) {
      const errorText = await response.text();
      console.error(`❌ Hyperliquid API error: ${response.status} ${response.statusText}`);
      console.error(`   Response: ${errorText}`);
      throw new Error(`Hyperliquid API error: ${response.status} - ${errorText}`);
    }
    
    const data = await response.json() as any[];
    
    // Check if response is an error object
    if (!Array.isArray(data)) {
      console.error(`❌ Hyperliquid returned non-array:`, data);
      throw new Error(`Hyperliquid returned invalid data format`);
    }
    
    if (data.length === 0) {
      console.warn(`⚠️  Hyperliquid returned 0 candles for ${coin} ${interval}`);
    } else {
      console.log(`✅ Hyperliquid response: ${data.length} candles`);
      console.log(`   First candle: ${new Date(data[0].t).toISOString()} - $${data[0].c}`);
      console.log(`   Last candle: ${new Date(data[data.length - 1].t).toISOString()} - $${data[data.length - 1].c}`);
    }
    
    // Transform Hyperliquid format to our format
    return data.map((candle: any) => ({
      time: Math.floor(candle.t / 1000), // Convert to seconds
      open: parseFloat(candle.o),
      high: parseFloat(candle.h),
      low: parseFloat(candle.l),
      close: parseFloat(candle.c),
      volume: parseFloat(candle.v)
    }));
    
  } catch (error) {
    console.error('❌ Failed to fetch Hyperliquid candles:', error);
    throw error;
  }
}

// ============================================
// ROUTES - Order matters! Specific before wildcards
// ============================================

/**
 * GET /api/market/list (or just /)
 * Get list of all available markets
 */
dashboardRouter.get('/list', (req, res) => {
  const markets = [
    {
      symbol: 'ETHSOL',
      name: 'ETH/SOL Ratio',
      baseAsset: 'ETH',
      quoteAsset: 'SOL',
      price: ethData && solData ? parseFloat((ethData.price / solData.price).toFixed(4)) : 0,
      change24h: ethData && solData 
        ? (ethData.priceChangePercent24h - solData.priceChangePercent24h).toFixed(2) + '%'
        : '0.00%',
      volume24h: ethData ? ethData.volume24h : 0,
      active: true
    },
    {
      symbol: 'BTCSOL',
      name: 'BTC/SOL Ratio',
      baseAsset: 'BTC',
      quoteAsset: 'SOL',
      price: btcData && solData ? parseFloat((btcData.price / solData.price).toFixed(4)) : 0,
      change24h: btcData && solData
        ? (btcData.priceChangePercent24h - solData.priceChangePercent24h).toFixed(2) + '%'
        : '0.00%',
      volume24h: btcData ? btcData.volume24h : 0,
      active: true
    },
    {
      symbol: 'ETH-PERP',
      name: 'Ethereum Spot',
      baseAsset: 'ETH',
      quoteAsset: 'USDC',
      price: ethData ? ethData.price : 0,
      change24h: ethData ? ethData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: ethData ? ethData.volume24h : 0,
      active: true
    },
    {
      symbol: 'BTC-PERP',
      name: 'Bitcoin Spot',
      baseAsset: 'BTC',
      quoteAsset: 'USDC',
      price: btcData ? btcData.price : 0,
      change24h: btcData ? btcData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: btcData ? btcData.volume24h : 0,
      active: true
    },
    {
      symbol: 'SOL-PERP',
      name: 'Solana Spot',
      baseAsset: 'SOL',
      quoteAsset: 'USDC',
      price: solData ? solData.price : 0,
      change24h: solData ? solData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: solData ? solData.volume24h : 0,
      active: true
    }
  ];
  
  res.json(markets);
});

/**
 * GET /api/market/debug/prices
 * Debug endpoint to check raw CoinGecko data
 */
dashboardRouter.get('/debug/prices', (req, res) => {
  res.json({
    ethData: ethData ? {
      price: ethData.price,
      volume24h: ethData.volume24h,
      change: ethData.priceChangePercent24h
    } : 'Not loaded',
    solData: solData ? {
      price: solData.price,
      volume24h: solData.volume24h,
      change: solData.priceChangePercent24h
    } : 'Not loaded',
    btcData: btcData ? {
      price: btcData.price,
      volume24h: btcData.volume24h,
      change: btcData.priceChangePercent24h
    } : 'Not loaded',
    ratios: ethData && solData ? {
      ethSol: ethData.price / solData.price,
      btcSol: btcData && solData ? btcData.price / solData.price : 'BTC not loaded'
    } : 'Data not ready'
  });
});

/**
 * GET /api/market/:symbol/orderbook
 * Get order book for a symbol
 */
dashboardRouter.get('/:symbol/orderbook', (req, res) => {
  const { symbol } = req.params;
  
  // Determine base price and spread based on symbol
  let basePrice, spread, priceDecimals;
  
  if (symbol === 'ETHSOL' || symbol === 'ETH-SOL') {
    if (!ethData || !solData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    basePrice = ethData.price / solData.price;
    spread = basePrice * 0.001; // 0.1% spread
    priceDecimals = 4;
  } else if (symbol === 'BTCSOL' || symbol === 'BTC-SOL') {
    if (!btcData || !solData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    basePrice = btcData.price / solData.price;
    spread = basePrice * 0.001;
    priceDecimals = 4;
  } else if (symbol === 'ETH-PERP' || symbol === 'ETHUSDC') {
    if (!ethData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    basePrice = ethData.price;
    spread = basePrice * 0.0005; // 0.05% spread
    priceDecimals = 2;
  } else if (symbol === 'BTC-PERP' || symbol === 'BTCUSDC') {
    if (!btcData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    basePrice = btcData.price;
    spread = basePrice * 0.0005; // 0.05% spread
    priceDecimals = 2;
  } else if (symbol === 'SOL-PERP' || symbol === 'SOLUSDC') {
    if (!solData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    basePrice = solData.price;
    spread = basePrice * 0.0005;
    priceDecimals = 2;
  } else {
    // Default to ETH
    if (!ethData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    basePrice = ethData.price;
    spread = basePrice * 0.0005;
    priceDecimals = 2;
  }
  
  // Generate realistic order book
  const bids: any[] = [];
  const asks: any[] = [];
  
  let bidPrice = basePrice - (spread / 2);
  let askPrice = basePrice + (spread / 2);
  let cumulativeBid = 0;
  let cumulativeAsk = 0;
  
  // Generate 20 bid levels
  for (let i = 0; i < 20; i++) {
    const quantity = parseFloat((Math.random() * 5 + 0.1).toFixed(4));
    cumulativeBid += quantity;
    bids.push({
      price: parseFloat(bidPrice.toFixed(priceDecimals)),
      quantity,
      total: parseFloat(cumulativeBid.toFixed(4))
    });
    bidPrice -= Math.random() * spread;
  }
  
  // Generate 20 ask levels
  for (let i = 0; i < 20; i++) {
    const quantity = parseFloat((Math.random() * 5 + 0.1).toFixed(4));
    cumulativeAsk += quantity;
    asks.push({
      price: parseFloat(askPrice.toFixed(priceDecimals)),
      quantity,
      total: parseFloat(cumulativeAsk.toFixed(4))
    });
    askPrice += Math.random() * spread;
  }
  
  // Calculate mid price from orderbook
  const bestBid = bids.length > 0 ? bids[0].price : basePrice;
  const bestAsk = asks.length > 0 ? asks[0].price : basePrice;
  
  // Use basePrice directly for more accurate pricing (matches CoinGecko exactly)
  // The orderbook spread is just for display, actual trades use basePrice
  const midPrice = basePrice;

  res.json({
    symbol,
    bids,
    asks,
    midPrice: parseFloat(midPrice.toFixed(priceDecimals)),
    spread: parseFloat((bestAsk - bestBid).toFixed(priceDecimals)),
    basePrice: parseFloat(basePrice.toFixed(priceDecimals)), // Raw CoinGecko price
    lastUpdate: Date.now()
  });
});


dashboardRouter.get('/:symbol/candles', async (req, res) => {
  const { symbol } = req.params;
  const { timeframe = '1d', limit = '10000', from, to } = req.query;
  
  const limitNum = Math.min(parseInt(limit as string), 500);
  
  // Parse from and to parameters (expecting Unix timestamps in milliseconds)
  let fromTime: number | undefined;
  let toTime: number | undefined;
  
  if (from) {
    fromTime = parseInt(from as string);
    if (isNaN(fromTime)) {
      return res.status(400).json({ error: 'Invalid from parameter. Expected Unix timestamp in milliseconds.' });
    }
  }
  
  if (to) {
    toTime = parseInt(to as string);
    if (isNaN(toTime)) {
      return res.status(400).json({ error: 'Invalid to parameter. Expected Unix timestamp in milliseconds.' });
    }
  }
  
  // Validate that from is before to if both are provided
  if (fromTime !== undefined && toTime !== undefined && fromTime >= toTime) {
    return res.status(400).json({ error: 'from parameter must be before to parameter.' });
  }
  
  try {
    // Map symbol to Hyperliquid coin
    const coin = mapSymbolToHyperliquidCoin(symbol);
    const interval = mapTimeframeToHyperliquidInterval(timeframe as string);
    
    console.log(`Fetching candles for ${symbol} -> ${coin} ${interval} (limit: ${limitNum}${fromTime ? `, from: ${fromTime}` : ''}${toTime ? `, to: ${toTime}` : ''})`);
    
    // Fetch real candlestick data from Hyperliquid
    const candles = await fetchHyperliquidCandles(coin, interval, limitNum, fromTime, toTime);
    
    // For ratio pairs (ETHSOL, BTCSOL), we need to calculate ratios
    if (symbol === 'ETHSOL' || symbol === 'ETH-SOL') {
      if (!ethData || !solData) {
        return res.status(503).json({ error: 'Market data not yet available for ratio calculation' });
      }
      
      // For now, return ETH candles with a note that ratios need separate handling
      // TODO: Implement proper ratio calculation using both ETH and SOL candles
      console.log('Note: ETH/SOL ratio candles need separate ETH and SOL data for proper calculation');
    } else if (symbol === 'BTCSOL' || symbol === 'BTC-SOL') {
      if (!btcData || !solData) {
        return res.status(503).json({ error: 'Market data not yet available for ratio calculation' });
      }
      
      // For now, return BTC candles with a note that ratios need separate handling
      console.log('Note: BTC/SOL ratio candles need separate BTC and SOL data for proper calculation');
    }
    
    res.json(candles);
    
  } catch (error) {
    console.error(`Failed to fetch candles for ${symbol}:`, error);
    
    // Fallback to generated data if API fails
    console.log(`Falling back to generated data for ${symbol}`);
    
    const candles: any[] = [];
    
    // Determine current price based on symbol for fallback
    let currentPrice = 2650; // Default
    let volatilityPercent = 0.02; // 2% default volatility
    
    if (symbol === 'ETHSOL' || symbol === 'ETH-SOL') {
      if (ethData && solData) {
        currentPrice = ethData.price / solData.price;
        volatilityPercent = 0.03;
      }
    } else if (symbol === 'BTCSOL' || symbol === 'BTC-SOL') {
      if (btcData && solData) {
        currentPrice = btcData.price / solData.price;
        volatilityPercent = 0.03;
      }
    } else if (symbol === 'BTC-PERP' || symbol === 'BTCUSDC') {
      if (btcData) {
        currentPrice = btcData.price;
        volatilityPercent = 0.025;
      }
    } else if (symbol === 'SOL-PERP' || symbol === 'SOLUSDC') {
      if (solData) {
        currentPrice = solData.price;
        volatilityPercent = 0.035;
      }
    } else {
      if (ethData) {
        currentPrice = ethData.price;
        volatilityPercent = 0.025;
      }
    }
    
    // Generate fallback candlestick data
    const now = Math.floor(Date.now() / 1000);
    const intervalSeconds = parseInt(timeframe as string) * 60; // Convert to seconds
    
    let price = currentPrice * 0.98; // Start slightly lower
    
    for (let i = limitNum - 1; i >= 0; i--) {
      const time = now - (i * intervalSeconds);
      
      // Random price movement with trend toward current price
      const open = price;
      const volatility = currentPrice * volatilityPercent;
      const trend = (currentPrice - price) * 0.1; // Trend toward current price
      const high = open + Math.random() * volatility + Math.max(0, trend);
      const low = open - Math.random() * volatility + Math.min(0, trend);
      const close = low + Math.random() * (high - low) + trend;
      const volume = (Math.random() * 0.5 + 0.5) * (ethData?.volume24h || 1000000) / 288; // Daily volume / 288 (5min candles per day)
      
      candles.push({
        time,
        open: parseFloat(open.toFixed(2)),
        high: parseFloat(high.toFixed(2)),
        low: parseFloat(low.toFixed(2)),
        close: parseFloat(close.toFixed(2)),
        volume: parseFloat(volume.toFixed(2))
      });
      
      price = close; // Next candle starts where this one ended
    }
    
    res.json(candles);
  }
});

/**
 * GET /api/dashboard/:coin (ethereum, bitcoin, solana)
 * Returns CoinGecko format for frontend chart
 */
dashboardRouter.get('/ethereum', (req, res) => {
  if (!ethData) {
    return res.status(503).json({ error: 'ETH market data not yet available' });
  }
  res.json({
    current_price: ethData.price,
    price_change_24h: ethData.priceChange24h,
    price_change_percentage_24h: ethData.priceChangePercent24h,
    total_volume: ethData.volume24h,
    market_cap: ethData.marketCap,
    high_24h: ethData.high24h,
    low_24h: ethData.low24h,
    circulating_supply: ethData.circulatingSupply,
    ath: ethData.ath,
    atl: ethData.atl
  });
});

dashboardRouter.get('/bitcoin', (req, res) => {
  if (!btcData) {
    return res.status(503).json({ error: 'BTC market data not yet available' });
  }
  res.json({
    current_price: btcData.price,
    price_change_24h: btcData.priceChange24h,
    price_change_percentage_24h: btcData.priceChangePercent24h,
    total_volume: btcData.volume24h,
    market_cap: btcData.marketCap,
    high_24h: btcData.high24h,
    low_24h: btcData.low24h,
    circulating_supply: btcData.circulatingSupply,
    ath: btcData.ath,
    atl: btcData.atl
  });
});

dashboardRouter.get('/solana', (req, res) => {
  if (!solData) {
    return res.status(503).json({ error: 'SOL market data not yet available' });
  }
  res.json({
    current_price: solData.price,
    price_change_24h: solData.priceChange24h,
    price_change_percentage_24h: solData.priceChangePercent24h,
    total_volume: solData.volume24h,
    market_cap: solData.marketCap,
    high_24h: solData.high24h,
    low_24h: solData.low24h,
    circulating_supply: solData.circulatingSupply,
    ath: solData.ath,
    atl: solData.atl
  });
});

/**
 * GET /api/market/:symbol
 * Get market data for a specific symbol
 * NOTE: This route MUST come last because it's a wildcard
 */
dashboardRouter.get('/:symbol', (req, res) => {
  const { symbol } = req.params;
  
  let price, changePercent, volume24h, high24h, low24h, marketCap, openInterest;
  
  // Calculate based on symbol
  if (symbol === 'ETHSOL' || symbol === 'ETH-SOL') {
    // ETH/SOL ratio
    if (!ethData || !solData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    price = ethData.price / solData.price;
    changePercent = ethData.priceChangePercent24h - solData.priceChangePercent24h;
    volume24h = ethData.volume24h; // Use ETH volume in USD
    high24h = ethData.high24h / solData.low24h; // Best case ratio
    low24h = ethData.low24h / solData.high24h; // Worst case ratio
    marketCap = ethData.marketCap;
    openInterest = ethData.volume24h * 0.15; // Estimate
  } else if (symbol === 'BTCSOL' || symbol === 'BTC-SOL') {
    // BTC/SOL ratio
    if (!btcData || !solData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    price = btcData.price / solData.price;
    changePercent = btcData.priceChangePercent24h - solData.priceChangePercent24h;
    volume24h = btcData.volume24h; // Use BTC volume in USD
    high24h = btcData.high24h / solData.low24h;
    low24h = btcData.low24h / solData.high24h;
    marketCap = btcData.marketCap;
    openInterest = btcData.volume24h * 0.15; // Estimate
  } else if (symbol === 'ETH-PERP' || symbol === 'ETHUSDC') {
    // ETH/USD
    if (!ethData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    price = ethData.price;
    changePercent = ethData.priceChangePercent24h;
    volume24h = ethData.volume24h;
    high24h = ethData.high24h;
    low24h = ethData.low24h;
    marketCap = ethData.marketCap;
    openInterest = ethData.volume24h * 0.15; // Estimate
  } else if (symbol === 'BTC-PERP' || symbol === 'BTCUSDC') {
    // BTC/USD
    if (!btcData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    price = btcData.price;
    changePercent = btcData.priceChangePercent24h;
    volume24h = btcData.volume24h;
    high24h = btcData.high24h;
    low24h = btcData.low24h;
    marketCap = btcData.marketCap;
    openInterest = btcData.volume24h * 0.15; // Estimate
  } else if (symbol === 'SOL-PERP' || symbol === 'SOLUSDC') {
    // SOL/USD
    if (!solData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    price = solData.price;
    changePercent = solData.priceChangePercent24h;
    volume24h = solData.volume24h;
    high24h = solData.high24h;
    low24h = solData.low24h;
    marketCap = solData.marketCap;
    openInterest = solData.volume24h * 0.15; // Estimate
  } else if (symbol === 'list') {
    // Redirect to list endpoint
    return res.redirect('/api/market/list');
  } else if (symbol === 'debug') {
    // Redirect to debug
    return res.redirect('/api/market/debug/prices');
  } else {
    // Default to ETH
    if (!ethData) {
      return res.status(503).json({ error: 'Market data not yet available' });
    }
    price = ethData.price;
    changePercent = ethData.priceChangePercent24h;
    volume24h = ethData.volume24h;
    high24h = ethData.high24h;
    low24h = ethData.low24h;
    marketCap = ethData.marketCap;
    openInterest = ethData.volume24h * 0.15; // Estimate
  }
  
  res.json({
    symbol,
    price: parseFloat(price.toFixed(4)),
    change24h: parseFloat(changePercent.toFixed(2)),
    volume24h: parseFloat(volume24h.toFixed(2)),
    high24h: parseFloat(high24h.toFixed(4)),
    low24h: parseFloat(low24h.toFixed(4)),
    fundingRate: 0.0001, // 0.01% for spot (minimal)
    openInterest: parseFloat(openInterest.toFixed(2)),
    indexPrice: parseFloat(price.toFixed(4)),
    markPrice: parseFloat(price.toFixed(4)),
    marketCap: marketCap ? parseFloat(marketCap.toFixed(2)) : 0,
  });
});
