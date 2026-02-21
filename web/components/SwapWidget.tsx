"use client";

import { useState, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import { useAccount } from "wagmi";

export type SlippageOption = "0.1" | "0.5" | "1" | "custom";

const SLIPPAGE_PRESETS: SlippageOption[] = ["0.1", "0.5", "1"];

export interface SwapWidgetProps {
  token0Symbol?: string;
  token1Symbol?: string;
  token0Balance?: string;
  estimatedOutput?: string;
  marketPrice?: string | null;
  isLoadingQuote?: boolean;
  onSwap?: (amountIn: string, minAmountOut: string, deadline: number) => Promise<void>;
  error?: string | null;
}

const springTap = {
  scale: 0.98,
  transition: { type: "spring", stiffness: 500, damping: 30 },
};

export function SwapWidget({
  token0Symbol = "TOKEN0",
  token1Symbol = "TOKEN1",
  token0Balance = "0",
  estimatedOutput = "0",
  marketPrice = null,
  isLoadingQuote = false,
  onSwap,
  error = null,
}: SwapWidgetProps) {
  const { isConnected } = useAccount();
  const [amountIn, setAmountIn] = useState("");
  const [slippage, setSlippage] = useState<SlippageOption>("0.5");
  const [customSlippage, setCustomSlippage] = useState("");
  const [isSwapping, setIsSwapping] = useState(false);
  const [inputFocused, setInputFocused] = useState(false);
  const inputRef = useRef<HTMLDivElement>(null);

  const effectiveSlippage = slippage === "custom" ? customSlippage : slippage;
  const numericSlippage = parseFloat(effectiveSlippage) || 0;

  const computedOutput =
    marketPrice && amountIn && parseFloat(amountIn) > 0
      ? (parseFloat(amountIn) * parseFloat(marketPrice)).toFixed(6)
      : estimatedOutput;
  const displayOutput = marketPrice ? computedOutput : estimatedOutput;
  const minAmountOut =
    displayOutput && amountIn
      ? (parseFloat(displayOutput) * (1 - numericSlippage / 100)).toFixed(6)
      : "0";

  const handleMax = () => {
    setAmountIn(token0Balance);
  };

  const handleSwap = async () => {
    if (!onSwap || !amountIn || parseFloat(amountIn) <= 0) return;
    setIsSwapping(true);
    try {
      const deadline = Math.floor(Date.now() / 1000) + 1200;
      await onSwap(amountIn, minAmountOut, deadline);
      setAmountIn("");
    } catch (e) {
      console.error(e);
    } finally {
      setIsSwapping(false);
    }
  };

  const isValid =
    isConnected &&
    amountIn &&
    parseFloat(amountIn) > 0 &&
    parseFloat(amountIn) <= parseFloat(token0Balance) &&
    !isSwapping;

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.25, 0.46, 0.45, 0.94] }}
      className="glass-card w-full max-w-md overflow-hidden"
      style={{
        boxShadow:
          "0 8px 32px rgba(0, 0, 0, 0.4), 0 2px 8px rgba(0, 0, 0, 0.2), inset 0 1px 0 rgba(255, 255, 255, 0.02)",
      }}
    >
      <div className="p-5 sm:p-6">
        <h2 className="mb-4 text-lg font-medium text-oak-text-primary">Swap</h2>

        {/* Token0 Input - Tactile with focus glow */}
        <motion.div
          ref={inputRef}
          className="relative rounded-oak overflow-hidden p-[1px] transition-all duration-300"
          animate={{
            boxShadow: inputFocused
              ? "0 0 0 2px rgba(34, 197, 94, 0.4), 0 0 20px rgba(34, 197, 94, 0.1)"
              : "0 0 0 1px rgba(34, 197, 94, 0.1)",
          }}
          transition={{ duration: 0.3 }}
        >
          <div className="rounded-oak bg-[rgba(12,18,16,0.9)] p-4 backdrop-blur-sm">
            <div className="flex items-center justify-between text-sm text-oak-text-secondary">
              <span>From</span>
              <span>Balance: {token0Balance}</span>
            </div>
            <div className="mt-2 flex items-center justify-between gap-2">
              <input
                type="text"
                inputMode="decimal"
                placeholder="0.0"
                value={amountIn}
                onFocus={() => setInputFocused(true)}
                onBlur={() => setInputFocused(false)}
                onChange={(e) =>
                  setAmountIn(e.target.value.replace(/[^0-9.]/g, ""))
                }
                className="min-w-0 flex-1 bg-transparent text-2xl font-medium text-oak-text-primary outline-none placeholder:text-oak-text-muted"
              />
              <div className="flex items-center gap-2">
                <motion.button
                  type="button"
                  onClick={handleMax}
                  whileTap={springTap}
                  className="rounded-md bg-oak-accent/15 px-2.5 py-1 text-xs font-medium text-oak-accent transition-colors hover:bg-oak-accent/25"
                >
                  Max
                </motion.button>
                <span className="rounded-md bg-oak-bg-hover/80 px-3 py-1.5 text-sm font-medium text-oak-text-primary backdrop-blur-sm">
                  {token0Symbol}
                </span>
              </div>
            </div>
          </div>
        </motion.div>

        {/* Swap direction indicator */}
        <div className="relative -my-1 flex justify-center">
          <div className="flex h-8 w-8 items-center justify-center rounded-full border border-oak-border bg-oak-bg-elevated/80 text-oak-text-muted backdrop-blur-sm">
            ↓
          </div>
        </div>

        {/* Token1 Output */}
        <div className="rounded-oak border border-oak-border/60 bg-[rgba(12,18,16,0.6)] p-4 backdrop-blur-sm">
          <div className="flex items-center justify-between text-sm text-oak-text-secondary">
            <span>To</span>
          </div>
          <div className="mt-2 flex items-center justify-between gap-2">
            <div className="min-w-0 flex-1 text-2xl font-medium text-oak-text-primary">
              {isLoadingQuote && !marketPrice ? (
                <span className="inline-block h-8 w-24 animate-pulse rounded bg-oak-border/50" />
              ) : (
                displayOutput || "0.0"
              )}
            </div>
            <span className="rounded-md bg-oak-bg-hover/80 px-3 py-1.5 text-sm font-medium text-oak-text-primary backdrop-blur-sm">
              {token1Symbol}
            </span>
          </div>
        </div>

        {/* Network Fee - Stylus with neon glow */}
        <div className="mt-3 flex items-center justify-between text-sm">
          <span className="text-oak-text-secondary">Network Fee</span>
          <div className="flex items-center gap-2">
            <span className="font-mono text-oak-text-primary">~0.00004 ETH</span>
            <span
              className="rounded px-2 py-0.5 text-xs font-medium text-oak-accent"
              style={{
                background: "rgba(34, 197, 94, 0.15)",
                boxShadow:
                  "0 0 12px rgba(34, 197, 94, 0.25), 0 0 24px rgba(34, 197, 94, 0.08)",
              }}
            >
              Stylus Boosted
            </span>
          </div>
        </div>

        {/* Slippage - Spring buttons */}
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="text-sm text-oak-text-secondary">Slippage</span>
          {SLIPPAGE_PRESETS.map((opt) => (
            <motion.button
              key={opt}
              type="button"
              onClick={() => setSlippage(opt)}
              whileTap={springTap}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                slippage === opt
                  ? "bg-oak-accent/20 text-oak-accent"
                  : "bg-oak-bg-hover/80 text-oak-text-secondary hover:text-oak-text-primary backdrop-blur-sm"
              }`}
            >
              {opt}%
            </motion.button>
          ))}
          <div className="flex items-center gap-1">
            <input
              type="text"
              inputMode="decimal"
              placeholder="Custom"
              value={slippage === "custom" ? customSlippage : ""}
              onFocus={() => setSlippage("custom")}
              onChange={(e) => {
                setSlippage("custom");
                setCustomSlippage(e.target.value.replace(/[^0-9.]/g, ""));
              }}
              className="glass-input w-16 px-2 py-1 text-xs text-oak-text-primary placeholder:text-oak-text-muted"
            />
            <span className="text-xs text-oak-text-muted">%</span>
          </div>
        </div>

        {error && (
          <p className="mt-3 text-sm text-oak-error" role="alert">
            {error}
          </p>
        )}

        {/* Swap button - Shimmer effect + spring */}
        <ShimmerSwapButton
          isValid={!!isValid}
          isSwapping={isSwapping}
          amountIn={amountIn}
          token0Balance={token0Balance}
          isConnected={!!isConnected}
          onSwap={handleSwap}
        />
      </div>
    </motion.div>
  );
}

function ShimmerSwapButton({
  isValid,
  isSwapping,
  amountIn,
  token0Balance,
  isConnected,
  onSwap,
}: {
  isValid: boolean;
  isSwapping: boolean;
  amountIn: string;
  token0Balance: string;
  isConnected: boolean;
  onSwap: () => void;
}) {
  const label = !isConnected
    ? "Connect Wallet"
    : isSwapping
      ? "Swapping…"
      : !amountIn || parseFloat(amountIn) <= 0
        ? "Enter amount"
        : parseFloat(amountIn) > parseFloat(token0Balance)
          ? "Insufficient funds"
          : "Swap";

  return (
    <motion.button
      type="button"
      onClick={onSwap}
      disabled={!isValid}
      whileTap={isValid ? springTap : undefined}
      className="relative mt-4 w-full overflow-hidden rounded-oak py-3.5 font-medium text-white transition-all duration-300 disabled:cursor-not-allowed disabled:opacity-50"
      style={{
        background: isValid
          ? "linear-gradient(135deg, #22c55e 0%, #16a34a 100%)"
          : "rgb(34, 197, 94, 0.4)",
      }}
    >
      {/* Shimmer overlay - passes every ~3s */}
      {isValid && (
        <span className="absolute inset-0 overflow-hidden">
          <span className="absolute inset-y-0 w-[40%] bg-gradient-to-r from-transparent via-white/30 to-transparent animate-shimmer" />
        </span>
      )}
      <span className="relative z-10">{label}</span>
    </motion.button>
  );
}
