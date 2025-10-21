'use client';

import { useState, useEffect } from 'react';

export default function WebSocketDebug() {
  const [status, setStatus] = useState<string>('Disconnected');
  const [messages, setMessages] = useState<string[]>([]);
  const [ws, setWs] = useState<WebSocket | null>(null);

  const connect = () => {
    try {
      setStatus('Connecting...');
      const websocket = new WebSocket('wss://api.hyperliquid.xyz/ws');
      
      websocket.onopen = () => {
        setStatus('Connected');
        setMessages(prev => [...prev, 'Connected to Hyperliquid WebSocket']);
        
        // Test different subscription formats
        const subscribeMessage = {
          method: 'subscribe',
          subscription: {
            type: 'candle',
            coin: 'SOL',
            interval: '1m'
          }
        };
        
        websocket.send(JSON.stringify(subscribeMessage));
        setMessages(prev => [...prev, 'Sent candle subscription: ' + JSON.stringify(subscribeMessage)]);
        
        // Also try trades subscription
        setTimeout(() => {
          const tradesMessage = {
            method: 'subscribe',
            subscription: {
              type: 'trades',
              coin: 'SOL'
            }
          };
          websocket.send(JSON.stringify(tradesMessage));
          setMessages(prev => [...prev, 'Sent trades subscription: ' + JSON.stringify(tradesMessage)]);
        }, 1000);
      };
      
      websocket.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          setMessages(prev => [...prev.slice(-9), `Received: ${JSON.stringify(data, null, 2)}`]);
        } catch (e) {
          setMessages(prev => [...prev.slice(-9), `Received (raw): ${event.data}`]);
        }
      };
      
      websocket.onerror = (error) => {
        setStatus('Error');
        setMessages(prev => [...prev, `Error: ${error}`]);
        console.error('WebSocket error:', error);
      };
      
      websocket.onclose = (event) => {
        setStatus(`Disconnected (Code: ${event.code})`);
        setMessages(prev => [...prev, `Connection closed: Code ${event.code}, Reason: ${event.reason}`]);
      };
      
      setWs(websocket);
    } catch (error) {
      setStatus('Failed to connect');
      setMessages(prev => [...prev, `Failed to connect: ${error}`]);
    }
  };

  const disconnect = () => {
    if (ws) {
      ws.close();
      setWs(null);
    }
  };

  return (
    <div className="p-4 bg-gray-800 rounded-lg">
      <h3 className="text-white text-lg font-bold mb-4">WebSocket Debug</h3>
      
      <div className="space-y-4">
        <div className="flex gap-2">
          <button 
            onClick={connect}
            className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
          >
            Connect
          </button>
          <button 
            onClick={disconnect}
            className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
          >
            Disconnect
          </button>
        </div>
        
        <div className="text-white">
          Status: <span className="font-bold">{status}</span>
        </div>
        
        <div className="text-white">
          <div className="font-semibold mb-2">Messages:</div>
          <div className="space-y-1 max-h-64 overflow-y-auto bg-black/20 p-2 rounded">
            {messages.map((msg, index) => (
              <div key={index} className="text-sm text-gray-300 font-mono">{msg}</div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
