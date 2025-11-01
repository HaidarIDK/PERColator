import type { Metadata, Viewport } from "next";
import "./globals.css";
import { WalletProvider } from "@/components/WalletProvider";
import { ToastProvider } from "@/components/ToastProvider";
import Script from "next/script";

export const metadata: Metadata = {
  title: "Percolator - Perpetual DEX",
  description: "Decentralized perpetual exchange on Solana",
};

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 5,
  userScalable: true,
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <head>
        <Script id="suppress-wallet-warnings" strategy="afterInteractive">
          {`
            const originalWarn = console.warn;
            console.warn = function(...args) {
              const msg = args.join(' ');
              if (msg.includes('StreamMiddleware') || 
                  msg.includes('SES_UNCAUGHT_EXCEPTION') ||
                  msg.includes('Standard Wallet')) {
                return;
              }
              originalWarn.apply(console, args);
            };
          `}
        </Script>
      </head>
      <body className="antialiased bg-gray-900 text-white">
        <WalletProvider>
          <ToastProvider>
            {children}
          </ToastProvider>
        </WalletProvider>
      </body>
    </html>
  );
}
