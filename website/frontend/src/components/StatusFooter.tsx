"use client"

import { useState, useEffect } from "react"
import { motion } from "motion/react"
import { PROGRAM_IDS, formatAddress } from "@/lib/program-config"
import { apiClient } from "@/lib/api-client"

export const StatusFooter = () => {
  const [wsStatus, setWsStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');
  const [apiStatus, setApiStatus] = useState<'operational' | 'degraded' | 'down'>('operational');
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date());

  // WebSocket status monitoring
  useEffect(() => {
    const checkWsStatus = () => {
      const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:5001/ws';
      const ws = new WebSocket(WS_URL);
      
      ws.onopen = () => {
        setWsStatus('connected');
        ws.close();
      };
      
      ws.onerror = () => {
        setWsStatus('disconnected');
      };
      
      ws.onclose = () => {
        if (wsStatus === 'connecting') {
          setWsStatus('disconnected');
        }
      };
    };

    checkWsStatus();
    const interval = setInterval(checkWsStatus, 10000);
    
    return () => clearInterval(interval);
  }, [wsStatus]);

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
      className="mt-4 bg-black/20 backdrop-blur-md border border-[#181825] rounded-xl p-3"
    >
      <div className="flex items-center justify-between">
        {/* Left side - Status indicators */}
        <div className="flex items-center space-x-6">
          {/* WebSocket Status */}
          <div className="flex items-center space-x-2">
            <span className="text-xs text-gray-400">WebSocket:</span>
            <div className="flex items-center space-x-1">
              {getStatusIcon(wsStatus)}
              <span className={`text-xs font-medium ${getStatusColor(wsStatus)}`}>
                {wsStatus}
              </span>
            </div>
          </div>

          {/* API Status */}
          <div className="flex items-center space-x-2">
            <span className="text-xs text-gray-400">API:</span>
            <div className="flex items-center space-x-1">
              {getStatusIcon(apiStatus)}
              <span className={`text-xs font-medium ${getStatusColor(apiStatus)}`}>
                {apiStatus}
              </span>
            </div>
          </div>
        </div>

        {/* Right side - Last update and version */}
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <span className="text-xs text-gray-400">Last update:</span>
            <span className="text-xs text-gray-300" suppressHydrationWarning>
              {lastUpdate.toLocaleTimeString()}
            </span>
          </div>
          
          <div className="flex items-center space-x-2">
            <span className="text-xs text-gray-400">Version:</span>
            <span className="text-xs text-[#B8B8FF] font-medium">v1.0.0</span>
          </div>

          <div className="flex items-center space-x-2">
            <span className="text-xs text-gray-400">Network:</span>
            <span className="text-xs text-amber-400 font-medium">Devnet</span>
          </div>
        </div>
      </div>

      {/* Bottom row - Program IDs */}
      <div className="px-4 py-2 bg-black/10 border-t border-[#181825]/50">
        <div className="flex items-center justify-between text-[10px]">
          <div className="flex items-center space-x-4">
            <span className="text-gray-500 font-medium">Programs:</span>
            <div className="flex items-center space-x-1">
              <span className="text-gray-400">Slab:</span>
              <code className="text-blue-400 font-mono bg-blue-500/5 px-1.5 py-0.5 rounded">{formatAddress(PROGRAM_IDS.slab)}</code>
            </div>
            <div className="flex items-center space-x-1">
              <span className="text-gray-400">Router:</span>
              <code className="text-purple-400 font-mono bg-purple-500/5 px-1.5 py-0.5 rounded">{formatAddress(PROGRAM_IDS.router)}</code>
            </div>
            <div className="flex items-center space-x-1">
              <span className="text-gray-400">AMM:</span>
              <code className="text-green-400 font-mono bg-green-500/5 px-1.5 py-0.5 rounded">{formatAddress(PROGRAM_IDS.amm)}</code>
            </div>
            <div className="flex items-center space-x-1">
              <span className="text-gray-400">Oracle:</span>
              <code className="text-orange-400 font-mono bg-orange-500/5 px-1.5 py-0.5 rounded">{formatAddress(PROGRAM_IDS.oracle)}</code>
            </div>
          </div>
          <span className="text-gray-500">All programs deployed on Devnet</span>
        </div>
      </div>
      
    </motion.div>
  );
};


