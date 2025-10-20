// Data service for your custom chart data
export interface CustomCandlestickData {
  time: string | number
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

// Example data service - replace with your actual data source
export class CustomDataService {
  // Example: Fetch data from your API
  static async fetchData(symbol: string, timeframe: string): Promise<CustomCandlestickData[]> {
    // Replace this with your actual API call
    const response = await fetch(`/api/chart-data/${symbol}?timeframe=${timeframe}`)
    const data = await response.json()
    
    // Transform your data to the expected format
    return data.map((item: any) => ({
      time: item.timestamp,
      open: item.open_price,
      high: item.high_price,
      low: item.low_price,
      close: item.close_price,
      volume: item.volume
    }))
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

  // Example: Real-time data updates (WebSocket)
  static subscribeToRealTimeData(
    symbol: string, 
    callback: (data: CustomCandlestickData) => void
  ): () => void {
    // Replace with your actual WebSocket connection
    const ws = new WebSocket(`ws://your-api.com/ws/${symbol}`)
    
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data)
      callback({
        time: data.timestamp,
        open: data.open,
        high: data.high,
        low: data.low,
        close: data.close,
        volume: data.volume
      })
    }

    // Return cleanup function
    return () => ws.close()
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
