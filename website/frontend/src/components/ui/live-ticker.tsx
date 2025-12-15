"use client"

import { useEffect, useState } from "react"
import { motion, AnimatePresence } from "motion/react"
import { TrendingUp, TrendingDown } from "lucide-react"

interface TickerItem {
  symbol: string
  price: number
  change: number
}

const DEMO_TICKERS: TickerItem[] = [
  { symbol: "SOL", price: 185.42, change: 3.24 },
  { symbol: "BTC", price: 104523.18, change: 1.87 },
  { symbol: "ETH", price: 3892.45, change: -0.52 },
  { symbol: "JUP", price: 1.23, change: 5.67 },
  { symbol: "BONK", price: 0.0000342, change: 12.34 },
  { symbol: "WIF", price: 2.87, change: -2.15 },
]

export function LiveTicker() {
  const [tickers, setTickers] = useState<TickerItem[]>(DEMO_TICKERS)
  const [isPaused, setIsPaused] = useState(false)

  // Simulate price updates
  useEffect(() => {
    const interval = setInterval(() => {
      setTickers(prev =>
        prev.map(ticker => ({
          ...ticker,
          price: ticker.price * (1 + (Math.random() - 0.5) * 0.002),
          change: ticker.change + (Math.random() - 0.5) * 0.1,
        }))
      )
    }, 3000)

    return () => clearInterval(interval)
  }, [])

  const formatPrice = (price: number) => {
    if (price < 0.001) return price.toFixed(8)
    if (price < 1) return price.toFixed(4)
    if (price < 100) return price.toFixed(2)
    return price.toLocaleString(undefined, { maximumFractionDigits: 2 })
  }

  return (
    <div
      className="relative overflow-hidden bg-black/30 backdrop-blur-sm border-y border-[#B8B8FF]/10"
      onMouseEnter={() => setIsPaused(true)}
      onMouseLeave={() => setIsPaused(false)}
    >
      <div className="absolute left-0 top-0 bottom-0 w-20 bg-gradient-to-r from-black to-transparent z-10" />
      <div className="absolute right-0 top-0 bottom-0 w-20 bg-gradient-to-l from-black to-transparent z-10" />

      <motion.div
        className="flex items-center gap-8 py-3 px-4"
        animate={{ x: isPaused ? 0 : "-50%" }}
        transition={{
          x: {
            duration: 30,
            repeat: Infinity,
            repeatType: "loop",
            ease: "linear",
          },
        }}
      >
        {[...tickers, ...tickers].map((ticker, index) => (
          <div
            key={`${ticker.symbol}-${index}`}
            className="flex items-center gap-3 shrink-0"
          >
            <span className="text-sm font-bold text-white">{ticker.symbol}</span>
            <span className="text-sm text-gray-300 font-mono">
              ${formatPrice(ticker.price)}
            </span>
            <span
              className={`flex items-center gap-1 text-xs font-medium ${
                ticker.change >= 0 ? "text-emerald-400" : "text-red-400"
              }`}
            >
              {ticker.change >= 0 ? (
                <TrendingUp className="w-3 h-3" />
              ) : (
                <TrendingDown className="w-3 h-3" />
              )}
              {ticker.change >= 0 ? "+" : ""}{ticker.change.toFixed(2)}%
            </span>

            {index < tickers.length * 2 - 1 && (
              <div className="w-[1px] h-4 bg-[#B8B8FF]/20 ml-4" />
            )}
          </div>
        ))}
      </motion.div>
    </div>
  )
}
