"use client"

import { useEffect, useRef, useState } from 'react';
import { type Orderbook } from "@/lib/api-client"

interface DepthChartProps {
  orderbook: Orderbook | null;
  midPrice: number;
}

export function DepthChart({ orderbook, midPrice }: DepthChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [hoveredPrice, setHoveredPrice] = useState<{ price: number; cumulative: number; side: 'bid' | 'ask' } | null>(null);

  useEffect(() => {
    if (!canvasRef.current || !containerRef.current || !orderbook) return;

    const canvas = canvasRef.current;
    const container = containerRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;

    // Clear canvas
    ctx.fillStyle = 'rgb(24, 24, 27)';
    ctx.fillRect(0, 0, width, height);

    const bids = orderbook.bids.slice(0, 20);
    const asks = orderbook.asks.slice(0, 20);

    if (bids.length === 0 && asks.length === 0) {
      ctx.fillStyle = 'rgb(113, 113, 122)';
      ctx.font = '12px Inter, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('No orderbook data', width / 2, height / 2);
      return;
    }

    // Calculate cumulative volumes
    let bidCumulative: { price: number; cumulative: number }[] = [];
    let askCumulative: { price: number; cumulative: number }[] = [];
    let runningTotal = 0;

    // Bids (reverse order for chart)
    for (let i = bids.length - 1; i >= 0; i--) {
      runningTotal += bids[i].quantity;
      bidCumulative.push({ price: bids[i].price, cumulative: runningTotal });
    }
    bidCumulative = bidCumulative.reverse();

    runningTotal = 0;
    for (const ask of asks) {
      runningTotal += ask.quantity;
      askCumulative.push({ price: ask.price, cumulative: runningTotal });
    }

    // Find price range and max cumulative
    const allPrices = [...bids.map(b => b.price), ...asks.map(a => a.price)];
    const minPrice = Math.min(...allPrices);
    const maxPrice = Math.max(...allPrices);
    const priceRange = maxPrice - minPrice || 1;
    const maxCumulative = Math.max(
      bidCumulative[0]?.cumulative || 0,
      askCumulative[askCumulative.length - 1]?.cumulative || 0
    );

    const padding = { top: 20, right: 10, bottom: 30, left: 10 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    // Helper functions
    const priceToX = (price: number) => {
      return padding.left + ((price - minPrice) / priceRange) * chartWidth;
    };

    const cumulativeToY = (cumulative: number) => {
      return padding.top + chartHeight - (cumulative / maxCumulative) * chartHeight;
    };

    // Draw grid lines
    ctx.strokeStyle = 'rgba(63, 63, 70, 0.3)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
      const y = padding.top + (chartHeight * i) / 4;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(width - padding.right, y);
      ctx.stroke();
    }

    // Draw mid price line
    if (midPrice > 0) {
      const midX = priceToX(midPrice);
      ctx.strokeStyle = 'rgba(250, 204, 21, 0.5)';
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      ctx.moveTo(midX, padding.top);
      ctx.lineTo(midX, height - padding.bottom);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Draw bid area (green)
    if (bidCumulative.length > 0) {
      ctx.beginPath();
      ctx.moveTo(priceToX(bidCumulative[0].price), cumulativeToY(0));

      for (const point of bidCumulative) {
        ctx.lineTo(priceToX(point.price), cumulativeToY(point.cumulative));
      }

      ctx.lineTo(priceToX(bidCumulative[bidCumulative.length - 1].price), cumulativeToY(0));
      ctx.closePath();

      const bidGradient = ctx.createLinearGradient(0, padding.top, 0, height - padding.bottom);
      bidGradient.addColorStop(0, 'rgba(34, 197, 94, 0.4)');
      bidGradient.addColorStop(1, 'rgba(34, 197, 94, 0.05)');
      ctx.fillStyle = bidGradient;
      ctx.fill();

      // Draw bid line
      ctx.beginPath();
      ctx.moveTo(priceToX(bidCumulative[0].price), cumulativeToY(bidCumulative[0].cumulative));
      for (const point of bidCumulative) {
        ctx.lineTo(priceToX(point.price), cumulativeToY(point.cumulative));
      }
      ctx.strokeStyle = 'rgb(34, 197, 94)';
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Draw ask area (red)
    if (askCumulative.length > 0) {
      ctx.beginPath();
      ctx.moveTo(priceToX(askCumulative[0].price), cumulativeToY(0));

      for (const point of askCumulative) {
        ctx.lineTo(priceToX(point.price), cumulativeToY(point.cumulative));
      }

      ctx.lineTo(priceToX(askCumulative[askCumulative.length - 1].price), cumulativeToY(0));
      ctx.closePath();

      const askGradient = ctx.createLinearGradient(0, padding.top, 0, height - padding.bottom);
      askGradient.addColorStop(0, 'rgba(239, 68, 68, 0.4)');
      askGradient.addColorStop(1, 'rgba(239, 68, 68, 0.05)');
      ctx.fillStyle = askGradient;
      ctx.fill();

      // Draw ask line
      ctx.beginPath();
      ctx.moveTo(priceToX(askCumulative[0].price), cumulativeToY(askCumulative[0].cumulative));
      for (const point of askCumulative) {
        ctx.lineTo(priceToX(point.price), cumulativeToY(point.cumulative));
      }
      ctx.strokeStyle = 'rgb(239, 68, 68)';
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Draw price labels
    ctx.fillStyle = 'rgb(113, 113, 122)';
    ctx.font = '10px Inter, sans-serif';
    ctx.textAlign = 'center';

    const priceSteps = 5;
    for (let i = 0; i <= priceSteps; i++) {
      const price = minPrice + (priceRange * i) / priceSteps;
      const x = priceToX(price);
      ctx.fillText(price.toFixed(2), x, height - 8);
    }

    // Draw volume labels
    ctx.textAlign = 'left';
    ctx.fillText(maxCumulative.toFixed(2), 5, padding.top + 10);
    ctx.fillText('0', 5, height - padding.bottom - 5);

  }, [orderbook, midPrice]);

  return (
    <div className="bg-zinc-900/50 rounded-xl border border-white/5 overflow-hidden">
      <div className="px-3 py-2 border-b border-white/5 flex items-center justify-between">
        <h3 className="text-xs font-semibold text-zinc-300">Market Depth</h3>
        <div className="flex items-center gap-3 text-[10px]">
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 rounded-full bg-emerald-500"></div>
            <span className="text-zinc-500">Bids</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-2 h-2 rounded-full bg-red-500"></div>
            <span className="text-zinc-500">Asks</span>
          </div>
        </div>
      </div>
      <div ref={containerRef} className="h-[200px] relative">
        <canvas ref={canvasRef} className="w-full h-full" />
        {hoveredPrice && (
          <div className="absolute top-2 left-2 bg-zinc-950/90 border border-white/10 rounded px-2 py-1 text-[10px]">
            <p className="text-zinc-400">Price: <span className="text-zinc-200">${hoveredPrice.price.toFixed(2)}</span></p>
            <p className="text-zinc-400">Cumulative: <span className={hoveredPrice.side === 'bid' ? 'text-emerald-400' : 'text-red-400'}>{hoveredPrice.cumulative.toFixed(4)}</span></p>
          </div>
        )}
      </div>
    </div>
  );
}
