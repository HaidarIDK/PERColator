"use client"

import { useEffect, useRef } from "react"
import { createChart, ColorType, IChartApi, ISeriesApi, CandlestickData, CandlestickSeries } from "lightweight-charts"
import { cn } from "@/lib/utils"

interface CustomChartProps {
  data: CandlestickData[]
  symbol?: string
  className?: string
  height?: number
  onTimeframeChange?: (timeframe: "15" | "1" | "3" | "5" | "30" | "60" | "240" | "120" | "180" | "D" | "W") => void
  selectedTimeframe?: string
}

export const CustomChart = ({ 
  data = [], 
  symbol = "CUSTOM_DATA", 
  className,
  height = 400,
  onTimeframeChange,
  selectedTimeframe = "15"
}: CustomChartProps) => {
  const chartContainerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<IChartApi | null>(null)
  const seriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null)

  useEffect(() => {
    if (!chartContainerRef.current) return

      const chart = createChart(chartContainerRef.current, {
        layout: {
          background: { type: ColorType.Solid, color: "rgba(0, 0, 0, 0.9)" },
          textColor: "#ffffff",
        },
        grid: {
          vertLines: { 
            color: "#181825", 
            style: 1,
            visible: true 
          },
          horzLines: { 
            color: "#181825", 
            style: 1,
            visible: true 
          },
        },
        crosshair: {
          mode: 1,
          vertLine: { 
            color: "#B8B8FF", 
            width: 1, 
            style: 2,
            labelBackgroundColor: "#B8B8FF"
          },
          horzLine: { 
            color: "#B8B8FF", 
            width: 1, 
            style: 2,
            labelBackgroundColor: "#B8B8FF"
          },
        },
        rightPriceScale: {
          borderColor: "#181825",
          textColor: "#ffffff",
          scaleMargins: {
            top: 0.1,
            bottom: 0.1,
          },
        },
        timeScale: {
          borderColor: "#181825",
          timeVisible: true,
          secondsVisible: false,
          rightOffset: 12,
          barSpacing: 3,
          minBarSpacing: 0.5,
        },
        width: chartContainerRef.current.clientWidth,
        height,
      })

    // ✅ Enhanced theme integration
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#B8B8FF",           
      downColor: "#ff6b6b",         
      borderUpColor: "#B8B8FF",   
      borderDownColor: "#ff6b6b",   
      wickUpColor: "#B8B8FF",       
      wickDownColor: "#ff6b6b",     
      borderVisible: true,           
      wickVisible: true,            
    })

    candlestickSeries.setData(data)

    chartRef.current = chart
    seriesRef.current = candlestickSeries

    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
        })
      }
    }

    window.addEventListener("resize", handleResize)

    return () => {
      window.removeEventListener("resize", handleResize)
      chart.remove()
    }
  }, [height]) // only depends on height

  useEffect(() => {
    if (seriesRef.current && data) {
      seriesRef.current.setData(data)
    }
  }, [data])

  return (
    <div className={cn("w-full bg-black/20 rounded-lg border border-[#181825]", className)}>
      <div 
        ref={chartContainerRef} 
        className="w-full rounded-lg overflow-hidden" 
        style={{ height: `${height}px` }} 
      />
    </div>
  )
}

// Timeframe selector component
export const TimeframeSelector = ({ 
  selectedTimeframe, 
  onTimeframeChange 
}: { 
  selectedTimeframe: string
  onTimeframeChange: (timeframe: "15" | "1" | "3" | "5" | "30" | "60" | "240" | "120" | "180" | "D" | "W") => void 
}) => {
  const timeframes = [
    { value: "1", label: "1m" },
    { value: "3", label: "3m" },
    { value: "5", label: "5m" },
    { value: "15", label: "15m" },
    { value: "30", label: "30m" },
    { value: "60", label: "1h" },
    { value: "120", label: "2h" },
    { value: "180", label: "3h" },
    { value: "240", label: "4h" },
    { value: "D", label: "1D" },
    { value: "W", label: "1W" },
  ]

  return (
    <div className="flex items-center space-x-1 bg-black/10 rounded-lg p-1 border border-[#181825]">
      {timeframes.map((timeframe) => (
        <button
          key={timeframe.value}
          onClick={() => onTimeframeChange(timeframe.value as "15" | "1" | "3" | "5" | "30" | "60" | "240" | "120" | "180" | "D" | "W")}
          className={cn(
            "px-3 py-1.5 text-xs rounded-md transition-all duration-200 font-medium",
            selectedTimeframe === timeframe.value
              ? "bg-[#B8B8FF] text-black shadow-lg shadow-[#B8B8FF]/20" 
              : "text-gray-400 hover:text-white hover:bg-white/5"
          )}
        >
          {timeframe.label}
        </button>
      ))}
    </div>
  )
}