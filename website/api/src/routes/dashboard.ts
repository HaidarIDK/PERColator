import { Router } from 'express';

export const dashboardRouter = Router();

// Real-time market data from CoinGecko API
// Initialize with fallback data to prevent 503 errors on startup
let ethData: any = { price: 3931.15, priceChangePercent24h: -2.14, volume24h: 34626730527, marketCap: 474592579277, high24h: 4082.02, low24h: 3926.31 };
let solData: any = { price: 186.12, priceChangePercent24h: -2.27, volume24h: 5808295794, marketCap: 101723519950, high24h: 194.13, low24h: 185.96 };
let btcData: any = { price: 98765.43, priceChangePercent24h: 1.50, volume24h: 45000000000, marketCap: 1950000000000, high24h: 99500, low24h: 97000 };
let jupData: any = { price: 1.23, priceChangePercent24h: 5.43, volume24h: 150000000, marketCap: 1230000000, high24h: 1.30, low24h: 1.15 };
let bonkData: any = { price: 0.00002345, priceChangePercent24h: 10.5, volume24h: 50000000, marketCap: 1500000000, high24h: 0.00002500, low24h: 0.00002200 };
let wifData: any = { price: 3.45, priceChangePercent24h: 8.2, volume24h: 80000000, marketCap: 3450000000, high24h: 3.60, low24h: 3.20 };

