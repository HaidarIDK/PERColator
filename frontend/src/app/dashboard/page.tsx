"use client"

import { Particles } from "@/components/ui/particles"
import { AuroraText } from "@/components/ui/aurora-text"
import { motion } from "motion/react"
import { cn } from "@/lib/utils"
import { 
  TrendingUp, 
  TrendingDown, 
  BarChart3, 
  PieChart, 
  ArrowUpRight, 
  ArrowDownRight,
  ChevronDown,
  Settings,
  Wallet,
  Clock,
  DollarSign,
  Activity,
  Users,
  Zap,
  Shield,
  Target,
  Eye,
  EyeOff
} from "lucide-react"
import { useState, useEffect } from "react"
import { CustomChart, TimeframeSelector } from "@/components/ui/custom-chart"
import { CustomDataService } from "@/lib/data-service"

// TradingView Chart Component
const TradingViewChartComponent = () => {
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [selectedTimeframe, setSelectedTimeframe] = useState<"15" | "1" | "3" | "5" | "30" | "60" | "240" | "120" | "180" | "D" | "W">("15")
  const [chartData, setChartData] = useState<any[]>([])

  // Load sample data on component mount
  useEffect(() => {
    const sampleData = CustomDataService.generateSampleData(50)
    setChartData(sampleData)
  }, [])

  const handleTimeframeChange = (timeframe: "15" | "1" | "3" | "5" | "30" | "60" | "240" | "120" | "180" | "D" | "W") => {
    setSelectedTimeframe(timeframe)
  }

  const toggleFullscreen = () => {
    setIsFullscreen(!isFullscreen)
  }

  return (
    <div className={cn(
      "bg-black/20 rounded-2xl border border-[#181825] overflow-hidden transition-all duration-300",
      isFullscreen 
        ? "fixed inset-0 z-50 rounded-none" 
        : "w-full h-full"
    )}>
      <div className="h-12 flex items-center justify-between px-4 border-b border-[#181825]">
        <div className="flex items-center space-x-4">
          <span className="text-white font-semibold">PERP_ETH_USDC</span>
          <div className="flex items-center space-x-2 text-sm text-gray-400">
            <span>O3975.86</span>
            <span>H3979.36</span>
            <span>L3963.66</span>
            <span>C3967.17</span>
            <span className="text-red-400">-8.02 (-0.20%)</span>
          </div>
        </div>
        <TimeframeSelector 
          selectedTimeframe={selectedTimeframe} 
          onTimeframeChange={handleTimeframeChange} 
        />
        <div className="flex items-center space-x-2">
          <button
            onClick={toggleFullscreen}
            className="ml-4 p-2 text-gray-400 hover:text-white transition-colors"
            title={isFullscreen ? "Exit Fullscreen" : "Enter Fullscreen"}
          >
            {isFullscreen ? (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            ) : (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
              </svg>
            )}
          </button>
        </div>
      </div>
      
      {/* TradingView Chart Widget */}
      <div className={cn(
        "transition-all duration-300",
        isFullscreen ? "h-[calc(100vh-120px)]" : "h-[calc(100%-120px)]"
      )}>
        
        <CustomChart 
          data={chartData}
          symbol="CUSTOM_DATA"
          selectedTimeframe={selectedTimeframe}
          onTimeframeChange={handleTimeframeChange}
          height={isFullscreen ? 600 : 400}
        />
      </div>
      
      <div className="h-16 flex items-center justify-between px-4 border-t border-[#181825]">
        <div className="flex items-center space-x-2">
          {["5y", "1y", "6m", "3m", "1m", "5d", "1d"].map((range) => (
            <button
              key={range}
              className="px-2 py-1 text-xs text-gray-400 hover:text-white transition-colors"
            >
              {range}
            </button>
          ))}
          <Clock className="w-4 h-4 text-gray-400" />
        </div>
        <div className="flex items-center space-x-4 text-sm text-gray-400">
          <span>21:14:45 (UTC+2)</span>
          <div className="flex items-center space-x-2">
            <button className="px-2 py-1 text-xs bg-[#B8B8FF]/10 text-[#B8B8FF] rounded">%</button>
            <button className="px-2 py-1 text-xs text-gray-400 hover:text-white">log</button>
            <button className="px-2 py-1 text-xs text-gray-400 hover:text-white">auto</button>
          </div>
        </div>
      </div>
    </div>
  )
}

