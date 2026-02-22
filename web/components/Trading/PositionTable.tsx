"use client";

import { motion } from "framer-motion";
import type { TradeRecord } from "@/store/useTradeStore";

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

export type PositionTableTab = "positions" | "orders" | "trades";

export interface PositionRow {
  symbol: string;
  side: string;
  size: string;
  entry: string;
  pnl: string;
}

export interface OrderRow {
  id: string;
  type: string;
  pair: string;
  amount: string;
  status: string;
}

export interface PositionTableProps {
  activeTab: PositionTableTab;
  onTabChange: (tab: PositionTableTab) => void;
  positions: PositionRow[];
  orders: OrderRow[];
  trades: TradeRecord[];
  className?: string;
}

export function PositionTable({
  activeTab,
  onTabChange,
  positions,
  orders,
  trades,
  className = "",
}: PositionTableProps) {
  const tabs: { id: PositionTableTab; label: string }[] = [
    { id: "positions", label: "Open Positions" },
    { id: "orders", label: "Orders History" },
    { id: "trades", label: "Trade Logs" },
  ];

  return (
    <div
      className={`flex flex-col overflow-hidden ${className}`}
      style={{
        background: "#051005",
        border: "1px solid rgba(0, 255, 0, 0.1)",
        borderRadius: "8px",
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
      }}
    >
      <div
        className="flex border-b"
        style={{ borderColor: "rgba(0, 255, 0, 0.1)" }}
      >
        {tabs.map(({ id, label }) => (
          <motion.button
            key={id}
            type="button"
            onClick={() => onTabChange(id)}
            whileTap={springTap}
            className="relative px-4 py-2.5 font-sans text-sm font-medium transition-colors"
            style={{
              color: activeTab === id ? "#22c55e" : "rgba(163, 163, 163, 0.9)",
            }}
          >
            {label}
            {activeTab === id && (
              <motion.span
                layoutId="position-table-tab"
                className="absolute inset-x-0 bottom-0 h-0.5 bg-emerald-500"
                style={{ background: "rgba(34, 197, 94, 0.9)" }}
                transition={{ type: "spring", stiffness: 400, damping: 30 }}
              />
            )}
          </motion.button>
        ))}
      </div>

      <div className="min-h-[140px] overflow-x-auto">
        {activeTab === "positions" && (
          <>
            {positions.length === 0 ? (
              <p className="px-4 py-8 font-sans text-center text-sm text-zinc-500">
                No open positions
              </p>
            ) : (
              <table className="w-full font-sans text-xs">
                <thead>
                  <tr className="text-zinc-500">
                    <th className="px-3 py-2 text-left font-medium">Symbol</th>
                    <th className="px-3 py-2 text-left font-medium">Side</th>
                    <th className="px-3 py-2 text-right font-medium">Size</th>
                    <th className="px-3 py-2 text-right font-medium">Entry</th>
                    <th className="px-3 py-2 text-right font-medium">PnL</th>
                  </tr>
                </thead>
                <tbody>
                  {positions.map((p, i) => (
                    <tr
                      key={i}
                      className="border-t border-white/5 text-zinc-300 hover:bg-white/5"
                    >
                      <td className="px-3 py-2 font-mono">{p.symbol}</td>
                      <td className="px-3 py-2">{p.side}</td>
                      <td className="px-3 py-2 text-right font-mono">{p.size}</td>
                      <td className="px-3 py-2 text-right font-mono">{p.entry}</td>
                      <td className="px-3 py-2 text-right font-mono text-emerald-400">{p.pnl}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}

        {activeTab === "orders" && (
          <table className="w-full font-sans text-xs">
            <thead>
              <tr className="text-zinc-500">
                <th className="px-3 py-2 text-left font-medium">Type</th>
                <th className="px-3 py-2 text-left font-medium">Pair</th>
                <th className="px-3 py-2 text-right font-medium">Amount</th>
                <th className="px-3 py-2 text-right font-medium">Status</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((o) => (
                <tr
                  key={o.id}
                  className="border-t border-white/5 text-zinc-300 hover:bg-white/5"
                >
                  <td className="px-3 py-2">{o.type}</td>
                  <td className="px-3 py-2 font-mono">{o.pair}</td>
                  <td className="px-3 py-2 text-right font-mono">{o.amount}</td>
                  <td className="px-3 py-2 text-right">
                    <span className={o.status === "Filled" ? "text-emerald-400" : "text-zinc-500"}>
                      {o.status}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {activeTab === "trades" && (
          <>
            {trades.length === 0 ? (
              <p className="px-4 py-8 font-sans text-center text-sm text-zinc-500">
                No trades this session
              </p>
            ) : (
              <table className="w-full font-sans text-xs">
                <thead>
                  <tr className="text-zinc-500">
                    <th className="px-3 py-2 text-left font-medium">Time</th>
                    <th className="px-3 py-2 text-left font-medium">Pair</th>
                    <th className="px-3 py-2 text-right font-medium">Amount</th>
                    <th className="px-3 py-2 text-right font-medium">Output</th>
                    <th className="px-3 py-2 text-right font-medium">Tx</th>
                  </tr>
                </thead>
                <tbody>
                  {trades.map((t) => (
                    <tr
                      key={t.id}
                      className="border-t border-white/5 text-zinc-300 hover:bg-white/5"
                    >
                      <td className="px-3 py-2 font-mono text-zinc-500">
                        {new Date(t.timestamp).toLocaleTimeString()}
                      </td>
                      <td className="px-3 py-2 font-mono">
                        {t.token0Symbol}/{t.token1Symbol}
                        {t.isDemo && (
                          <span
                            className="ml-1.5 rounded px-1.5 py-0.5 font-sans text-[10px] font-semibold"
                            style={{
                              background: "rgba(245, 158, 11, 0.25)",
                              color: "#fbbf24",
                            }}
                          >
                            DEMO
                          </span>
                        )}
                      </td>
                      <td className="px-3 py-2 text-right font-mono">
                        {t.amountIn} {t.token0Symbol}
                      </td>
                      <td className="px-3 py-2 text-right font-mono text-emerald-400">
                        {t.amountOut} {t.token1Symbol}
                      </td>
                      <td className="px-3 py-2 text-right">
                        <a
                          href={`https://arbiscan.io/tx/${t.txHash}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="font-mono text-emerald-400 hover:underline"
                        >
                          {t.txHash.slice(0, 10)}…
                        </a>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}
      </div>
    </div>
  );
}
