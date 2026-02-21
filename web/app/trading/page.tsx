"use client";

import { useMemo } from "react";
import { Header } from "@/components/Header";
import { SwapWidget } from "@/components/SwapWidget";
import { TradingViewChart } from "@/components/TradingViewChart";
import { useBinanceData, formatOrderBookRow } from "@/hooks/useBinanceData";
import {
  getPlaceholderPoolData,
  getPlaceholderSwapHandler,
} from "@/lib/placeholders";

const ROWS_PER_SIDE = 10;

// Placeholder positions and orders
const PLACEHOLDER_POSITIONS: { symbol: string; side: string; size: string; entry: string; pnl: string }[] = [];
const PLACEHOLDER_ORDERS: { id: string; type: string; pair: string; amount: string; status: string }[] = [
  { id: "0x1a2b", type: "Limit Buy", pair: "ETH/USDC", amount: "0.5 ETH", status: "Filled" },
  { id: "0x3c4d", type: "Market Sell", pair: "ETH/USDC", amount: "0.25 ETH", status: "Filled" },
  { id: "0x5e6f", type: "Limit Sell", pair: "ETH/USDC", amount: "1.0 ETH", status: "Open" },
];

function formatPrice(p: string): string {
  const n = parseFloat(p);
  if (n >= 1000) return n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

export default function TradingPage() {
  const poolData = useMemo(() => getPlaceholderPoolData(), []);
  const swapHandler = useMemo(() => getPlaceholderSwapHandler(), []);
  const { lastPrice, bids, asks, isConnecting, error } = useBinanceData();

  const displayBids = bids.slice(0, ROWS_PER_SIDE);
  const displayAsks = asks.slice(0, ROWS_PER_SIDE);
  const spread =
    displayBids.length > 0 && displayAsks.length > 0
      ? (parseFloat(displayAsks[0][0]) - parseFloat(displayBids[0][0])).toFixed(2)
      : null;

  return (
    <div className="flex min-h-screen flex-col bg-oak-bg">
      <Header />
      <main className="flex flex-1 flex-col overflow-hidden">
        {/* Trading layout: chart (80%) + sidebar (20%) */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left/Center: Chart - 80% width */}
          <div className="flex w-[80%] flex-col border-r border-oak-border bg-oak-bg-elevated">
            <div className="flex items-center justify-between border-b border-oak-border px-4 py-2">
              <h1 className="text-sm font-medium text-oak-text-primary">
                ETH/USDC · 1H
              </h1>
              <span className="text-xs text-oak-text-muted">
                Powered by TradingView
              </span>
            </div>
            <div className="relative flex-1 min-h-0">
              <TradingViewChart
                symbol="BINANCE:ETHUSDT"
                theme="dark"
                autosize
                interval="60"
                className="absolute inset-0"
              />
            </div>
          </div>

          {/* Right: Sidebar - 20% width */}
          <aside className="flex w-[20%] min-w-[280px] flex-col gap-4 overflow-y-auto border-l border-oak-border bg-oak-bg p-4">
            {/* Swap Widget */}
            <SwapWidget
              token0Symbol={poolData.token0Symbol}
              token1Symbol={poolData.token1Symbol}
              token0Balance={poolData.token0Balance}
              estimatedOutput={poolData.estimatedOutput}
              marketPrice={lastPrice}
              isLoadingQuote={poolData.isLoadingQuote}
              onSwap={swapHandler}
              error={poolData.error ?? error}
            />

            {/* Order Book - Live from Binance depth stream */}
            <div className="rounded-oak-lg border border-oak-border bg-oak-bg-card">
              <div className="flex items-center justify-between border-b border-oak-border px-4 py-3">
                <h2 className="text-sm font-medium text-oak-text-primary">
                  Order Book
                </h2>
                {isConnecting && (
                  <span className="text-xs text-oak-text-muted animate-pulse">
                    Connecting to live feed…
                  </span>
                )}
              </div>
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-oak-text-muted">
                      <th className="px-3 py-2 text-left font-medium">Price</th>
                      <th className="px-3 py-2 text-right font-medium">Amount</th>
                      <th className="px-3 py-2 text-right font-medium">Total</th>
                    </tr>
                  </thead>
                  <tbody>
                    {/* Bids (highest first) */}
                    {displayBids.map((row, i) => {
                      const f = formatOrderBookRow(row);
                      return (
                        <tr
                          key={`b-${i}`}
                          className="border-t border-oak-border/60 text-oak-accent"
                        >
                          <td className="px-3 py-1.5 font-mono">{f.price}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.amount}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.total}</td>
                        </tr>
                      );
                    })}
                    {/* Current price / spread row */}
                    <tr className="border-t border-oak-border bg-oak-bg-elevated">
                      <td
                        colSpan={3}
                        className="px-3 py-2 text-center font-mono text-base font-semibold text-oak-text-primary"
                      >
                        {lastPrice ? formatPrice(lastPrice) : "—"}
                        {spread != null && (
                          <span className="ml-2 text-xs font-normal text-oak-text-muted">
                            (Δ {spread})
                          </span>
                        )}
                      </td>
                    </tr>
                    {/* Asks (lowest first) */}
                    {displayAsks.map((row, i) => {
                      const f = formatOrderBookRow(row);
                      return (
                        <tr
                          key={`a-${i}`}
                          className="border-t border-oak-border/60 text-oak-error"
                        >
                          <td className="px-3 py-1.5 font-mono">{f.price}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.amount}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.total}</td>
                        </tr>
                      );
                    })}
                    {displayBids.length === 0 && displayAsks.length === 0 && !isConnecting && (
                      <tr>
                        <td colSpan={3} className="px-3 py-6 text-center text-oak-text-muted">
                          No order book data
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          </aside>
        </div>

        {/* Bottom: Open Positions + Order History */}
        <div className="flex border-t border-oak-border bg-oak-bg-card">
          {/* Open Positions */}
          <div className="w-1/2 border-r border-oak-border">
            <h2 className="border-b border-oak-border px-4 py-2 text-sm font-medium text-oak-text-primary">
              Open Positions
            </h2>
            <div className="min-h-[100px] overflow-x-auto">
              {PLACEHOLDER_POSITIONS.length === 0 ? (
                <p className="px-4 py-6 text-center text-sm text-oak-text-muted">
                  No open positions
                </p>
              ) : (
                <table className="w-full text-xs">
                  <thead>
                    <tr className="text-oak-text-muted">
                      <th className="px-3 py-2 text-left">Symbol</th>
                      <th className="px-3 py-2 text-left">Side</th>
                      <th className="px-3 py-2 text-right">Size</th>
                      <th className="px-3 py-2 text-right">Entry</th>
                      <th className="px-3 py-2 text-right">PnL</th>
                    </tr>
                  </thead>
                  <tbody>
                    {PLACEHOLDER_POSITIONS.map((p, i) => (
                      <tr key={i} className="border-t border-oak-border/60">
                        <td className="px-3 py-2 font-mono">{p.symbol}</td>
                        <td className="px-3 py-2">{p.side}</td>
                        <td className="px-3 py-2 text-right font-mono">{p.size}</td>
                        <td className="px-3 py-2 text-right font-mono">{p.entry}</td>
                        <td className="px-3 py-2 text-right font-mono">{p.pnl}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>

          {/* Order History */}
          <div className="w-1/2">
            <h2 className="border-b border-oak-border px-4 py-2 text-sm font-medium text-oak-text-primary">
              Order History
            </h2>
            <div className="min-h-[100px] overflow-x-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-oak-text-muted">
                    <th className="px-3 py-2 text-left">Type</th>
                    <th className="px-3 py-2 text-left">Pair</th>
                    <th className="px-3 py-2 text-right">Amount</th>
                    <th className="px-3 py-2 text-right">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {PLACEHOLDER_ORDERS.map((o) => (
                    <tr
                      key={o.id}
                      className="border-t border-oak-border/60 text-oak-text-secondary"
                    >
                      <td className="px-3 py-2">{o.type}</td>
                      <td className="px-3 py-2 font-mono">{o.pair}</td>
                      <td className="px-3 py-2 text-right font-mono">{o.amount}</td>
                      <td className="px-3 py-2 text-right">
                        <span
                          className={
                            o.status === "Filled"
                              ? "text-oak-accent"
                              : "text-oak-text-muted"
                          }
                        >
                          {o.status}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
