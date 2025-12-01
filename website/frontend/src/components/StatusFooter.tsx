"use client"

import { useState, useEffect } from "react"
import { motion } from "motion/react"
import { PROGRAM_IDS, formatAddress } from "@/lib/program-config"
import { apiClient } from "@/lib/api-client"

export const StatusFooter = () => {
  const [wsStatus, setWsStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');
  const [apiStatus, setApiStatus] = useState<'operational' | 'degraded' | 'down'>('operational');
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());

  // WebSocket status monitoring - disabled to prevent spam
  useEffect(() => {
    // Skip WebSocket health check to avoid connection spam
    // The CLI page has its own WebSocket connection on /ws/cli
    setWsStatus('disconnected');
  }, []);

  // API status monitoring
  useEffect(() => {
    const checkApiStatus = async () => {
      try {
        const response = await fetch(`${apiClient.baseUrl}/api/health`);
        if (response.ok) {
          setApiStatus('operational');
        } else {
          setApiStatus('degraded');
        }
      } catch {
        setApiStatus('down');
      }
    };

    checkApiStatus();
    const interval = setInterval(checkApiStatus, 15000);
    
    return () => clearInterval(interval);
  }, []);

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'connected':
      case 'operational':
        return 'text-green-400';
      case 'connecting':
      case 'degraded':
        return 'text-yellow-400';
      case 'disconnected':
      case 'down':
        return 'text-red-400';
      default:
        return 'text-gray-400';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'connected':
      case 'operational':
        return <div className="w-2 h-2 bg-green-400 rounded-full"></div>;
      case 'connecting':
      case 'degraded':
        return <div className="w-2 h-2 bg-yellow-400 rounded-full"></div>;
      case 'disconnected':
      case 'down':
        return <div className="w-2 h-2 bg-red-400 rounded-full"></div>;
      default:
        return <div className="w-2 h-2 bg-gray-400 rounded-full"></div>;
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="bg-zinc-950 border-t border-white/5 py-1.5 px-4"
    >
      <div className="flex items-center justify-between text-[10px] text-zinc-500">
        {/* Left side - Status indicators */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-1.5">
            <div className={`w-1.5 h-1.5 rounded-full ${
              wsStatus === 'connected' ? 'bg-emerald-500' : 
              wsStatus === 'connecting' ? 'bg-amber-500' : 'bg-red-500'
            }`} />
            <span>WS: <span className={wsStatus === 'connected' ? 'text-emerald-500' : 'text-zinc-400'}>{wsStatus}</span></span>
          </div>

          <div className="flex items-center gap-1.5">
            <div className={`w-1.5 h-1.5 rounded-full ${
              apiStatus === 'operational' ? 'bg-emerald-500' : 
              apiStatus === 'degraded' ? 'bg-amber-500' : 'bg-red-500'
            }`} />
            <span>API: <span className={apiStatus === 'operational' ? 'text-emerald-500' : 'text-zinc-400'}>{apiStatus}</span></span>
          </div>
        </div>

        {/* Center - Program IDs (Compact) */}
        <div className="hidden md:flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <span>Slab:</span>
            <code className="font-mono text-zinc-400 hover:text-blue-400 cursor-help transition-colors" title={PROGRAM_IDS.slab}>{formatAddress(PROGRAM_IDS.slab)}</code>
          </div>
          <div className="w-px h-3 bg-white/10" />
          <div className="flex items-center gap-1.5">
            <span>Router:</span>
            <code className="font-mono text-zinc-400 hover:text-violet-400 cursor-help transition-colors" title={PROGRAM_IDS.router}>{formatAddress(PROGRAM_IDS.router)}</code>
          </div>
          <div className="w-px h-3 bg-white/10" />
          <div className="flex items-center gap-1.5">
            <span>AMM:</span>
            <code className="font-mono text-zinc-400 hover:text-emerald-400 cursor-help transition-colors" title={PROGRAM_IDS.amm}>{formatAddress(PROGRAM_IDS.amm)}</code>
          </div>
        </div>

        {/* Right side - Network & Version */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
             <span className="w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse" />
             <span className="text-blue-400 font-medium">Devnet</span>
          </div>
          <div className="w-px h-3 bg-white/10" />
          <span>v0.1.0</span>
        </div>
      </div>
    </motion.div>
  );
};


