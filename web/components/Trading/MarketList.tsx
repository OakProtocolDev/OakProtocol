"use client";

import { motion } from "framer-motion";

/** Single market row: pair, 24h change, price, neon sparkline. */
export interface MarketRow {
  pair: string;
  price: string;
  change24h: number;
  /** Sparkline points (normalized 0–1 or raw for path). */
  sparkData: number[];
}

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

function Sparkline({ data }: { data: number[] }) {
  if (data.length < 2) return null;
  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;
  const w = 64;
  const h = 24;
  const points = data.map((v, i) => {
    const x = (i / (data.length - 1)) * w;
    const y = h - ((v - min) / range) * (h - 2) - 1;
    return `${x},${y}`;
  });
  const pathD = `M ${points.join(" L ")}`;
  const isUp = data[data.length - 1] >= data[0];

  return (
    <svg width={w} height={h} className="overflow-visible" aria-hidden>
      <path
        d={pathD}
        fill="none"
        stroke={isUp ? "rgba(34, 197, 94, 0.8)" : "rgba(239, 68, 68, 0.8)"}
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        style={{ filter: "drop-shadow(0 0 4px currentColor)" }}
      />
    </svg>
  );
}

const DEFAULT_MARKETS: MarketRow[] = [
  { pair: "ETH/USDC", price: "3,842.50", change24h: 2.34, sparkData: [0.2, 0.5, 0.35, 0.6, 0.55, 0.8, 0.75, 0.9, 1] },
  { pair: "BTC/USDC", price: "97,120.00", change24h: -0.82, sparkData: [0.9, 0.7, 0.85, 0.6, 0.75, 0.5, 0.65, 0.55, 0.45] },
  { pair: "ARB/USDC", price: "1.12", change24h: 5.12, sparkData: [0.3, 0.4, 0.35, 0.5, 0.6, 0.55, 0.7, 0.85, 1] },
];

export interface MarketListProps {
  /** Currently selected pair (e.g. ETH/USDC). */
  selectedPair: string;
  onSelectPair: (pair: string) => void;
  /** Optional live price override for selected pair. */
  livePrice?: string | null;
  className?: string;
}

export function MarketList({
  selectedPair,
  onSelectPair,
  livePrice = null,
  className = "",
}: MarketListProps) {
  const markets = DEFAULT_MARKETS.map((m) =>
    m.pair === selectedPair && livePrice ? { ...m, price: livePrice } : m
  );

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
        className="border-b px-3 py-2"
        style={{ borderColor: "rgba(0, 255, 0, 0.1)" }}
      >
        <span className="font-sans text-xs font-medium uppercase tracking-wider text-zinc-500">
          Market Watch
        </span>
      </div>
      <div className="flex flex-col">
        {markets.map((row) => {
          const isSelected = row.pair === selectedPair;
          return (
            <motion.button
              key={row.pair}
              type="button"
              onClick={() => onSelectPair(row.pair)}
              whileTap={springTap}
              className="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors"
              style={{
                background: isSelected ? "rgba(0, 255, 0, 0.06)" : "transparent",
                borderLeft: isSelected ? "2px solid rgba(34, 197, 94, 0.6)" : "2px solid transparent",
              }}
            >
              <div className="min-w-0 flex-1">
                <div className="font-sans text-sm font-medium text-white">
                  {row.pair}
                </div>
                <div className="mt-0.5 flex items-center gap-2">
                  <span className="font-mono text-xs text-zinc-400">
                    {row.price}
                  </span>
                  <span
                    className={`font-mono text-xs ${
                      row.change24h >= 0 ? "text-emerald-400" : "text-red-400"
                    }`}
                  >
                    {row.change24h >= 0 ? "+" : ""}
                    {row.change24h}%
                  </span>
                </div>
              </div>
              <Sparkline data={row.sparkData} />
            </motion.button>
          );
        })}
      </div>
    </div>
  );
}
