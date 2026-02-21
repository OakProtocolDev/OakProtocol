"use client";

import { useMemo, useState, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Header } from "@/components/Header";
import { SwapWidget } from "@/components/SwapWidget";
import { TradingViewChart } from "@/components/TradingViewChart";
import { LiveLogsPanel, type LogEntry } from "@/components/LiveLogsPanel";
import { SuccessModal } from "@/components/SuccessModal";
import { useBinanceData, formatOrderBookRow } from "@/hooks/useBinanceData";
import {
  getPlaceholderPoolData,
  getPlaceholderSwapHandler,
} from "@/lib/placeholders";

const ROWS_PER_SIDE = 10;

const PLACEHOLDER_POSITIONS: { symbol: string; side: string; size: string; entry: string; pnl: string }[] = [];
const PLACEHOLDER_ORDERS: { id: string; type: string; pair: string; amount: string; status: string }[] = [
  { id: "0x1a2b", type: "Limit Buy", pair: "ETH/USDC", amount: "0.5 ETH", status: "Filled" },
  { id: "0x3c4d", type: "Market Sell", pair: "ETH/USDC", amount: "0.25 ETH", status: "Filled" },
  { id: "0x5e6f", type: "Limit Sell", pair: "ETH/USDC", amount: "1.0 ETH", status: "Open" },
];

export interface TradeRecord {
  id: string;
  timestamp: number;
  side: "buy" | "sell";
  amountIn: string;
  amountOut: string;
  token0Symbol: string;
  token1Symbol: string;
  txHash: string;
}

