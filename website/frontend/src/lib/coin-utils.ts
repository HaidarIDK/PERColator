// Centralized coin mappings and utilities
// Single source of truth for coin data across the application

export type Coin = 'SOL' | 'ETH' | 'BTC' | 'JUP' | 'BONK' | 'WIF';

export type CoinGeckoId = 'solana' | 'ethereum' | 'bitcoin' | 'jupiter' | 'bonk' | 'dogwifhat';

export interface CoinInfo {
  symbol: Coin;
  name: string;
  geckoId: CoinGeckoId;
  decimals: number;
  color: string;
}

// Master coin configuration
export const COINS: Record<Coin, CoinInfo> = {
  SOL: {
    symbol: 'SOL',
    name: 'Solana',
    geckoId: 'solana',
    decimals: 9,
    color: '#9945FF',
  },
  ETH: {
    symbol: 'ETH',
    name: 'Ethereum',
    geckoId: 'ethereum',
    decimals: 18,
    color: '#627EEA',
  },
  BTC: {
    symbol: 'BTC',
    name: 'Bitcoin',
    geckoId: 'bitcoin',
    decimals: 8,
    color: '#F7931A',
  },
  JUP: {
    symbol: 'JUP',
    name: 'Jupiter',
    geckoId: 'jupiter',
    decimals: 6,
    color: '#00D18C',
  },
  BONK: {
    symbol: 'BONK',
    name: 'Bonk',
    geckoId: 'bonk',
    decimals: 5,
    color: '#FF6B35',
  },
  WIF: {
    symbol: 'WIF',
    name: 'dogwifhat',
    geckoId: 'dogwifhat',
    decimals: 6,
    color: '#A855F7',
  },
};

// Available coins list
export const AVAILABLE_COINS: Coin[] = ['SOL', 'ETH', 'BTC', 'JUP', 'BONK', 'WIF'];

// Convert coin symbol to CoinGecko ID
export function coinToGeckoId(coin: Coin): CoinGeckoId {
  return COINS[coin].geckoId;
}

// Convert coin symbol to display name
export function coinToName(coin: Coin): string {
  return COINS[coin].name;
}

// Get coin color for charts/UI
export function coinToColor(coin: Coin): string {
  return COINS[coin].color;
}

// Get coin decimals
export function coinDecimals(coin: Coin): number {
  return COINS[coin].decimals;
}

// Format trading pair (e.g., SOL -> SOLUSD)
export function coinToTradingPair(coin: Coin): string {
  return `${coin}USD`;
}

// Format price with appropriate decimals
export function formatPrice(price: number, coin?: Coin): string {
  if (price >= 1000) {
    return price.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  } else if (price >= 1) {
    return price.toFixed(2);
  } else if (price >= 0.01) {
    return price.toFixed(4);
  } else {
    return price.toFixed(8);
  }
}

// Format volume with K, M, B suffixes
export function formatVolume(volume: number): string {
  if (volume >= 1e9) {
    return `$${(volume / 1e9).toFixed(2)}B`;
  } else if (volume >= 1e6) {
    return `$${(volume / 1e6).toFixed(2)}M`;
  } else if (volume >= 1e3) {
    return `$${(volume / 1e3).toFixed(2)}K`;
  }
  return `$${volume.toFixed(2)}`;
}

// Format percentage change
export function formatPercentChange(change: number): string {
  const sign = change >= 0 ? '+' : '';
  return `${sign}${change.toFixed(2)}%`;
}

// Check if percentage change is positive
export function isPositiveChange(change: number): boolean {
  return change >= 0;
}
