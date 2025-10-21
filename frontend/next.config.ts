import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  eslint: {
    ignoreDuringBuilds: true,
  },
  typescript: {
    ignoreBuildErrors: true,
  },
  async headers() {
    return [
      {
        source: '/:path*',
        headers: [
          {
            key: 'Content-Security-Policy',
            value: "default-src 'self'; script-src 'self' 'unsafe-eval' 'unsafe-inline' https://s3.tradingview.com https://www.tradingview.com; style-src 'self' 'unsafe-inline' https://www.tradingview.com https://fonts.googleapis.com http://fonts.googleapis.com; style-src-elem 'self' 'unsafe-inline' https://www.tradingview.com https://fonts.googleapis.com http://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com http://fonts.gstatic.com; frame-src https://www.tradingview.com https://s.tradingview.com https://tradingview-widget.com https://www.tradingview-widget.com; connect-src 'self' http://localhost:* ws://localhost:* wss://localhost:* https://www.tradingview.com https://tradingview-widget.com https://www.tradingview-widget.com wss://widgetdata.tradingview.com wss://data.tradingview.com wss://api.hyperliquid.xyz wss://api.hyperliquid-testnet.xyz https://api.devnet.solana.com https://api.mainnet-beta.solana.com; img-src 'self' data: https://www.tradingview.com;",
          },
        ],
      },
    ];
  },
};

export default nextConfig;
