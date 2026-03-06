"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { useOrders, useCurrentPrice } from "@/hooks/useOrders";
import { ARBITRUM_SEPOLIA_TOKENS, TOKEN_SYMBOLS, type TokenKey } from "@/config/tokens";
import type { OrderType } from "@/lib/oakContract";
import { ORDER_TYPE_LABEL } from "@/lib/oakContract";

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

export interface TPSLOrderFormProps {
  /** Default pair for token_out (sell) / token_in (receive). */
  defaultTokenOut?: TokenKey;
  defaultTokenIn?: TokenKey;
  onPlaced?: (orderId: bigint) => void;
  className?: string;
}

export function TPSLOrderForm({
  defaultTokenOut = "WETH",
  defaultTokenIn = "USDC",
  onPlaced,
  className = "",
}: TPSLOrderFormProps) {
  const [tokenOut, setTokenOut] = useState<TokenKey>(defaultTokenOut);
  const [tokenIn, setTokenIn] = useState<TokenKey>(defaultTokenIn);
  const [amountOut, setAmountOut] = useState("");
  const [triggerPrice, setTriggerPrice] = useState("");
  const [orderType, setOrderType] = useState<OrderType>(0);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const tokenOutAddr = ARBITRUM_SEPOLIA_TOKENS[tokenOut];
  const tokenInAddr = ARBITRUM_SEPOLIA_TOKENS[tokenIn];
  const { placeOrder } = useOrders();
  const { price: currentPrice } = useCurrentPrice(tokenInAddr, tokenOutAddr);

  const handlePlace = async () => {
    const amount = amountOut.trim();
    const price = triggerPrice.trim();
    if (!amount || !price) {
      setError("Amount and trigger price required");
      return;
    }
    const amountWei = BigInt(amount);
    if (amountWei <= 0n) {
      setError("Amount must be > 0");
      return;
    }
    setError(null);
    setIsSubmitting(true);
    try {
      const txHash = await placeOrder(
        tokenInAddr,
        tokenOutAddr,
        amount,
        price,
        orderType
      );
      setAmountOut("");
      setTriggerPrice("");
      onPlaced?.(0n);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Place order failed");
    } finally {
      setIsSubmitting(false);
    }
  };

  const currentPriceStr = currentPrice !== undefined ? currentPrice.toString() : "—";

  return (
    <div
      className={`flex flex-col overflow-hidden ${className}`}
      style={{
        background: "#051005",
        border: "1px solid rgba(0, 255, 0, 0.1)",
        borderRadius: "8px",
        backdropFilter: "blur(12px)",
      }}
    >
      <div
        className="border-b px-3 py-2"
        style={{ borderColor: "rgba(0, 255, 0, 0.1)" }}
      >
        <span className="font-sans text-xs font-medium uppercase tracking-wider text-zinc-500">
          Limit / TP / SL
        </span>
      </div>
      <div className="p-3 space-y-3">
        <div className="flex gap-2">
          <select
            value={tokenOut}
            onChange={(e) => setTokenOut(e.target.value as TokenKey)}
            className="flex-1 rounded border bg-black/40 px-2 py-2 font-sans text-sm text-white"
            style={{ borderColor: "rgba(0, 255, 0, 0.2)" }}
          >
            {(Object.keys(TOKEN_SYMBOLS) as TokenKey[]).map((k) => (
              <option key={k} value={k}>{TOKEN_SYMBOLS[k]} (sell)</option>
            ))}
          </select>
          <span className="self-center text-zinc-500">→</span>
          <select
            value={tokenIn}
            onChange={(e) => setTokenIn(e.target.value as TokenKey)}
            className="flex-1 rounded border bg-black/40 px-2 py-2 font-sans text-sm text-white"
            style={{ borderColor: "rgba(0, 255, 0, 0.2)" }}
          >
            {(Object.keys(TOKEN_SYMBOLS) as TokenKey[]).map((k) => (
              <option key={k} value={k}>{TOKEN_SYMBOLS[k]} (receive)</option>
            ))}
          </select>
        </div>
        <div>
          <label className="font-sans text-xs text-zinc-500">Amount (sell, wei)</label>
          <input
            type="text"
            inputMode="decimal"
            placeholder="0"
            value={amountOut}
            onChange={(e) => setAmountOut(e.target.value.replace(/[^0-9]/g, ""))}
            className="mt-1 w-full rounded border bg-black/40 px-3 py-2 font-mono text-sm text-white"
            style={{ borderColor: "rgba(0, 255, 0, 0.15)" }}
          />
        </div>
        <div>
          <label className="font-sans text-xs text-zinc-500">Trigger price (token_in per token_out)</label>
          <input
            type="text"
            inputMode="decimal"
            placeholder="0"
            value={triggerPrice}
            onChange={(e) => setTriggerPrice(e.target.value.replace(/[^0-9.]/g, ""))}
            className="mt-1 w-full rounded border bg-black/40 px-3 py-2 font-mono text-sm text-white"
            style={{ borderColor: "rgba(0, 255, 0, 0.15)" }}
          />
          <p className="mt-1 font-sans text-[10px] text-zinc-500">Current: {currentPriceStr}</p>
        </div>
        <div className="flex gap-2">
          {([0, 1, 2] as OrderType[]).map((t) => (
            <motion.button
              key={t}
              type="button"
              whileTap={springTap}
              onClick={() => setOrderType(t)}
              className="flex-1 rounded py-2 font-sans text-xs font-medium"
              style={{
                background: orderType === t ? "rgba(34, 197, 94, 0.2)" : "rgba(255,255,255,0.04)",
                color: orderType === t ? "#22c55e" : "rgba(163, 163, 163, 0.9)",
                border: `1px solid ${orderType === t ? "rgba(34, 197, 94, 0.5)" : "rgba(0, 255, 0, 0.1)"}`,
              }}
            >
              {ORDER_TYPE_LABEL[t]}
            </motion.button>
          ))}
        </div>
        {error && (
          <p className="font-sans text-xs text-red-400">{error}</p>
        )}
        <motion.button
          type="button"
          disabled={isSubmitting || !amountOut.trim() || !triggerPrice.trim()}
          whileTap={!isSubmitting ? springTap : undefined}
          onClick={handlePlace}
          className="w-full rounded py-3 font-sans text-sm font-bold uppercase tracking-wider text-white disabled:opacity-50"
          style={{
            background: "rgba(34, 197, 94, 0.9)",
            border: "1px solid rgba(34, 197, 94, 0.5)",
          }}
        >
          {isSubmitting ? "Placing…" : "Place order"}
        </motion.button>
      </div>
    </div>
  );
}
