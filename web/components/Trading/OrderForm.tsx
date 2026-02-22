"use client";

import { useState } from "react";
import { motion } from "framer-motion";

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

export type OrderSide = "buy" | "sell";
export type OrderType = "market" | "limit";

export interface OrderFormProps {
  /** ETH balance string (e.g. from getDisplayBalance). */
  balanceEth: string;
  /** Current market price for display. */
  marketPrice: string | null;
  /** Demo mode: primary button and accent become amber/gold. */
  isDemoMode: boolean;
  /** When true, allow execute without wallet. */
  canExecute: boolean;
  /** Executing in progress. */
  isExecuting: boolean;
  /** Callback: (amountIn, side, orderType). Page runs 4-step and updates store. */
  onExecute: (amountIn: string, side: OrderSide, orderType: OrderType) => Promise<void>;
  className?: string;
}

export function OrderForm({
  balanceEth,
  marketPrice,
  isDemoMode,
  canExecute,
  isExecuting,
  onExecute,
  className = "",
}: OrderFormProps) {
  const [side, setSide] = useState<OrderSide>("buy");
  const [orderType, setOrderType] = useState<OrderType>("market");
  const [amount, setAmount] = useState("");
  const [leverage, setLeverage] = useState(1);

  const amountNum = parseFloat(amount) || 0;
  const balanceNum = parseFloat(balanceEth) || 0;
  const validAmount = amountNum > 0 && amountNum <= balanceNum;
  const isValid = canExecute && validAmount && !isExecuting;

  const primaryColor = isDemoMode
    ? { bg: "rgba(245, 158, 11, 0.9)", hover: "rgba(251, 191, 36, 0.95)", glow: "0 0 24px rgba(245, 158, 11, 0.4)" }
    : { bg: "rgba(34, 197, 94, 0.9)", hover: "rgba(22, 163, 74, 0.95)", glow: "0 0 24px rgba(34, 197, 94, 0.3)" };

  const handleExecute = async () => {
    if (!isValid || !amount.trim()) return;
    await onExecute(amount.trim(), side, orderType);
    setAmount("");
  };

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
          Execution Panel
        </span>
      </div>

      <div className="p-3">
        {/* Buy / Sell */}
        <div className="grid grid-cols-2 gap-2">
          <motion.button
            type="button"
            whileTap={springTap}
            onClick={() => setSide("buy")}
            className="rounded py-2.5 font-sans text-sm font-semibold transition-all"
            style={{
              background: side === "buy" ? "rgba(34, 197, 94, 0.2)" : "rgba(255,255,255,0.04)",
              color: side === "buy" ? "#22c55e" : "rgba(163, 163, 163, 0.9)",
              border: `1px solid ${side === "buy" ? "rgba(34, 197, 94, 0.5)" : "rgba(0, 255, 0, 0.1)"}`,
              boxShadow: side === "buy" ? "0 0 12px rgba(34, 197, 94, 0.2)" : "none",
            }}
          >
            Buy
          </motion.button>
          <motion.button
            type="button"
            whileTap={springTap}
            onClick={() => setSide("sell")}
            className="rounded py-2.5 font-sans text-sm font-semibold transition-all"
            style={{
              background: side === "sell" ? "rgba(239, 68, 68, 0.2)" : "rgba(255,255,255,0.04)",
              color: side === "sell" ? "#ef4444" : "rgba(163, 163, 163, 0.9)",
              border: `1px solid ${side === "sell" ? "rgba(239, 68, 68, 0.5)" : "rgba(0, 255, 0, 0.1)"}`,
              boxShadow: side === "sell" ? "0 0 12px rgba(239, 68, 68, 0.2)" : "none",
            }}
          >
            Sell
          </motion.button>
        </div>

        {/* Market / Limit */}
        <div className="mt-3 flex gap-2 rounded border border-white/10 bg-black/20 p-1">
          {(["market", "limit"] as const).map((type) => (
            <motion.button
              key={type}
              type="button"
              whileTap={springTap}
              onClick={() => setOrderType(type)}
              className="flex-1 rounded py-1.5 font-sans text-xs font-medium capitalize"
              style={{
                background: orderType === type ? "rgba(255,255,255,0.08)" : "transparent",
                color: orderType === type ? "#fff" : "rgba(163, 163, 163, 0.9)",
              }}
            >
              {type}
            </motion.button>
          ))}
        </div>

        {/* Amount + Balance */}
        <div className="mt-3">
          <div className="flex justify-between font-sans text-xs text-zinc-500">
            <span>Amount (ETH)</span>
            <span className="font-mono text-zinc-400">Balance: {balanceEth} ETH</span>
          </div>
          <input
            type="text"
            inputMode="decimal"
            placeholder="0.0"
            value={amount}
            onChange={(e) => setAmount(e.target.value.replace(/[^0-9.]/g, ""))}
            className="mt-1 w-full rounded border bg-black/40 px-3 py-2.5 font-mono text-sm text-white placeholder:text-zinc-600 outline-none transition-colors focus:border-emerald-500/50"
            style={{ borderColor: "rgba(0, 255, 0, 0.15)" }}
          />
        </div>

        {/* Leverage 1x–50x glassmorphism */}
        <div
          className="mt-3 rounded border border-white/10 p-3"
          style={{
            background: "rgba(255, 255, 255, 0.03)",
            backdropFilter: "blur(12px)",
          }}
        >
          <div className="flex items-center justify-between font-sans text-xs text-zinc-500">
            <span>Leverage</span>
            <span className="font-mono font-medium text-white">{leverage}x</span>
          </div>
          <input
            type="range"
            min={1}
            max={50}
            value={leverage}
            onChange={(e) => setLeverage(Number(e.target.value))}
            className="mt-2 h-2 w-full appearance-none rounded-full bg-white/10 accent-emerald-500"
            style={{
              background: `linear-gradient(to right, rgba(34, 197, 94, 0.5) 0%, rgba(34, 197, 94, 0.5) ${((leverage - 1) / 49) * 100}%, rgba(255,255,255,0.1) ${((leverage - 1) / 49) * 100}%, rgba(255,255,255,0.1) 100%)`,
            }}
          />
        </div>

        {/* Gas: Stylus Boosted + fire icon */}
        <div className="mt-3 flex items-center justify-between font-sans text-xs">
          <span className="text-zinc-500">Est. gas</span>
          <span className="inline-flex items-center gap-1.5 font-mono text-zinc-400">
            ~0.00004 ETH
            <span
              className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-sans text-[10px] font-medium"
              style={{
                background: "rgba(34, 197, 94, 0.15)",
                color: "#22c55e",
                boxShadow: "0 0 8px rgba(34, 197, 94, 0.2)",
              }}
            >
              <svg className="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M12.395 2.553a1 1 0 00-1.45-.385c-.345.23-.614.558-.822.88-.214.33-.403.713-.57 1.116-.334.804-.614 1.768-.84 2.734a31.365 31.365 0 00-.613 3.58 2.64 2.64 0 01-.945-1.067c-.328-.68-.398-1.534-.398-2.654A1 1 0 005.05 6.05 6.981 6.981 0 003 11v7a1 1 0 001 1h12a1 1 0 001-1v-7a6.981 6.981 0 00-2.05-4.95A1 1 0 0014 6z" clipRule="evenodd" />
              </svg>
              Stylus Boosted
            </span>
          </span>
        </div>

        {/* EXECUTE SECURE TRADE */}
        <motion.button
          type="button"
          disabled={!isValid}
          whileTap={isValid ? springTap : undefined}
          onClick={handleExecute}
          className="mt-4 w-full rounded py-3.5 font-sans text-sm font-bold uppercase tracking-wider text-white transition-all disabled:cursor-not-allowed disabled:opacity-50"
          style={{
            background: isValid ? primaryColor.bg : "rgba(255,255,255,0.1)",
            boxShadow: isValid ? primaryColor.glow : "none",
            border: `1px solid ${isValid ? (isDemoMode ? "rgba(245, 158, 11, 0.5)" : "rgba(34, 197, 94, 0.5)") : "rgba(255,255,255,0.1)"}`,
          }}
        >
          {isExecuting ? "Executing…" : "Execute Secure Trade"}
        </motion.button>
      </div>
    </div>
  );
}
