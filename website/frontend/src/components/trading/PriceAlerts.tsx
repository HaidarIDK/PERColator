"use client"

import { useState, useEffect } from 'react';
import { Bell, BellRing, Plus, Trash2, TrendingUp, TrendingDown, X } from 'lucide-react';
import { toast } from '@/components/ui/use-toast';

interface Alert {
  id: string;
  coin: string;
  symbol: string;
  targetPrice: number;
  condition: 'above' | 'below';
  createdAt: Date;
  triggered: boolean;
}

interface PriceAlertsProps {
  prices: Record<string, number>;
  currentCoin: string;
  currentSymbol: string;
}

export function PriceAlerts({ prices, currentCoin, currentSymbol }: PriceAlertsProps) {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [showAddAlert, setShowAddAlert] = useState(false);
  const [newAlertPrice, setNewAlertPrice] = useState('');
  const [newAlertCondition, setNewAlertCondition] = useState<'above' | 'below'>('above');

  // Load alerts from localStorage
  useEffect(() => {
    const savedAlerts = localStorage.getItem('price-alerts');
    if (savedAlerts) {
      try {
        const parsed = JSON.parse(savedAlerts);
        setAlerts(parsed.map((a: any) => ({ ...a, createdAt: new Date(a.createdAt) })));
      } catch (e) {
        console.error('Failed to load alerts:', e);
      }
    }
  }, []);

  // Save alerts to localStorage
  useEffect(() => {
    if (alerts.length > 0) {
      localStorage.setItem('price-alerts', JSON.stringify(alerts));
    }
  }, [alerts]);

  // Check alerts against current prices
  useEffect(() => {
    alerts.forEach(alert => {
      if (alert.triggered) return;

      const currentPrice = prices[alert.symbol];
      if (!currentPrice) return;

      const shouldTrigger =
        (alert.condition === 'above' && currentPrice >= alert.targetPrice) ||
        (alert.condition === 'below' && currentPrice <= alert.targetPrice);

      if (shouldTrigger) {
        // Trigger the alert
        setAlerts(prev => prev.map(a =>
          a.id === alert.id ? { ...a, triggered: true } : a
        ));

        // Show notification
        toast({
          type: 'info',
          title: 'Price Alert Triggered!',
          message: `${alert.symbol} is now ${alert.condition} $${alert.targetPrice.toFixed(2)} (Current: $${currentPrice.toFixed(2)})`
        });

        // Play sound if available
        try {
          const audio = new Audio('/alert.mp3');
          audio.volume = 0.5;
          audio.play().catch(() => {});
        } catch (e) {}
      }
    });
  }, [prices, alerts]);

  const addAlert = () => {
    if (!newAlertPrice || isNaN(parseFloat(newAlertPrice))) return;

    const newAlert: Alert = {
      id: Date.now().toString(),
      coin: currentCoin,
      symbol: currentSymbol,
      targetPrice: parseFloat(newAlertPrice),
      condition: newAlertCondition,
      createdAt: new Date(),
      triggered: false,
    };

    setAlerts(prev => [...prev, newAlert]);
    setNewAlertPrice('');
    setShowAddAlert(false);

    toast({
      type: 'success',
      title: 'Alert Created',
      message: `Alert set for ${currentSymbol} ${newAlertCondition} $${parseFloat(newAlertPrice).toFixed(2)}`
    });
  };

  const deleteAlert = (id: string) => {
    setAlerts(prev => prev.filter(a => a.id !== id));
  };

  const clearTriggered = () => {
    setAlerts(prev => prev.filter(a => !a.triggered));
  };

  const activeAlerts = alerts.filter(a => !a.triggered);
  const triggeredAlerts = alerts.filter(a => a.triggered);

  return (
    <div className="bg-zinc-900/50 rounded-xl p-4 border border-white/5">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Bell className="w-4 h-4 text-zinc-400" />
          <h3 className="text-sm font-semibold text-zinc-300">Price Alerts</h3>
          {activeAlerts.length > 0 && (
            <span className="text-[10px] bg-blue-500/20 text-blue-400 px-1.5 py-0.5 rounded-full">
              {activeAlerts.length} active
            </span>
          )}
        </div>
        <button
          onClick={() => setShowAddAlert(!showAddAlert)}
          className="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1"
        >
          <Plus className="w-3 h-3" />
          Add Alert
        </button>
      </div>

      {/* Add Alert Form */}
      {showAddAlert && (
        <div className="bg-zinc-950/50 rounded-lg p-3 mb-3 border border-blue-500/20">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs text-zinc-400">New alert for {currentSymbol}</span>
            <button onClick={() => setShowAddAlert(false)} className="text-zinc-500 hover:text-zinc-300">
              <X className="w-3 h-3" />
            </button>
          </div>
          <div className="flex gap-2">
            <select
              value={newAlertCondition}
              onChange={(e) => setNewAlertCondition(e.target.value as 'above' | 'below')}
              className="bg-zinc-900 border border-white/10 rounded px-2 py-1.5 text-xs text-zinc-300 focus:outline-none focus:border-blue-500/50"
            >
              <option value="above">Price Above</option>
              <option value="below">Price Below</option>
            </select>
            <input
              type="number"
              value={newAlertPrice}
              onChange={(e) => setNewAlertPrice(e.target.value)}
              placeholder={`${prices[currentSymbol]?.toFixed(2) || '0.00'}`}
              className="flex-1 bg-zinc-900 border border-white/10 rounded px-2 py-1.5 text-xs text-zinc-300 focus:outline-none focus:border-blue-500/50"
            />
            <button
              onClick={addAlert}
              disabled={!newAlertPrice}
              className="bg-blue-500 hover:bg-blue-600 disabled:bg-zinc-700 disabled:text-zinc-500 text-white text-xs px-3 py-1.5 rounded font-medium transition-colors"
            >
              Set
            </button>
          </div>
        </div>
      )}

      {/* Active Alerts */}
      {activeAlerts.length === 0 && triggeredAlerts.length === 0 ? (
        <p className="text-xs text-zinc-500 text-center py-3">No price alerts set</p>
      ) : (
        <div className="space-y-2">
          {activeAlerts.map(alert => {
            const currentPrice = prices[alert.symbol] || 0;
            const distance = ((alert.targetPrice - currentPrice) / currentPrice) * 100;

            return (
              <div
                key={alert.id}
                className="flex items-center justify-between bg-zinc-950/50 rounded-lg px-3 py-2 border border-white/5"
              >
                <div className="flex items-center gap-2">
                  {alert.condition === 'above' ? (
                    <TrendingUp className="w-3 h-3 text-emerald-400" />
                  ) : (
                    <TrendingDown className="w-3 h-3 text-red-400" />
                  )}
                  <div>
                    <p className="text-xs text-zinc-300">
                      <span className="font-medium">{alert.symbol}</span>
                      <span className="text-zinc-500"> {alert.condition} </span>
                      <span className="font-mono text-zinc-200">${alert.targetPrice.toFixed(2)}</span>
                    </p>
                    <p className="text-[10px] text-zinc-500">
                      {distance > 0 ? '+' : ''}{distance.toFixed(2)}% from current
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => deleteAlert(alert.id)}
                  className="text-zinc-500 hover:text-red-400 transition-colors"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            );
          })}

          {/* Triggered Alerts */}
          {triggeredAlerts.length > 0 && (
            <>
              <div className="flex items-center justify-between pt-2 border-t border-white/5">
                <span className="text-[10px] text-zinc-500 uppercase">Triggered</span>
                <button
                  onClick={clearTriggered}
                  className="text-[10px] text-zinc-500 hover:text-zinc-300"
                >
                  Clear all
                </button>
              </div>
              {triggeredAlerts.map(alert => (
                <div
                  key={alert.id}
                  className="flex items-center justify-between bg-emerald-500/5 rounded-lg px-3 py-2 border border-emerald-500/20"
                >
                  <div className="flex items-center gap-2">
                    <BellRing className="w-3 h-3 text-emerald-400" />
                    <div>
                      <p className="text-xs text-zinc-300">
                        <span className="font-medium">{alert.symbol}</span>
                        <span className="text-zinc-500"> hit </span>
                        <span className="font-mono text-emerald-400">${alert.targetPrice.toFixed(2)}</span>
                      </p>
                    </div>
                  </div>
                  <button
                    onClick={() => deleteAlert(alert.id)}
                    className="text-zinc-500 hover:text-red-400 transition-colors"
                  >
                    <X className="w-3 h-3" />
                  </button>
                </div>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
}
