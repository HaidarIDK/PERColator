# Custom Chart Data Integration Guide

## 🎯 Overview

You now have a custom chart component that can display your own data instead of relying on TradingView's data feeds. This gives you complete control over your chart data and styling.

## 📊 Data Format

Your data should follow this structure:

```typescript
interface CustomCandlestickData {
  time: string | number    // Timestamp (string like "2024-01-01" or number in seconds)
  open: number             // Opening price
  high: number             // Highest price
  low: number              // Lowest price
  close: number            // Closing price
  volume?: number          // Optional volume data
}
```

## 🔧 How to Use Your Own Data

### Option 1: Replace Sample Data

In `src/app/dashboard/page.tsx`, replace the sample data loading:

```typescript
// Replace this:
const sampleData = CustomDataService.generateSampleData(50)
setChartData(sampleData)

// With your data:
const myData = await fetchMyData()
setChartData(myData)
```

### Option 2: API Integration

Create an API endpoint and use the data service:

```typescript
// In your component:
useEffect(() => {
  const loadData = async () => {
    try {
      const data = await CustomDataService.fetchData('YOUR_SYMBOL', selectedTimeframe)
      setChartData(data)
    } catch (error) {
      console.error('Failed to load data:', error)
    }
  }
  
  loadData()
}, [selectedTimeframe])
```

### Option 3: Real-time Data (WebSocket)

For live data updates:

```typescript
useEffect(() => {
  const unsubscribe = CustomDataService.subscribeToRealTimeData(
    'YOUR_SYMBOL',
    (newData) => {
      setChartData(prev => [...prev, newData])
    }
  )
  
  return unsubscribe // Cleanup on unmount
}, [])
```

## 🎨 Customization Options

### Chart Colors
The chart uses your theme colors:
- **Bullish candles**: `#B8B8FF` (your accent color)
- **Bearish candles**: `#ff6b6b` (red)
- **Background**: `rgba(0, 0, 0, 0.8)`
- **Grid lines**: `#181825` (your border color)
- **Crosshair**: `#B8B8FF` (your accent color)

### Chart Features
- ✅ Candlestick display
- ✅ Volume indicators
- ✅ Timeframe switching
- ✅ Fullscreen mode
- ✅ Crosshair tooltips
- ✅ Responsive design
- ✅ Dark theme integration

## 📝 Example Data Sources

### From Database
```typescript
const fetchFromDatabase = async (symbol: string, timeframe: string) => {
  const response = await fetch(`/api/chart-data?symbol=${symbol}&timeframe=${timeframe}`)
  return response.json()
}
```

### From External API
```typescript
const fetchFromExternalAPI = async (symbol: string) => {
  const response = await fetch(`https://api.example.com/ohlc/${symbol}`)
  const data = await response.json()
  
  // Transform to our format
  return data.map(item => ({
    time: item.timestamp,
    open: item.o,
    high: item.h,
    low: item.l,
    close: item.c,
    volume: item.v
  }))
}
```

### From CSV/File
```typescript
const parseCSVData = (csvContent: string) => {
  const lines = csvContent.split('\n')
  return lines.slice(1).map(line => {
    const [time, open, high, low, close, volume] = line.split(',')
    return {
      time: time,
      open: parseFloat(open),
      high: parseFloat(high),
      low: parseFloat(low),
      close: parseFloat(close),
      volume: parseFloat(volume)
    }
  })
}
```

## 🚀 Performance Tips

1. **Data Pagination**: Load data in chunks for better performance
2. **Caching**: Cache data to avoid repeated API calls
3. **Real-time Updates**: Use WebSockets for live data
4. **Data Compression**: Compress large datasets before transmission

## 🔄 Timeframe Handling

The chart supports these timeframes:
- `1`, `3`, `5`, `15`, `30`, `60`, `120`, `180`, `240` (minutes)
- `D` (daily), `W` (weekly)

When timeframe changes, reload your data accordingly.

## 📱 Mobile Responsiveness

The chart automatically adjusts to different screen sizes and supports:
- Touch gestures for zooming/panning
- Responsive height adjustments
- Mobile-friendly controls

## 🎯 Next Steps

1. **Replace sample data** with your actual data source
2. **Implement API endpoints** for data fetching
3. **Add real-time updates** if needed
4. **Customize styling** further if required
5. **Add additional indicators** (moving averages, etc.)

Your custom chart is now ready to display your own data with full theme integration!
