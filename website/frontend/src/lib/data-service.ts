import { apiClient } from './api-client';

// Data service for your custom chart data
export interface CustomCandlestickData {
  time: string | number
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

// Updated data service - now connects to your real backend!
export class CustomDataService {
  // Fetch real data from your API
  static async fetchData(symbol: string, timeframe: string): Promise<CustomCandlestickData[]> {
    try {
      const data = await apiClient.getChartData(symbol, timeframe, 100);
      
      // Transform API data to chart format
      return data.map((item) => ({
        time: item.time,
        open: item.open,
        high: item.high,
        low: item.low,
        close: item.close,
        volume: item.volume
      }));
    } catch (error) {
      console.error('Failed to fetch chart data, using sample data:', error);
      // Fallback to sample data if API fails
      return this.generateSampleData(100);
    }
  }

  // Example: Generate sample data for testing
  static generateSampleData(count: number = 100): CustomCandlestickData[] {
    const data: CustomCandlestickData[] = []
    let price = 100
    const baseTime = new Date('2024-01-01').getTime() // Start from a specific date

    for (let i = 0; i < count; i++) {
      const volatility = Math.random() * 0.1 // 10% max volatility
      const change = (Math.random() - 0.5) * volatility * price
      const open = price
      const close = price + change
      const high = Math.max(open, close) + Math.random() * volatility * price * 0.5
      const low = Math.min(open, close) - Math.random() * volatility * price * 0.5

      data.push({
        time: (baseTime + i * 24 * 60 * 60 * 1000) / 1000, // Convert to seconds timestamp
        open: Number(open.toFixed(2)),
        high: Number(high.toFixed(2)),
        low: Number(low.toFixed(2)),
        close: Number(close.toFixed(2)),
        volume: Math.floor(Math.random() * 1000000) + 100000
      })

      price = close
    }

    return data
  }

  // Real-time data updates (WebSocket) - now connected to your backend!
  static subscribeToRealTimeData(
    symbol: string, 
    callback: (data: CustomCandlestickData) => void
  ): () => void {
    const cleanup = apiClient.connectWebSocket((message) => {
      if (message.type === 'candle' && message.symbol === symbol) {
        callback({
          time: message.data.time,
          open: message.data.open,
          high: message.data.high,
          low: message.data.low,
          close: message.data.close,
          volume: message.data.volume
        });
      }
    });

    // Subscribe to this specific symbol
    apiClient.subscribeToMarket(symbol);

    return cleanup;
  }
}

// Example usage in your component:
/*
import { CustomDataService } from './data-service'

const MyChartComponent = () => {
  const [data, setData] = useState<CustomCandlestickData[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const loadData = async () => {
      try {
        const chartData = await CustomDataService.fetchData('MY_SYMBOL', '15m')
        setData(chartData)
      } catch (error) {
        console.error('Failed to load data:', error)
        // Fallback to sample data
        setData(CustomDataService.generateSampleData(50))
      } finally {
        setLoading(false)
      }
    }

    loadData()
  }, [])

  if (loading) return <div>Loading chart...</div>

  return (
    <CustomChart 
      data={data}
      symbol="MY_SYMBOL"
      selectedTimeframe="15"
      onTimeframeChange={(tf) => {
        // Reload data for new timeframe
        CustomDataService.fetchData('MY_SYMBOL', tf).then(setData)
      }}
    />
  )
}
*/