// Fetch real market data from CoinGecko
async function fetchCoinGeckoData() {
  try {
    console.log('Fetching data from CoinGecko...');
    
    // Check if fetch is available
    if (typeof fetch === 'undefined') {
      console.error('Fetch is not available in this Node.js version');
      return;
    }
    
    // Fetch ETH, BTC, SOL, JUP, BONK, WIF data in one call
    const response = await fetch(
      'https://api.coingecko.com/api/v3/coins/markets?vs_currency=usd&ids=bitcoin,ethereum,solana,jupiter-exchange-solana,bonk,dogwifcoin&order=market_cap_desc&per_page=10&page=1&sparkline=false&price_change_percentage=24h'
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
        const coinData = {
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

        if (coin.id === 'ethereum') {
          ethData = coinData;
          console.log(`ETH: $${ethData.price.toFixed(2)}`);
        } else if (coin.id === 'bitcoin') {
          btcData = coinData;
          console.log(`BTC: $${btcData.price.toFixed(2)}`);
        } else if (coin.id === 'solana') {
          solData = coinData;
          console.log(`SOL: $${solData.price.toFixed(2)}`);
        } else if (coin.id === 'jupiter-exchange-solana') {
          jupData = coinData;
          console.log(`JUP: $${jupData.price.toFixed(4)}`);
        } else if (coin.id === 'bonk') {
          bonkData = coinData;
          console.log(`BONK: $${bonkData.price.toFixed(8)}`);
        } else if (coin.id === 'dogwifcoin') {
          wifData = coinData;
          console.log(`WIF: $${wifData.price.toFixed(4)}`);
        }
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
    'BTCUSD': 'BTC',
    'ETH-PERP': 'ETH',
    'ETHUSDC': 'ETH',
    'ETHUSD': 'ETH',
    'SOL-PERP': 'SOL',
    'SOLUSDC': 'SOL',
    'SOLUSD': 'SOL',
    'ETHSOL': 'ETH',
    'ETH-SOL': 'ETH',
    'BTCSOL': 'BTC',
    'BTC-SOL': 'BTC',
    'JUP-PERP': 'JUP',
    'JUPUSD': 'JUP',
    'JUPUSDC': 'JUP',
    'BONK-PERP': 'BONK',
    'BONKUSD': 'BONK',
    'BONKUSDC': 'BONK',
    'WIF-PERP': 'WIF',
    'WIFUSD': 'WIF',
    'WIFUSDC': 'WIF'
  };
  
  return symbolMap[symbol] || 'BTC'; // Default to BTC
}

/**
 * Map timeframe to Hyperliquid interval format
 */
function mapTimeframeToHyperliquidInterval(timeframe: string): string {
  const intervalMap: { [key: string]: string } = {
    '1m': '1m',
    '5m': '5m',
    '15m': '15m',
    '30m': '30m',
    '1h': '1h',
    '4h': '4h',
    '12h': '4h',
    '1d': '1d',
    '1': '1m',
    '5': '5m',
    '15': '15m',
    '30': '30m',
    '60': '1h',
    '240': '4h',
    '720': '4h',
    '1440': '1d'
  };
  
  const numericTimeframe = timeframe.replace(/[mhd]$/i, '').toLowerCase();
  return intervalMap[timeframe.toLowerCase()] || intervalMap[numericTimeframe] || '15m';
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
      // Map interval strings to minutes
      const intervalMinutesMap: { [key: string]: number } = {
        '1m': 1, '5m': 5, '15m': 15, '30m': 30,
        '1h': 60, '4h': 240, '12h': 720, '1d': 1440
      };
      
      let intervalMinutes = intervalMinutesMap[interval] || 15;
      const intervalMs = intervalMinutes * 60 * 1000;
      startTime = now - (limit * intervalMs);
      endTime = now;
    }
    
    const payload = {
      type: "candleSnapshot",
      req: { coin, interval, startTime, endTime }
    };
    
    console.log(`📊 Fetching Hyperliquid candles for ${coin} ${interval}`);
    
    const response = await fetch('https://api.hyperliquid.xyz/info', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    
    if (!response.ok) {
      throw new Error(`Hyperliquid API error: ${response.status}`);
    }
    
    const data = await response.json() as any[];
    
    if (!Array.isArray(data)) {
      throw new Error(`Hyperliquid returned invalid data format`);
    }
    
    return data.map((candle: any) => {
      let open = typeof candle.o === 'string' ? parseFloat(candle.o) : candle.o;
      let high = typeof candle.h === 'string' ? parseFloat(candle.h) : candle.h;
      let low = typeof candle.l === 'string' ? parseFloat(candle.l) : candle.l;
      let close = typeof candle.c === 'string' ? parseFloat(candle.c) : candle.c;
      const volume = typeof candle.v === 'string' ? parseFloat(candle.v) : candle.v;
      
      // Fix Hyperliquid price scaling issues
      if (coin === 'SOL' && close > 1000) { open/=1000; high/=1000; low/=1000; close/=1000; }
      else if (coin === 'ETH' && close > 100000) { open/=100; high/=100; low/=100; close/=100; }
      else if (coin === 'BTC' && close > 100000000) { open/=1000; high/=1000; low/=1000; close/=1000; }
      
      return {
        time: Math.floor(candle.t / 1000),
        open, high, low, close,
        volume: volume || 0
      };
    });
    
  } catch (error) {
    console.error('❌ Failed to fetch Hyperliquid candles:', error);
    throw error;
  }
}

// ============================================
// ROUTES
// ============================================

dashboardRouter.get('/list', (req, res) => {
  const markets = [
    {
      symbol: 'ETHSOL', name: 'ETH/SOL Ratio', baseAsset: 'ETH', quoteAsset: 'SOL',
      price: ethData && solData ? parseFloat((ethData.price / solData.price).toFixed(4)) : 0,
      change24h: ethData && solData ? (ethData.priceChangePercent24h - solData.priceChangePercent24h).toFixed(2) + '%' : '0.00%',
      volume24h: ethData ? ethData.volume24h : 0, active: true
    },
    {
      symbol: 'BTCSOL', name: 'BTC/SOL Ratio', baseAsset: 'BTC', quoteAsset: 'SOL',
      price: btcData && solData ? parseFloat((btcData.price / solData.price).toFixed(4)) : 0,
      change24h: btcData && solData ? (btcData.priceChangePercent24h - solData.priceChangePercent24h).toFixed(2) + '%' : '0.00%',
      volume24h: btcData ? btcData.volume24h : 0, active: true
    },
    {
      symbol: 'ETH-PERP', name: 'Ethereum Spot', baseAsset: 'ETH', quoteAsset: 'USDC',
      price: ethData ? ethData.price : 0,
      change24h: ethData ? ethData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: ethData ? ethData.volume24h : 0, active: true
    },
    {
      symbol: 'BTC-PERP', name: 'Bitcoin Spot', baseAsset: 'BTC', quoteAsset: 'USDC',
      price: btcData ? btcData.price : 0,
      change24h: btcData ? btcData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: btcData ? btcData.volume24h : 0, active: true
    },
    {
      symbol: 'SOL-PERP', name: 'Solana Spot', baseAsset: 'SOL', quoteAsset: 'USDC',
      price: solData ? solData.price : 0,
      change24h: solData ? solData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: solData ? solData.volume24h : 0, active: true
    },
    {
      symbol: 'JUP-PERP', name: 'Jupiter Spot', baseAsset: 'JUP', quoteAsset: 'USDC',
      price: jupData ? jupData.price : 0,
      change24h: jupData ? jupData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: jupData ? jupData.volume24h : 0, active: true
    },
    {
      symbol: 'BONK-PERP', name: 'Bonk Spot', baseAsset: 'BONK', quoteAsset: 'USDC',
      price: bonkData ? bonkData.price : 0,
      change24h: bonkData ? bonkData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: bonkData ? bonkData.volume24h : 0, active: true
    },
    {
      symbol: 'WIF-PERP', name: 'dogwifhat Spot', baseAsset: 'WIF', quoteAsset: 'USDC',
      price: wifData ? wifData.price : 0,
      change24h: wifData ? wifData.priceChangePercent24h.toFixed(2) + '%' : '0.00%',
      volume24h: wifData ? wifData.volume24h : 0, active: true
    }
  ];
  
  res.json(markets);
});

dashboardRouter.get('/debug/prices', (req, res) => {
  res.json({
    ethData: ethData ? { price: ethData.price, volume: ethData.volume24h } : 'Not loaded',
    solData: solData ? { price: solData.price, volume: solData.volume24h } : 'Not loaded',
    btcData: btcData ? { price: btcData.price, volume: btcData.volume24h } : 'Not loaded',
    jupData: jupData ? { price: jupData.price, volume: jupData.volume24h } : 'Not loaded',
    bonkData: bonkData ? { price: bonkData.price, volume: bonkData.volume24h } : 'Not loaded',
    wifData: wifData ? { price: wifData.price, volume: wifData.volume24h } : 'Not loaded'
  });
});

dashboardRouter.get('/:symbol/orderbook', (req, res) => {
  const { symbol } = req.params;
  
  let basePrice, spread, priceDecimals;
  
  // Define data sources map
  const dataMap: Record<string, any> = {
    'ETH': ethData, 'BTC': btcData, 'SOL': solData, 
    'JUP': jupData, 'BONK': bonkData, 'WIF': wifData
  };
  
  if (symbol.includes('ETHSOL')) {
    if (!ethData || !solData) return res.status(503).json({ error: 'Data not available' });
    basePrice = ethData.price / solData.price;
    spread = basePrice * 0.001; priceDecimals = 4;
  } else if (symbol.includes('BTCSOL')) {
    if (!btcData || !solData) return res.status(503).json({ error: 'Data not available' });
    basePrice = btcData.price / solData.price;
    spread = basePrice * 0.001; priceDecimals = 4;
  } else {
    // Generic handler for PERP/USD pairs
    let coin = 'ETH';
    if (symbol.includes('BTC')) coin = 'BTC';
    else if (symbol.includes('SOL')) coin = 'SOL';
    else if (symbol.includes('JUP')) coin = 'JUP';
    else if (symbol.includes('BONK')) coin = 'BONK';
    else if (symbol.includes('WIF')) coin = 'WIF';
    
    const data = dataMap[coin];
    if (!data) return res.status(503).json({ error: 'Data not available' });
    
    basePrice = data.price;
    spread = basePrice * 0.0005;
    priceDecimals = basePrice < 1 ? 8 : (basePrice < 10 ? 4 : 2);
  }
  
  // Generate realistic order book
  const bids: any[] = [];
  const asks: any[] = [];
  let bidPrice = basePrice - (spread / 2);
  let askPrice = basePrice + (spread / 2);
  let cumulativeBid = 0;
  let cumulativeAsk = 0;
  
  for (let i = 0; i < 20; i++) {
    const quantity = parseFloat((Math.random() * 5 + 0.1).toFixed(4));
    cumulativeBid += quantity;
    bids.push({ price: parseFloat(bidPrice.toFixed(priceDecimals)), quantity, total: parseFloat(cumulativeBid.toFixed(4)) });
    bidPrice -= Math.random() * spread;
    
    const askQty = parseFloat((Math.random() * 5 + 0.1).toFixed(4));
    cumulativeAsk += askQty;
    asks.push({ price: parseFloat(askPrice.toFixed(priceDecimals)), quantity: askQty, total: parseFloat(cumulativeAsk.toFixed(4)) });
    askPrice += Math.random() * spread;
  }

  res.json({
    symbol, bids, asks,
    midPrice: parseFloat(basePrice.toFixed(priceDecimals)),
    spread: parseFloat((asks[0].price - bids[0].price).toFixed(priceDecimals)),
    basePrice: parseFloat(basePrice.toFixed(priceDecimals)),
    lastUpdate: Date.now()
  });
});

dashboardRouter.get('/:symbol/candles', async (req, res) => {
  const { symbol } = req.params;
  const { timeframe = '1d', limit = '10000', from, to } = req.query;
  
  try {
    const coin = mapSymbolToHyperliquidCoin(symbol);
    const interval = mapTimeframeToHyperliquidInterval(timeframe as string);
    const limitNum = Math.min(parseInt(limit as string), 500);
    const fromTime = from ? parseInt(from as string) : undefined;
    const toTime = to ? parseInt(to as string) : undefined;
    
    const candles = await fetchHyperliquidCandles(coin, interval, limitNum, fromTime, toTime);
    res.json(candles);
  } catch (error) {
    console.error(`Failed to fetch candles for ${symbol}:`, error);
    
    // Fallback generator
    const candles: any[] = [];
    let currentPrice = 2650;
    if (symbol.includes('BTC')) currentPrice = btcData?.price || 98000;
    else if (symbol.includes('SOL')) currentPrice = solData?.price || 180;
    else if (symbol.includes('JUP')) currentPrice = jupData?.price || 1.2;
    else if (symbol.includes('BONK')) currentPrice = bonkData?.price || 0.00002;
    else if (symbol.includes('WIF')) currentPrice = wifData?.price || 3.4;
    
    const now = Math.floor(Date.now() / 1000);
    const intervalSeconds = parseInt(timeframe as string) * 60 || 900;
    let price = currentPrice * 0.98;
    
    for (let i = 0; i < 100; i++) {
       const time = now - ((99-i) * intervalSeconds);
       const change = (Math.random() - 0.5) * (currentPrice * 0.01);
       price += change;
       candles.push({
         time, open: price, high: price * 1.001, low: price * 0.999, close: price + (Math.random() - 0.5),
         volume: 100000 + Math.random() * 100000
       });
    }
    res.json(candles);
  }
});

dashboardRouter.get('/:symbol', (req, res) => {
  const { symbol } = req.params;
  
  // Map common names to PERP symbols for easier data lookup
  let lookupSymbol = symbol;
  if (['ethereum', 'ETH'].includes(symbol)) lookupSymbol = 'ETH-PERP';
  if (['bitcoin', 'BTC'].includes(symbol)) lookupSymbol = 'BTC-PERP';
  if (['solana', 'SOL'].includes(symbol)) lookupSymbol = 'SOL-PERP';
  if (['jupiter', 'JUP'].includes(symbol)) lookupSymbol = 'JUP-PERP';
  
  let data, coin = 'ETH';
  
  if (symbol.includes('ETH')) { data = ethData; coin = 'ETH'; }
  else if (symbol.includes('BTC')) { data = btcData; coin = 'BTC'; }
  else if (symbol.includes('SOL')) { data = solData; coin = 'SOL'; }
  else if (symbol.includes('JUP')) { data = jupData; coin = 'JUP'; }
  else if (symbol.includes('BONK')) { data = bonkData; coin = 'BONK'; }
  else if (symbol.includes('WIF')) { data = wifData; coin = 'WIF'; }
  
  if (!data) return res.status(503).json({ error: 'Market data not yet available' });
  
  res.json({
    symbol,
    price: data.price,
    change24h: data.priceChangePercent24h,
    volume24h: data.volume24h,
    high24h: data.high24h,
    low24h: data.low24h,
    marketCap: data.marketCap,
    fundingRate: 0.0001,
    openInterest: data.volume24h * 0.15
  });
});