function formatPrice(p: string): string {
  const n = parseFloat(p);
  if (n >= 1000) return n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

function randomTxHash(): string {
  return "0x" + Array.from(crypto.getRandomValues(new Uint8Array(32)))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function addLogEntry(setLogs: React.Dispatch<React.SetStateAction<LogEntry[]>>, message: string, level?: LogEntry["level"]) {
  setLogs((prev) => [
    ...prev,
    {
      id: crypto.randomUUID(),
      timestamp: new Date().toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit", fractionalSecondDigits: 3 }),
      message,
      level,
    },
  ]);
}

type BottomTab = "positions" | "orders" | "trades";

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

export default function TradingPage() {
  const poolData = useMemo(() => getPlaceholderPoolData(), []);
  const baseSwapHandler = useMemo(() => getPlaceholderSwapHandler(), []);
  const { lastPrice, bids, asks, isConnecting, error } = useBinanceData();

  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [tradeHistory, setTradeHistory] = useState<TradeRecord[]>([]);
  const [bottomTab, setBottomTab] = useState<BottomTab>("orders");
  const [successModal, setSuccessModal] = useState<{
    isOpen: boolean;
    txHash: string;
    amountIn: string;
    amountOut: string;
  }>({ isOpen: false, txHash: "", amountIn: "", amountOut: "" });

  const swapHandler = useCallback(
    async (amountIn: string, minAmountOut: string, deadline: number) => {
      const amountOut = lastPrice
        ? (parseFloat(amountIn) * parseFloat(lastPrice)).toFixed(6)
        : "0";

      addLogEntry(setLogs, "[1/4] Submitting commit hash to mempool…", "info");
      await new Promise((r) => setTimeout(r, 380));
      addLogEntry(setLogs, "[2/4] Commitment broadcast · MEV shield active", "info");
      await new Promise((r) => setTimeout(r, 420));
      addLogEntry(setLogs, "[3/4] Revealing transaction on Arbitrum Stylus…", "info");
      await new Promise((r) => setTimeout(r, 550));
      addLogEntry(setLogs, "[4/4] Swap confirmed · Frontrunning protected", "success");

      await baseSwapHandler(amountIn, minAmountOut, deadline);

      const txHash = randomTxHash();
      setTradeHistory((prev) => [
        {
          id: crypto.randomUUID(),
          timestamp: Date.now(),
          side: "sell",
          amountIn,
          amountOut,
          token0Symbol: poolData.token0Symbol,
          token1Symbol: poolData.token1Symbol,
          txHash,
        },
        ...prev,
      ]);
      setSuccessModal({ isOpen: true, txHash, amountIn, amountOut });
    },
    [lastPrice, baseSwapHandler, poolData.token0Symbol, poolData.token1Symbol]
  );

  const displayBids = bids.slice(0, ROWS_PER_SIDE);
  const displayAsks = asks.slice(0, ROWS_PER_SIDE);
  const spread =
    displayBids.length > 0 && displayAsks.length > 0
      ? (parseFloat(displayAsks[0][0]) - parseFloat(displayBids[0][0])).toFixed(2)
      : null;

  return (
    <div className="flex min-h-screen flex-col">
      <Header />
      <SuccessModal
        isOpen={successModal.isOpen}
        txHash={successModal.txHash}
        amountIn={successModal.amountIn}
        amountOut={successModal.amountOut}
        token0Symbol={poolData.token0Symbol}
        token1Symbol={poolData.token1Symbol}
        onClose={() => setSuccessModal((s) => ({ ...s, isOpen: false }))}
      />
      <main className="flex flex-1 flex-col overflow-hidden">
        <div className="flex flex-1 overflow-hidden">
          {/* Chart area */}
          <div className="flex w-[80%] flex-col border-r border-oak-border/60 bg-oak-bg-elevated/50 backdrop-blur-sm">
            <div className="flex items-center justify-between border-b border-oak-border/60 px-4 py-2">
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

          {/* Sidebar */}
          <aside className="flex w-[20%] min-w-[280px] flex-col gap-4 overflow-y-auto border-l border-oak-border/60 bg-transparent p-4">
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

            {/* Order Book - Glass + animated rows + hover */}
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.4, delay: 0.05 }}
              className="glass-card overflow-hidden"
              style={{
                background: "rgba(15, 22, 19, 0.55)",
                backdropFilter: "blur(20px)",
                border: "1px solid rgba(34, 197, 94, 0.08)",
              }}
            >
              <div className="flex items-center justify-between border-b border-oak-border/60 px-4 py-3">
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
                    {displayBids.map((row, i) => {
                      const f = formatOrderBookRow(row);
                      return (
                        <motion.tr
                          key={`b-${row[0]}-${i}`}
                          initial={{ opacity: 0.6 }}
                          animate={{ opacity: 1 }}
                          transition={{ duration: 0.15 }}
                          className="group border-t border-oak-border/40 text-oak-accent transition-colors hover:bg-oak-accent/5"
                        >
                          <td className="px-3 py-1.5 font-mono">{f.price}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.amount}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.total}</td>
                        </motion.tr>
                      );
                    })}
                    <tr className="border-t border-oak-border bg-oak-bg-elevated/50">
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
                    {displayAsks.map((row, i) => {
                      const f = formatOrderBookRow(row);
                      return (
                        <motion.tr
                          key={`a-${row[0]}-${i}`}
                          initial={{ opacity: 0.6 }}
                          animate={{ opacity: 1 }}
                          transition={{ duration: 0.15 }}
                          className="group border-t border-oak-border/40 text-oak-error transition-colors hover:bg-oak-error/5"
                        >
                          <td className="px-3 py-1.5 font-mono">{f.price}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.amount}</td>
                          <td className="px-3 py-1.5 font-mono text-right">{f.total}</td>
                        </motion.tr>
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
            </motion.div>

            <LiveLogsPanel logs={logs} maxLines={6} className="mt-auto shrink-0" />
          </aside>
        </div>

        {/* Bottom panel - Glass + tab springs */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.2 }}
          className="glass-card flex flex-col border-t border-oak-border/60"
          style={{
            background: "rgba(15, 22, 19, 0.6)",
            backdropFilter: "blur(20px)",
            border: "1px solid rgba(34, 197, 94, 0.06)",
            borderTop: "1px solid rgba(34, 197, 94, 0.1)",
          }}
        >
          <div className="flex border-b border-oak-border/60">
            {(
              [
                ["positions", "Open Positions"],
                ["orders", "Order History"],
                ["trades", "Trade History"],
              ] as const
            ).map(([key, label]) => (
              <motion.button
                key={key}
                type="button"
                onClick={() => setBottomTab(key)}
                whileTap={springTap}
                className={`relative px-4 py-2.5 text-sm font-medium transition-colors ${
                  bottomTab === key
                    ? "text-oak-accent"
                    : "text-oak-text-muted hover:text-oak-text-primary"
                }`}
              >
                {bottomTab === key && (
                  <motion.span
                    layoutId="tab-indicator"
                    className="absolute inset-x-0 bottom-0 h-0.5 bg-oak-accent"
                    transition={{ type: "spring", stiffness: 400, damping: 30 }}
                  />
                )}
                {label}
              </motion.button>
            ))}
          </div>
          <div className="min-h-[120px] overflow-x-auto">
            {bottomTab === "positions" && (
              <>
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
                        <motion.tr
                          key={i}
                          initial={{ opacity: 0.5 }}
                          animate={{ opacity: 1 }}
                          className="border-t border-oak-border/40 text-oak-text-secondary transition-colors hover:bg-oak-accent/5"
                        >
                          <td className="px-3 py-2 font-mono">{p.symbol}</td>
                          <td className="px-3 py-2">{p.side}</td>
                          <td className="px-3 py-2 text-right font-mono">{p.size}</td>
                          <td className="px-3 py-2 text-right font-mono">{p.entry}</td>
                          <td className="px-3 py-2 text-right font-mono">{p.pnl}</td>
                        </motion.tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </>
            )}
            {bottomTab === "orders" && (
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
                    <motion.tr
                      key={o.id}
                      initial={{ opacity: 0.5 }}
                      animate={{ opacity: 1 }}
                      className="border-t border-oak-border/40 text-oak-text-secondary transition-colors hover:bg-oak-accent/5"
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
                    </motion.tr>
                  ))}
                </tbody>
              </table>
            )}
            {bottomTab === "trades" && (
              <>
                {tradeHistory.length === 0 ? (
                  <p className="px-4 py-6 text-center text-sm text-oak-text-muted">
                    No trades this session
                  </p>
                ) : (
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="text-oak-text-muted">
                        <th className="px-3 py-2 text-left">Time</th>
                        <th className="px-3 py-2 text-left">Pair</th>
                        <th className="px-3 py-2 text-right">Amount</th>
                        <th className="px-3 py-2 text-right">Output</th>
                        <th className="px-3 py-2 text-right">Tx</th>
                      </tr>
                    </thead>
                    <tbody>
                      <AnimatePresence mode="sync">
                        {tradeHistory.map((t) => (
                          <motion.tr
                            key={t.id}
                            layout
                            initial={{ opacity: 0, y: -8, scale: 0.96 }}
                            animate={{ opacity: 1, y: 0, scale: 1 }}
                            exit={{ opacity: 0, x: -20 }}
                            transition={{ type: "spring", stiffness: 400, damping: 30 }}
                            className="border-t border-oak-border/40 text-oak-text-secondary transition-colors hover:bg-oak-accent/5"
                          >
                            <td className="px-3 py-2 text-oak-text-muted">
                              {new Date(t.timestamp).toLocaleTimeString()}
                            </td>
                            <td className="px-3 py-2 font-mono">
                              {t.token0Symbol}/{t.token1Symbol}
                            </td>
                            <td className="px-3 py-2 text-right font-mono">
                              {t.amountIn} {t.token0Symbol}
                            </td>
                            <td className="px-3 py-2 text-right font-mono text-oak-accent">
                              {t.amountOut} {t.token1Symbol}
                            </td>
                            <td className="px-3 py-2 text-right">
                              <a
                                href={`https://arbiscan.io/tx/${t.txHash}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="font-mono text-oak-accent transition-colors hover:text-oak-accent-hover hover:underline"
                              >
                                {t.txHash.slice(0, 10)}…
                              </a>
                            </td>
                          </motion.tr>
                        ))}
                      </AnimatePresence>
                    </tbody>
                  </table>
                )}
              </>
            )}
          </div>
        </motion.div>
      </main>
    </div>
  );
}
