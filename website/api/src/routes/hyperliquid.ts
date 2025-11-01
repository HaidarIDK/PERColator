import { Router, Request, Response } from 'express';

export const hyperliquidRouter = Router();

/**
 * GET /api/hyperliquid/candles
 * Fetch historical candlestick data from Hyperliquid
 * 
 * Query params:
 * - coin: BTC, ETH, SOL
 * - interval: 1m, 15m, 1h, 12h
 * - startTime: Unix timestamp (ms) - optional, defaults to 90 days ago
 * - endTime: Unix timestamp (ms) - optional, defaults to now
 */
hyperliquidRouter.get('/candles', async (req: Request, res: Response) => {
  try {
    const { coin, interval, startTime, endTime } = req.query;
    
    if (!coin || !interval) {
      return res.status(400).json({ 
        error: 'Missing required parameters: coin and interval' 
      });
    }

    // Map interval to Hyperliquid format
    const intervalMap: Record<string, string> = {
      '1m': '1m',
      '15m': '15m',
      '1h': '1h',
      '12h': '12h',
    };

    const hyperliquidInterval = intervalMap[interval as string] || '15m';
    
    // Validate coin
    const validCoins = ['BTC', 'ETH', 'SOL'];
    if (!validCoins.includes(coin as string)) {
      return res.status(400).json({ 
        error: `Invalid coin. Must be one of: ${validCoins.join(', ')}` 
      });
    }

    // Calculate time range
    const now = Date.now();
    const defaultStartTime = now - (90 * 24 * 60 * 60 * 1000); // 90 days ago
    
    const start = startTime ? parseInt(startTime as string) : defaultStartTime;
    const end = endTime ? parseInt(endTime as string) : now;

    if (isNaN(start) || isNaN(end)) {
      return res.status(400).json({ 
        error: 'Invalid timestamp format. Use Unix timestamp in milliseconds.' 
      });
    }

    if (start >= end) {
      return res.status(400).json({ 
        error: 'startTime must be before endTime' 
      });
    }

    console.log(`Fetching Hyperliquid candles: ${coin} ${hyperliquidInterval}`);
    console.log(`   Time range: ${new Date(start).toISOString()} → ${new Date(end).toISOString()}`);

    // Fetch from Hyperliquid REST API
    const payload = {
      type: "candleSnapshot",
      req: {
        coin: coin as string,
        interval: hyperliquidInterval,
        startTime: start,
        endTime: end,
      },
    };

    const response = await fetch('https://api.hyperliquid.xyz/info', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error(`Hyperliquid API error: ${response.status} - ${errorText}`);
      return res.status(500).json({ 
        error: `Hyperliquid API error: ${response.status}`,
        details: errorText 
      });
    }

    const data = await response.json();

    if (!Array.isArray(data)) {
      console.error(`Hyperliquid returned non-array:`, data);
      return res.status(500).json({ 
        error: 'Invalid response format from Hyperliquid' 
      });
    }

    // Transform to our format
    const candles = data.map((candle: any) => ({
      time: Math.floor(candle.t / 1000), // Convert ms to seconds for lightweight-charts
      open: parseFloat(candle.o),
      high: parseFloat(candle.h),
      low: parseFloat(candle.l),
      close: parseFloat(candle.c),
      volume: parseFloat(candle.v || 0),
    }));

    console.log(`Returned ${candles.length} candles for ${coin} ${hyperliquidInterval}`);

    res.json({
      success: true,
      coin,
      interval: hyperliquidInterval,
      count: candles.length,
      startTime: start,
      endTime: end,
      candles,
    });

  } catch (error: any) {
    console.error('Error fetching Hyperliquid candles:', error);
    res.status(500).json({ 
      error: 'Failed to fetch candles',
      message: error.message 
    });
  }
});

/**
 * GET /api/hyperliquid/price
 * Get latest price for a coin
 */
hyperliquidRouter.get('/price', async (req: Request, res: Response) => {
  try {
    const { coin } = req.query;
    
    if (!coin) {
      return res.status(400).json({ error: 'coin parameter required' });
    }

    const validCoins = ['BTC', 'ETH', 'SOL'];
    if (!validCoins.includes(coin as string)) {
      return res.status(400).json({ 
        error: `Invalid coin. Must be one of: ${validCoins.join(', ')}` 
      });
    }

    // Fetch latest price from Hyperliquid
    const payload = {
      type: "meta",
    };

    const response = await fetch('https://api.hyperliquid.xyz/info', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      return res.status(500).json({ error: 'Failed to fetch price' });
    }

    const meta = await response.json();
    
    // Find coin in metadata
    const coinInfo = meta?.universe?.find((c: any) => c.name === coin);
    
    if (!coinInfo) {
      return res.status(404).json({ error: `Coin ${coin} not found` });
    }

    // Get latest mark price
    const pricePayload = {
      type: "l2Book",
      coin: coin as string,
    };

    const priceResponse = await fetch('https://api.hyperliquid.xyz/info', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(pricePayload),
    });

    if (!priceResponse.ok) {
      return res.status(500).json({ error: 'Failed to fetch latest price' });
    }

    const priceData = await priceResponse.json();
    
    // Calculate mid price from order book
    const bids = priceData?.levels?.[0]?.[0] || [];
    const asks = priceData?.levels?.[0]?.[1] || [];
    
    const bestBid = bids.length > 0 ? parseFloat(bids[0].px) : 0;
    const bestAsk = asks.length > 0 ? parseFloat(asks[0].px) : 0;
    const midPrice = bestBid && bestAsk ? (bestBid + bestAsk) / 2 : (bestBid || bestAsk || 0);

    res.json({
      coin: coin as string,
      price: midPrice,
      bestBid,
      bestAsk,
      spread: bestBid && bestAsk ? bestAsk - bestBid : 0,
      timestamp: Date.now(),
    });

  } catch (error: any) {
    console.error('Error fetching price:', error);
    res.status(500).json({ 
      error: 'Failed to fetch price',
      message: error.message 
    });
  }
});

