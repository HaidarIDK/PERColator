'use client';

import { useState, useEffect } from 'react';
import { apiClient } from '@/lib/api-client';

export default function WebSocketTest() {
  const [isConnected, setIsConnected] = useState(false);
  const [messages, setMessages] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedSymbol, setSelectedSymbol] = useState('SOL');
  const [selectedInterval, setSelectedInterval] = useState('1m');
  const [connectionStatus, setConnectionStatus] = useState<string>('Disconnected');

  const symbols = ['SOL', 'ETH', 'BTC'];
  const intervals = ['1m', '5m', '15m', '1h', '4h', '1d'];

  useEffect(() => {
    const connectAndSubscribe = async () => {
      try {
        setError(null);
        setConnectionStatus('Connecting...');
        
        console.log('🧪 Test component: Connecting to server WebSocket...');
        await apiClient.connectServerWebSocket();
        
        setIsConnected(true);
        setConnectionStatus('Connected');
        console.log('🧪 Test component: Connected successfully');

        // Subscribe to candle updates
        console.log(`🧪 Test component: Subscribing to ${selectedSymbol} ${selectedInterval}`);
        apiClient.subscribeToServerCandle(selectedSymbol, selectedInterval);

        // Listen for messages
        const cleanup = apiClient.onServerMessage((data: any) => {
          console.log('🧪 Test component: Received message:', data);
          
          setMessages(prev => {
            const newMessages = [...prev, {
              timestamp: new Date().toLocaleTimeString(),
              type: data.type,
              symbol: data.symbol,
              timeframe: data.timeframe,
              subscriptionId: data.subscriptionId,
              data: data.data,
              serverData: data.serverData,
              rawMessage: data
            }];
            
            // Keep only last 20 messages
            return newMessages.slice(-20);
          });
        });

        return cleanup;
      } catch (err) {
        console.error('🧪 Test component: Connection failed:', err);
        setError(`Connection failed: ${err}`);
        setIsConnected(false);
        setConnectionStatus('Connection Failed');
        return () => {};
      }
    };

    let cleanup: (() => void) | null = null;
    connectAndSubscribe().then((cleanupFn) => {
      cleanup = cleanupFn;
    });

    return () => {
      console.log('🧪 Test component: Cleaning up...');
      if (cleanup) cleanup();
      apiClient.disconnectServerWebSocket();
      setIsConnected(false);
      setConnectionStatus('Disconnected');
    };
  }, [selectedSymbol, selectedInterval]);

  const handleSymbolChange = (symbol: string) => {
    setSelectedSymbol(symbol);
    setMessages([]); // Clear existing messages
  };

  const handleIntervalChange = (interval: string) => {
    setSelectedInterval(interval);
    setMessages([]); // Clear existing messages
  };

  const clearMessages = () => {
    setMessages([]);
  };

  const testConnection = async () => {
    try {
      setConnectionStatus('Testing...');
      await apiClient.connectServerWebSocket();
      setIsConnected(true);
      setConnectionStatus('Connected');
      setError(null);
    } catch (err) {
      setError(`Test failed: ${err}`);
      setConnectionStatus('Test Failed');
    }
  };

  return (
    <div className="p-6 bg-gray-900 rounded-lg border border-gray-700 max-w-4xl mx-auto">
      <h3 className="text-white text-xl font-bold mb-4">🧪 WebSocket Test Component</h3>
      
      <div className="space-y-4">
        {/* Connection Status */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`}></div>
            <span className="text-white">
              Status: <span className="font-bold">{connectionStatus}</span>
            </span>
          </div>
          
          <button 
            onClick={testConnection}
            className="px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
          >
            Test Connection
          </button>
        </div>

        {/* Controls */}
        <div className="flex gap-4 items-end">
          <div>
            <label className="text-white text-sm mb-1 block">Symbol:</label>
            <select 
              value={selectedSymbol} 
              onChange={(e) => handleSymbolChange(e.target.value)}
              className="px-3 py-1 bg-gray-800 text-white rounded border border-gray-600"
            >
              {symbols.map(symbol => (
                <option key={symbol} value={symbol}>{symbol}</option>
              ))}
            </select>
          </div>
          
          <div>
            <label className="text-white text-sm mb-1 block">Interval:</label>
            <select 
              value={selectedInterval} 
              onChange={(e) => handleIntervalChange(e.target.value)}
              className="px-3 py-1 bg-gray-800 text-white rounded border border-gray-600"
            >
              {intervals.map(interval => (
                <option key={interval} value={interval}>{interval}</option>
              ))}
            </select>
          </div>

          <button 
            onClick={clearMessages}
            className="px-3 py-1 bg-gray-600 text-white rounded hover:bg-gray-700 text-sm"
          >
            Clear Messages
          </button>
        </div>

        {/* Error Display */}
        {error && (
          <div className="p-3 bg-red-900/50 border border-red-500 rounded text-red-200">
            <strong>Error:</strong> {error}
          </div>
        )}

        {/* Connection Info */}
        <div className="text-xs text-gray-400 bg-gray-800/50 p-3 rounded">
          <div><strong>WebSocket URL:</strong> ws://localhost:3000/ws</div>
          <div><strong>Current Subscription:</strong> {selectedSymbol} {selectedInterval}</div>
          <div><strong>Messages Received:</strong> {messages.length}</div>
          <div><strong>Server Status:</strong> {apiClient.isServerConnected() ? 'Connected' : 'Disconnected'}</div>
        </div>

        {/* Messages */}
        <div>
          <h4 className="text-white font-semibold mb-2">
            Live Messages ({messages.length}):
          </h4>
          <div className="max-h-96 overflow-y-auto bg-black/20 p-3 rounded border border-gray-600">
            {messages.length === 0 ? (
              <div className="text-gray-400 text-sm">No messages received yet...</div>
            ) : (
              <div className="space-y-2">
                {messages.map((msg, index) => (
                  <div key={index} className="text-sm text-gray-300 font-mono bg-gray-800/50 p-3 rounded">
                    <div className="flex justify-between items-center mb-2">
                      <span className="text-blue-400 font-bold">{msg.type}</span>
                      <span className="text-gray-500 text-xs">{msg.timestamp}</span>
                    </div>
                    
                    {msg.type === 'candle' && (
                      <div className="space-y-1">
                        <div className="text-xs text-gray-500">
                          Symbol: {msg.symbol} | Timeframe: {msg.timeframe} | ID: {msg.subscriptionId}
                        </div>
                        <div className="grid grid-cols-4 gap-2 text-xs">
                          <div>O: {msg.data?.open}</div>
                          <div>H: {msg.data?.high}</div>
                          <div>L: {msg.data?.low}</div>
                          <div>C: {msg.data?.close}</div>
                        </div>
                        {msg.serverData && (
                          <div className="text-xs text-green-400 mt-1">
                            Server: {msg.serverData.priceChangePercent?.toFixed(2)}% change, 
                            Vol: {msg.serverData.volume}, 
                            Trades: {msg.serverData.trades}
                          </div>
                        )}
                      </div>
                    )}
                    
                    {msg.type === 'subscription' && (
                      <div className="text-xs text-green-400">
                        Subscription confirmed: {msg.subscriptionId}
                      </div>
                    )}
                    
                    {msg.type === 'error' && (
                      <div className="text-xs text-red-400">
                        Error: {msg.data}
                      </div>
                    )}
                    
                    <details className="mt-2">
                      <summary className="text-xs text-gray-500 cursor-pointer">Raw Data</summary>
                      <pre className="text-xs text-gray-400 mt-1 overflow-x-auto">
                        {JSON.stringify(msg.rawMessage, null, 2)}
                      </pre>
                    </details>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Debug Instructions */}
        <div className="text-xs text-gray-400 bg-blue-900/20 p-3 rounded border border-blue-500/30">
          <div className="font-bold text-blue-400 mb-2">🔍 Debug Instructions:</div>
          <div>1. Check browser console for detailed logs</div>
          <div>2. Verify your server is running on ws://localhost:3000/ws</div>
          <div>3. Ensure your server sends data in the expected format</div>
          <div>4. Look for connection, subscription, and message logs</div>
        </div>
      </div>
    </div>
  );
}
