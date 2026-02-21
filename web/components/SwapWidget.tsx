"use client";

import { useState } from "react";
import { useAccount } from "wagmi";

export type SlippageOption = "0.1" | "0.5" | "1" | "custom";

const SLIPPAGE_PRESETS: SlippageOption[] = ["0.1", "0.5", "1"];

export interface SwapWidgetProps {
  /** Token0 symbol (input) */
  token0Symbol?: string;
  /** Token1 symbol (output) */
  token1Symbol?: string;
  /** User balance for token0 (from hooks/utils) */
  token0Balance?: string;
  /** Estimated output for token1 (from hooks/utils). Overridden by marketPrice when swapping base->quote. */
  estimatedOutput?: string;
  /** Live market price (e.g. from Binance). When set, output = amountIn * marketPrice for base->quote swap. */
  marketPrice?: string | null;
  /** Loading state for quote */
  isLoadingQuote?: boolean;
  /** Swap handler - implement via hooks */
  onSwap?: (amountIn: string, minAmountOut: string, deadline: number) => Promise<void>;
  /** Error message to display */
  error?: string | null;
}

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

  const effectiveSlippage = slippage === "custom" ? customSlippage : slippage;
  const numericSlippage = parseFloat(effectiveSlippage) || 0;

  // Use market price for output when swapping base (ETH) -> quote (USDC)
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
      const deadline = Math.floor(Date.now() / 1000) + 1200; // 20 min
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
    <div className="w-full max-w-md rounded-oak-lg border border-oak-border bg-oak-bg-card shadow-oak transition-shadow duration-oak hover:shadow-oak-glow">
      <div className="p-5 sm:p-6">
        <h2 className="mb-4 text-lg font-medium text-oak-text-primary">Swap</h2>

        {/* Token0 Input */}
        <div className="rounded-oak border border-oak-border bg-oak-bg-elevated p-4 transition-colors duration-oak hover:border-oak-border">
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
              onChange={(e) => setAmountIn(e.target.value.replace(/[^0-9.]/g, ""))}
              className="min-w-0 flex-1 bg-transparent text-2xl font-medium text-oak-text-primary outline-none placeholder:text-oak-text-muted"
            />
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={handleMax}
                className="rounded-md bg-oak-accent/15 px-2.5 py-1 text-xs font-medium text-oak-accent transition-colors hover:bg-oak-accent/25"
              >
                Max
              </button>
              <span className="rounded-md bg-oak-bg-hover px-3 py-1.5 text-sm font-medium text-oak-text-primary">
                {token0Symbol}
              </span>
            </div>
          </div>
        </div>

        {/* Swap direction indicator */}
        <div className="relative -my-1 flex justify-center">
          <div className="flex h-8 w-8 items-center justify-center rounded-full border-2 border-oak-bg-card bg-oak-bg-elevated text-oak-text-muted">
            ↓
          </div>
        </div>

        {/* Token1 Output */}
        <div className="rounded-oak border border-oak-border bg-oak-bg-elevated p-4 transition-colors duration-oak">
          <div className="flex items-center justify-between text-sm text-oak-text-secondary">
            <span>To</span>
          </div>
          <div className="mt-2 flex items-center justify-between gap-2">
            <div className="min-w-0 flex-1 text-2xl font-medium text-oak-text-primary">
              {isLoadingQuote && !marketPrice ? (
                <span className="inline-block h-8 w-24 animate-pulse rounded bg-oak-border" />
              ) : (
                displayOutput || "0.0"
              )}
            </div>
            <span className="rounded-md bg-oak-bg-hover px-3 py-1.5 text-sm font-medium text-oak-text-primary">
              {token1Symbol}
            </span>
          </div>
        </div>

        {/* Slippage */}
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="text-sm text-oak-text-secondary">Slippage</span>
          {SLIPPAGE_PRESETS.map((opt) => (
            <button
              key={opt}
              type="button"
              onClick={() => setSlippage(opt)}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                slippage === opt
                  ? "bg-oak-accent/20 text-oak-accent"
                  : "bg-oak-bg-hover text-oak-text-secondary hover:text-oak-text-primary"
              }`}
            >
              {opt}%
            </button>
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
              className="w-16 rounded-md border border-oak-border bg-oak-bg-elevated px-2 py-1 text-xs text-oak-text-primary outline-none placeholder:text-oak-text-muted focus:border-oak-accent"
            />
            <span className="text-xs text-oak-text-muted">%</span>
          </div>
        </div>

        {error && (
          <p className="mt-3 text-sm text-oak-error" role="alert">
            {error}
          </p>
        )}

        <button
          type="button"
          onClick={handleSwap}
          disabled={!isValid}
          className="mt-4 w-full rounded-oak bg-oak-accent py-3.5 font-medium text-white transition-all duration-oak hover:bg-oak-accent-hover disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-oak-accent"
        >
          {!isConnected
            ? "Connect Wallet"
            : isSwapping
              ? "Swapping…"
              : !amountIn || parseFloat(amountIn) <= 0
                ? "Enter amount"
                : parseFloat(amountIn) > parseFloat(token0Balance)
                  ? "Insufficient funds"
                  : "Swap"}
        </button>
      </div>
    </div>
  );
}