// Order Book Component
const OrderBook = () => {
  const asks = [
    { price: 3968.4, qty: 12.5, total: 12.5 },
    { price: 3968.2, qty: 8.3, total: 20.8 },
    { price: 3967.8, qty: 15.2, total: 36.0 },
    { price: 3967.5, qty: 6.7, total: 42.7 },
    { price: 3967.2, qty: 9.1, total: 51.8 },
  ]

  const bids = [
    { price: 3967.1, qty: 18.3, total: 18.3 },
    { price: 3966.8, qty: 22.1, total: 40.4 },
    { price: 3966.5, qty: 14.7, total: 55.1 },
    { price: 3966.2, qty: 11.9, total: 67.0 },
    { price: 3966.0, qty: 16.4, total: 83.4 },
  ]

  return (
    <div className="w-full h-full bg-black/20 rounded-2xl border border-[#181825] overflow-hidden">
      <div className="h-12 flex items-center px-4 border-b border-[#181825]">
        <div className="flex space-x-4">
          <button className="text-[#B8B8FF] font-medium">Order book</button>
          <button className="text-gray-400 hover:text-white transition-colors">Last trades</button>
        </div>
      </div>
      
      <div className="p-4">
        <div className="grid grid-cols-3 gap-2 text-xs text-gray-400 mb-2">
          <span>Price (USDC)</span>
          <span>Qty (ETH)</span>
          <span>Total (ETH)</span>
        </div>
        
        {/* Asks */}
        <div className="space-y-1 mb-4">
          {asks.map((ask, index) => (
            <div key={index} className="grid grid-cols-3 gap-2 text-sm">
              <span className="text-red-400">{ask.price}</span>
              <span className="text-white">{ask.qty}</span>
              <span className="text-gray-400">{ask.total}</span>
            </div>
          ))}
        </div>
        
        {/* Mark Price */}
        <div className="flex items-center justify-center py-2 border-y border-[#181825] my-2">
          <span className="text-white font-semibold">3,967.16</span>
          <span className="text-gray-400 text-xs ml-2">0.0026%</span>
        </div>
        
        {/* Bids */}
        <div className="space-y-1">
          {bids.map((bid, index) => (
            <div key={index} className="grid grid-cols-3 gap-2 text-sm">
              <span className="text-green-400">{bid.price}</span>
              <span className="text-white">{bid.qty}</span>
              <span className="text-gray-400">{bid.total}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

// Order Form Component
const OrderForm = () => {
  const [orderType, setOrderType] = useState("Limit")
  const [side, setSide] = useState("Buy")
  const [leverage, setLeverage] = useState("100X")
  const [price, setPrice] = useState("3967.16")
  const [quantity, setQuantity] = useState("0.1")
  const [percentage, setPercentage] = useState(0)

  return (
    <div className="w-full h-full bg-black/20 rounded-2xl border border-[#181825] overflow-hidden">
      <div className="h-10 flex items-center px-3 border-b border-[#181825]">
        <h3 className="text-white font-medium text-sm">Place Order</h3>
      </div>
      
      <div className="p-3 space-y-3">
        {/* Buy/Sell Tabs */}
        <div className="flex bg-[#181825] rounded-lg p-0.5">
          <button
            onClick={() => setSide("Buy")}
            className={cn(
              "flex-1 py-1.5 px-3 rounded-md text-xs font-medium transition-colors",
              side === "Buy" 
                ? "bg-green-500/20 text-green-400" 
                : "text-gray-400 hover:text-white"
            )}
          >
            Buy
          </button>
          <button
            onClick={() => setSide("Sell")}
            className={cn(
              "flex-1 py-1.5 px-3 rounded-md text-xs font-medium transition-colors",
              side === "Sell" 
                ? "bg-red-500/20 text-red-400" 
                : "text-gray-400 hover:text-white"
            )}
          >
            Sell
          </button>
        </div>

        {/* Order Type & Leverage */}
        <div className="flex space-x-2">
          <select
            value={orderType}
            onChange={(e) => setOrderType(e.target.value)}
            className="flex-1 bg-[#181825] border border-[#181825] rounded-lg px-2 py-1.5 text-white text-xs focus:border-[#B8B8FF]/50 focus:outline-none"
          >
            <option value="Limit">Limit</option>
            <option value="Market">Market</option>
            <option value="Stop">Stop</option>
          </select>
          <select
            value={leverage}
            onChange={(e) => setLeverage(e.target.value)}
            className="flex-1 bg-[#181825] border border-[#181825] rounded-lg px-2 py-1.5 text-white text-xs focus:border-[#B8B8FF]/50 focus:outline-none"
          >
            <option value="1X">1X</option>
            <option value="10X">10X</option>
            <option value="50X">50X</option>
            <option value="100X">100X</option>
          </select>
        </div>

        {/* Available Balance */}
        <div className="text-xs text-gray-400">
          Available: <span className="text-white">1,247.50 USDC</span>
        </div>

        {/* Price Input */}
        <div>
          <label className="block text-xs text-gray-400 mb-1">Price</label>
          <div className="flex space-x-1">
            <input
              type="number"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              className="flex-1 bg-[#181825] border border-[#181825] rounded-lg px-2 py-1.5 text-white text-xs focus:border-[#B8B8FF]/50 focus:outline-none"
              placeholder="0 USDC"
            />
            <button className="px-2 py-1.5 bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-lg text-[#B8B8FF] text-xs hover:bg-[#B8B8FF]/20 transition-colors">
              BBO
            </button>
            <button className="px-2 py-1.5 bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-lg text-[#B8B8FF] text-xs hover:bg-[#B8B8FF]/20 transition-colors">
              Mid
            </button>
          </div>
        </div>

        {/* Quantity Input */}
        <div>
          <label className="block text-xs text-gray-400 mb-1">Qty</label>
          <input
            type="number"
            value={quantity}
            onChange={(e) => setQuantity(e.target.value)}
            className="w-full bg-[#181825] border border-[#181825] rounded-lg px-2 py-1.5 text-white text-xs focus:border-[#B8B8FF]/50 focus:outline-none"
            placeholder="0 ETH"
          />
          <div className="text-xs text-gray-400 mt-1">
            Order size≈ <span className="text-white">396.72 USDC</span>
          </div>
        </div>

        {/* Percentage Slider */}
        <div>
          <label className="block text-xs text-gray-400 mb-1">Max {side.toLowerCase()}</label>
          <div className="flex items-center space-x-2">
            <input
              type="range"
              min="0"
              max="100"
              value={percentage}
              onChange={(e) => setPercentage(Number(e.target.value))}
              className="flex-1"
            />
            <span className="text-xs text-white w-8">{percentage}%</span>
          </div>
        </div>

        {/* Submit Button */}
        <button
          className={cn(
            "w-full py-2 rounded-lg font-medium text-sm transition-colors",
            side === "Buy" 
              ? "bg-green-500/20 border border-green-500/30 text-green-400 hover:bg-green-500/30" 
              : "bg-red-500/20 border border-red-500/30 text-red-400 hover:bg-red-500/30"
          )}
        >
          {side} / {side === "Buy" ? "Long" : "Short"}
        </button>

        {/* Order Details */}
        <div className="space-y-1 text-xs">
          <div className="flex justify-between text-gray-400">
            <span>Est. liq. price:</span>
            <span className="text-white">- USDC</span>
          </div>
          <div className="flex justify-between text-gray-400">
            <span>Fees:</span>
            <span className="text-white">Taker: 0.05% / Maker: 0.02%</span>
          </div>
        </div>

        {/* Advanced Options */}
        <div className="space-y-2 pt-3 border-t border-[#181825]">
          <div className="flex items-center justify-between">
            <span className="text-xs text-gray-400">TP/SL</span>
            <button className="w-10 h-5 bg-[#181825] rounded-full relative">
              <div className="w-4 h-4 bg-[#B8B8FF] rounded-full absolute top-0.5 left-0.5 transition-transform"></div>
            </button>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-gray-400">Reduce only</span>
            <button className="w-10 h-5 bg-[#181825] rounded-full relative">
              <div className="w-4 h-4 bg-gray-400 rounded-full absolute top-0.5 left-0.5 transition-transform"></div>
            </button>
          </div>
          
          <div className="space-y-1">
            <div className="flex space-x-1">
              <button className="flex-1 py-1 px-2 bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded text-[#B8B8FF] text-xs">Post only</button>
              <button className="flex-1 py-1 px-2 bg-[#181825] border border-[#181825] rounded text-gray-400 text-xs">IOC</button>
              <button className="flex-1 py-1 px-2 bg-[#181825] border border-[#181825] rounded text-gray-400 text-xs">FOK</button>
            </div>
          </div>
          
          <div className="space-y-1">
            <div className="flex items-center space-x-2">
              <input type="checkbox" defaultChecked className="w-3 h-3 text-[#B8B8FF] bg-[#181825] border-[#181825] rounded focus:ring-[#B8B8FF]/50" />
              <span className="text-xs text-gray-400">Order confirm</span>
            </div>
            <div className="flex items-center space-x-2">
              <input type="checkbox" className="w-3 h-3 text-[#B8B8FF] bg-[#181825] border-[#181825] rounded focus:ring-[#B8B8FF]/50" />
              <span className="text-xs text-gray-400">Hidden</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// Market Header Component
const MarketHeader = () => {
  return (
    <div className="w-full bg-black/20 backdrop-blur-md border border-[#181825] rounded-2xl p-3">
      <div className="flex items-center justify-between">
        {/* Left - Risk Rate */}
        <div className="flex items-center space-x-3">
          <div className="text-xs text-gray-400">Risk rate</div>
          <div className="w-24 h-1.5 bg-[#181825] rounded-full">
            <div className="w-1/3 h-full bg-[#B8B8FF] rounded-full"></div>
          </div>
        </div>

        {/* Center - Market Info */}
        <div className="flex items-center space-x-6">
          <div className="text-center">
            <div className="text-sm font-bold text-white">★ ETH-PERP</div>
            <div className="text-lg font-bold text-white">$3,966.93</div>
          </div>
          
          <div className="flex items-center space-x-4 text-xs">
            <div>
              <div className="text-gray-400">24h change</div>
              <div className="text-red-400">-24.07/-0.60%</div>
            </div>
            <div>
              <div className="text-gray-400">Mark</div>
              <div className="text-white">3,967.16</div>
            </div>
            <div>
              <div className="text-gray-400">Index</div>
              <div className="text-white">3,967.73</div>
            </div>
            <div>
              <div className="text-gray-400">24h volume</div>
              <div className="text-white">283.61M</div>
            </div>
            <div>
              <div className="text-gray-400">Pred. funding rate</div>
              <div className="text-white">0.0100% in 04:45:14</div>
            </div>
            <div>
              <div className="text-gray-400">Open interest</div>
              <div className="text-white">8.46M USDC</div>
            </div>
          </div>
        </div>

        {/* Right - Layout Button */}
        <button className="px-3 py-1.5 bg-[#B8B8FF]/10 border border-[#B8B8FF]/30 rounded-lg text-[#B8B8FF] text-xs hover:bg-[#B8B8FF]/20 transition-colors">
          Layout
        </button>
      </div>
    </div>
  )
}

// Bottom Assets Bar
const AssetsBar = () => {
  const assets = [
    { symbol: "144.1", price: "+1.14%", positive: true },
    { symbol: "ZEN", price: "12.810 +13.16%", positive: true },
    { symbol: "USELESS", price: "0.32951 +3.89%", positive: true },
    { symbol: "S", price: "0.1738 -2.79%", positive: false },
    { symbol: "ZEC", price: "280.92 +20.58%", positive: true },
    { symbol: "ASTER", price: "1.1430 -6.62%", positive: false },
    { symbol: "AAVE", price: "225.86 +0.54%", positive: true },
    { symbol: "SPX", price: "1.0177 -1.59%", positive: false },
    { symbol: "IP", price: "5.4900 -0.16%", positive: false },
    { symbol: "XPL", price: "0.3833 -9.59%", positive: false },
    { symbol: "TRUMP", price: "5.932 -C", positive: false },
  ]

  return (
    <div className="w-full bg-black/20 backdrop-blur-md border border-[#181825] rounded-2xl p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-6">
          <div className="flex items-center space-x-4 text-sm">
            <span className="text-green-400">Operational</span>
            <span className="text-gray-400">Join our community</span>
            <span className="text-gray-400">Charts powered by TradingView</span>
          </div>
        </div>

        <div className="flex items-center space-x-4 overflow-x-auto">
          {assets.map((asset, index) => (
            <div key={index} className="flex items-center space-x-2 whitespace-nowrap">
              <span className="text-white text-sm">{asset.symbol}</span>
              <span className={cn(
                "text-sm",
                asset.positive ? "text-green-400" : "text-red-400"
              )}>
                {asset.price}
              </span>
            </div>
          ))}
        </div>

        <div className="flex items-center space-x-2">
          <input type="checkbox" className="w-4 h-4 text-[#B8B8FF] bg-[#181825] border-[#181825] rounded focus:ring-[#B8B8FF]/50" />
          <span className="text-sm text-gray-400">Hide other symbols</span>
        </div>
      </div>
    </div>
  )
}

export default function TradingDashboard() {
  return (
    <div className="relative min-h-screen bg-black overflow-hidden"> 
      <Particles
        className="absolute inset-0 z-10"
        quantity={30}
        color="#B8B8FF"
        size={0.6}
        staticity={30}
        ease={80}
      />
      
      <main className="relative z-10 p-4 space-y-4">
        {/* Market Header */}
        <MarketHeader />

        {/* Main Trading Interface */}
        <div className="grid grid-cols-12 gap-4 h-[calc(100vh-200px)]">
          {/* Left Sidebar - Order Form */}
          <div className="col-span-3">
            <OrderForm />
          </div>

          {/* Center - Chart */}
          <div className="col-span-6">
            <TradingViewChartComponent />
          </div>

          {/* Right Sidebar - Order Book */}
          <div className="col-span-3">
            <OrderBook />
          </div>
        </div>

        {/* Bottom Assets Bar */}
        <AssetsBar />
      </main>
    </div>
  )
}