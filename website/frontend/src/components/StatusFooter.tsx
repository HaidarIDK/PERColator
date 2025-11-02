"use client"

import { useState, useEffect } from 'react'
import { cn } from '@/lib/utils'
import { Activity, Wifi, WifiOff, Database, Server } from 'lucide-react'

interface StatusIndicatorProps {
  label: string
  status: 'online' | 'offline' | 'degraded'
  icon?: React.ReactNode
}

function StatusIndicator({ label, status, icon }: StatusIndicatorProps) {
  const statusColors = {
    online: 'bg-green-500',
    degraded: 'bg-yellow-500',
    offline: 'bg-red-500'
  }

  const statusText = {
    online: 'Online',
    degraded: 'Degraded',
    offline: 'Offline'
  }

  return (
    <div className="flex items-center gap-2">
      {icon}
      <span className="text-xs text-gray-400">{label}:</span>
      <div className="flex items-center gap-1.5">
        <div className={cn(
          "w-2 h-2 rounded-full",
          statusColors[status],
          status === 'online' && "animate-pulse"
        )}></div>
        <span className={cn(
          "text-xs font-medium",
          status === 'online' && "text-green-400",
          status === 'degraded' && "text-yellow-400",
          status === 'offline' && "text-red-400"
        )}>
          {statusText[status]}
        </span>
      </div>
    </div>
  )
}

export function StatusFooter() {
  const [apiStatus, setApiStatus] = useState<'online' | 'offline' | 'degraded'>('offline')
  const [wsStatus, setWsStatus] = useState<'online' | 'offline' | 'degraded'>('offline')
  const [slabStatus, setSlabStatus] = useState<'online' | 'offline' | 'degraded'>('offline')
  const [oracleStatus, setOracleStatus] = useState<'online' | 'offline' | 'degraded'>('offline')

  useEffect(() => {
    const checkStatus = async () => {
      // Check API
      try {
        const apiResponse = await fetch('http://localhost:5001/api/health', {
          signal: AbortSignal.timeout(3000)
        })
        setApiStatus(apiResponse.ok ? 'online' : 'degraded')
      } catch {
        setApiStatus('offline')
      }

      // Check WebSocket (just assume online if API is online for now)
      setWsStatus(apiStatus === 'online' ? 'online' : 'offline')

      // Check Slab
      try {
        const slabResponse = await fetch('http://localhost:5001/api/slab/orderbook', {
          signal: AbortSignal.timeout(3000)
        })
        const slabData = await slabResponse.json()
        setSlabStatus(slabData.success ? 'online' : 'degraded')
      } catch {
        setSlabStatus('offline')
      }

      // Check Oracle (assume online if API is online)
      setOracleStatus(apiStatus === 'online' ? 'online' : 'offline')
    }

    checkStatus()
    const interval = setInterval(checkStatus, 10000) // Check every 10 seconds

    return () => clearInterval(interval)
  }, [apiStatus])

  return (
    <div className="relative z-10 bg-gradient-to-r from-black/60 via-black/40 to-black/60 backdrop-blur-xl border-t border-[#181825]">
      <div className="max-w-[1600px] mx-auto px-4 py-2 sm:py-3">
        <div className="flex flex-col sm:flex-row items-center justify-between gap-3">
          {/* Left - System Status */}
          <div className="flex items-center gap-4 sm:gap-6 flex-wrap justify-center sm:justify-start">
            <div className="flex items-center gap-2">
              <Activity className="w-4 h-4 text-[#B8B8FF]" />
              <span className="text-sm font-semibold text-[#B8B8FF]">System Status</span>
            </div>
            
            <StatusIndicator 
              label="API" 
              status={apiStatus}
              icon={<Server className="w-3.5 h-3.5 text-gray-500" />}
            />
            
            <StatusIndicator 
              label="WebSocket" 
              status={wsStatus}
              icon={wsStatus === 'online' ? 
                <Wifi className="w-3.5 h-3.5 text-gray-500" /> : 
                <WifiOff className="w-3.5 h-3.5 text-gray-500" />
              }
            />
            
            <StatusIndicator 
              label="Slab" 
              status={slabStatus}
              icon={<Database className="w-3.5 h-3.5 text-gray-500" />}
            />
            
            <StatusIndicator 
              label="Oracle" 
              status={oracleStatus}
              icon={<Activity className="w-3.5 h-3.5 text-gray-500" />}
            />
          </div>

          {/* Right - Info */}
          <div className="flex items-center gap-2 text-xs text-gray-500">
            <span>Percolator v2</span>
            <span>•</span>
            <span>Solana Devnet</span>
          </div>
        </div>
      </div>
    </div>
  )
}

