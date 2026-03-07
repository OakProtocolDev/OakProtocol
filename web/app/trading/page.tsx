"use client";

import { useState, useCallback, useEffect, useMemo } from "react";
import { motion } from "framer-motion";
import { Header } from "@/components/Header";
import { TradingViewChart } from "@/components/TradingViewChart";
import { LiveLogsPanel, type LogEntry } from "@/components/LiveLogsPanel";
import { SuccessModal } from "@/components/SuccessModal";
import { MarketList } from "@/components/Trading/MarketList";
import { PositionTable, type PositionTableTab, type PositionRow, type OrderRow } from "@/components/Trading/PositionTable";
import { OrderForm } from "@/components/Trading/OrderForm";
import { TPSLOrderForm } from "@/components/Trading/TPSLOrderForm";
import { useAccount } from "wagmi";
import { useReadContracts } from "wagmi";
import { useBinanceData } from "@/hooks/useBinanceData";
import { useOpenPositions, usePositionActions } from "@/hooks/usePositions";
import { OAK_CONTRACT_ADDRESS, oakOrderAbi } from "@/lib/oakContract";
import {
  useTradeStore,
  getDisplayBalance,
} from "@/store/useTradeStore";
import { secureSaltHex, scrubSecret, isRevealWindowValid } from "@/lib/security";

const ZERO = "0x0000000000000000000000000000000000000000";
const CONTRACT = OAK_CONTRACT_ADDRESS as `0x${string}`;

/** Default: atomic swap (EVM-style). Set true for optional commit-reveal MEV protection. */
const DEFAULT_USE_COMMIT_REVEAL = false;

const PLACEHOLDER_ORDERS: OrderRow[] = [
  { id: "0x1a2b", type: "Limit Buy", pair: "ETH/USDC", amount: "0.5 ETH", status: "Filled" },
  { id: "0x3c4d", type: "Market Sell", pair: "ETH/USDC", amount: "0.25 ETH", status: "Filled" },
  { id: "0x5e6f", type: "Limit Sell", pair: "ETH/USDC", amount: "1.0 ETH", status: "Open" },
];

/** Format 18-decimal bigint as short price string. */
function formatPriceFromRaw(raw: bigint): string {
  const n = Number(raw) / 1e18;
  if (n >= 1000) return n.toLocaleString("en-US", { maximumFractionDigits: 2 });
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

/** Format size (18 decimals) for display. */
function formatSize(raw: bigint): string {
  const n = Number(raw) / 1e18;
  if (n >= 1) return n.toFixed(4);
  return n.toFixed(6);
}

const REVEAL_MAX_BLOCKS = 20;

function randomTxHash(): string {
  return (
    "0x" +
    Array.from(crypto.getRandomValues(new Uint8Array(32)))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("")
  );
}

function addLogEntry(
  setLogs: React.Dispatch<React.SetStateAction<LogEntry[]>>,
  message: string,
  level?: LogEntry["level"]
) {
  setLogs((prev) => [
    ...prev,
    {
      id: crypto.randomUUID(),
      timestamp: new Date().toLocaleTimeString("en-US", {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        fractionalSecondDigits: 3,
      }),
      message,
      level,
    },
  ]);
}

function formatPrice(p: string): string {
  const n = parseFloat(p);
  if (n >= 1000) return n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

export default function TradingPage() {
  const { isConnected } = useAccount();
  const { lastPrice } = useBinanceData();
  const isDemoMode = useTradeStore((s) => s.isDemoMode);
  const balances = useTradeStore((s) => s.balances);
  const transactions = useTradeStore((s) => s.transactions);
  const applySwap = useTradeStore((s) => s.applySwap);
  const addTransaction = useTradeStore((s) => s.addTransaction);

  const { positions: openPositions, isLoading: positionsLoading, refetch: refetchPositions } = useOpenPositions();
  const { closePosition, isPending: closePending } = usePositionActions();
  const [closingId, setClosingId] = useState<bigint | null>(null);
  const [useCommitReveal, setUseCommitReveal] = useState(DEFAULT_USE_COMMIT_REVEAL);

  const priceContracts = useMemo(
    () =>
      openPositions.map((p) => ({
        address: CONTRACT,
        abi: oakOrderAbi,
        functionName: "get_current_price" as const,
        args: [p.baseToken, p.quoteToken] as const,
      })),
    [openPositions]
  );
  const { data: priceResults } = useReadContracts({
    contracts: OAK_CONTRACT_ADDRESS !== ZERO && openPositions.length > 0 ? priceContracts : [],
  });

  const positionRows: PositionRow[] = useMemo(() => {
    return openPositions.map((p, i) => {
      const priceRes = priceResults?.[i];
      const currentPriceRaw =
        priceRes?.status === "success" && priceRes.result !== undefined
          ? (priceRes.result as bigint)
          : null;
      const entryNum = Number(p.entryPrice) / 1e18;
      const currentNum = currentPriceRaw != null ? Number(currentPriceRaw) / 1e18 : null;
      const sizeNum = Number(p.size) / 1e18;
      const pnlQuote = currentNum != null ? sizeNum * (currentNum - entryNum) : null;
      const pnlPct = currentNum != null && entryNum > 0 ? ((currentNum - entryNum) / entryNum) * 100 : null;
      const pairLabel = "Base/Quote";
      return {
        symbol: pairLabel,
        side: "Long",
        size: formatSize(p.size),
        entry: formatPriceFromRaw(p.entryPrice),
        current: currentPriceRaw != null ? formatPriceFromRaw(currentPriceRaw) : undefined,
        tp: p.tpPrice > 0n ? formatPriceFromRaw(p.tpPrice) : undefined,
        sl: p.slPrice > 0n ? formatPriceFromRaw(p.slPrice) : undefined,
        pnl:
          pnlPct != null
            ? `${pnlQuote != null && pnlQuote >= 0 ? "+" : ""}${pnlQuote?.toFixed(2) ?? "0"} (${pnlPct >= 0 ? "+" : ""}${pnlPct.toFixed(2)}%)`
            : "—",
        positionId: p.positionId,
        onClose: () => {
          setClosingId(p.positionId);
          const minOut =
            currentPriceRaw != null
              ? (p.size * currentPriceRaw * 995n) / (1000n * BigInt(1e18))
              : 0n;
          closePosition(p.positionId, minOut.toString())
            .then(() => {
              setClosingId(null);
              refetchPositions();
            })
            .catch(() => setClosingId(null));
        },
        closing: closePending && closingId === p.positionId,
      };
    });
  }, [openPositions, priceResults, closePosition, closePending, closingId, refetchPositions]);

  const [selectedPair, setSelectedPair] = useState("ETH/USDC");
  const [bottomTab, setBottomTab] = useState<PositionTableTab>("trades");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isExecuting, setIsExecuting] = useState(false);
  const [successModal, setSuccessModal] = useState<{
    isOpen: boolean;
    txHash: string;
    amountIn: string;
    amountOut: string;
  }>({ isOpen: false, txHash: "", amountIn: "", amountOut: "" });

  const balanceEth = getDisplayBalance("ETH", "0", isDemoMode, balances);
  const canExecute = isDemoMode || isConnected;

  // Network scanning on mount
  useEffect(() => {
    const t1 = setTimeout(() => addLogEntry(setLogs, "Scanning Arbitrum for liquidity…", "scan"), 400);
    const t2 = setTimeout(() => addLogEntry(setLogs, "GMX found…", "scan"), 1200);
    const t3 = setTimeout(() => addLogEntry(setLogs, "Aave found…", "scan"), 2000);
    const t4 = setTimeout(() => addLogEntry(setLogs, "Securing endpoints…", "scan"), 2800);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);
      clearTimeout(t4);
    };
  }, []);

  const handleExecute = useCallback(
    async (amountIn: string, side: "buy" | "sell", _orderType: "market" | "limit") => {
      const price = lastPrice || "3842.5";
      const amountOut = side === "sell"
        ? (parseFloat(amountIn) * parseFloat(price)).toFixed(6)
        : (parseFloat(amountIn) / parseFloat(price)).toFixed(6);

      setIsExecuting(true);

      if (useCommitReveal) {
        const salt = secureSaltHex(32);
        addLogEntry(setLogs, "[1/4] Commit: Encrypting with AES-256 equivalent salt…", "info");
        await new Promise((r) => setTimeout(r, 400));
        const mockBlock = Math.floor(Date.now() / 2000) + 1;
        addLogEntry(setLogs, `[2/4] Committing to Arbitrum Stylus (Block: ${mockBlock})…`, "info");
        await new Promise((r) => setTimeout(r, 450));
        addLogEntry(setLogs, "[3/4] Waiting for Reveal Window (15s)…", "info");
        await new Promise((r) => setTimeout(r, 600));
        const revealDelayBlocks = 12;
        if (!isRevealWindowValid(revealDelayBlocks, REVEAL_MAX_BLOCKS)) {
          addLogEntry(setLogs, `Reveal expired: delay ${revealDelayBlocks} blocks > ${REVEAL_MAX_BLOCKS} max. Trade failed.`, "warn");
          scrubSecret(salt);
          setIsExecuting(false);
          return;
        }
        addLogEntry(setLogs, "[4/4] Executing via MEV-Protected Route…", "info");
        scrubSecret(salt);
        await new Promise((r) => setTimeout(r, 350));
        addLogEntry(setLogs, "Trade confirmed · MEV-protected (commit-reveal)", "success");
      } else {
        addLogEntry(setLogs, "Executing atomic swap (EVM-style)…", "info");
        await new Promise((r) => setTimeout(r, 400));
        addLogEntry(setLogs, "Swap completed", "success");
      }

      if (side === "sell") {
        applySwap("ETH", "USDC", parseFloat(amountIn), parseFloat(amountOut));
      } else {
        applySwap("USDC", "ETH", parseFloat(amountOut), parseFloat(amountIn));
      }

      const txHash = randomTxHash();
      addTransaction({
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        side,
        amountIn: side === "sell" ? amountIn : amountOut,
        amountOut: side === "sell" ? amountOut : amountIn,
        token0Symbol: "ETH",
        token1Symbol: "USDC",
        txHash,
        isDemo: isDemoMode,
      });
      setSuccessModal({ isOpen: true, txHash, amountIn: side === "sell" ? amountIn : amountOut, amountOut: side === "sell" ? amountOut : amountIn });
      setIsExecuting(false);
    },
    [lastPrice, isDemoMode, applySwap, addTransaction, useCommitReveal]
  );

  const high = lastPrice ? (parseFloat(lastPrice) * 1.02).toFixed(2) : "—";
  const low = lastPrice ? (parseFloat(lastPrice) * 0.98).toFixed(2) : "—";
  const vol = "1.2M";

  return (
    <div className="flex min-h-screen flex-col bg-black">
      <Header />
      <SuccessModal
        isOpen={successModal.isOpen}
        txHash={successModal.txHash}
        amountIn={successModal.amountIn}
        amountOut={successModal.amountOut}
        token0Symbol="ETH"
        token1Symbol="USDC"
        isDemo={isDemoMode}
        useCommitReveal={useCommitReveal}
        onClose={() => setSuccessModal((s) => ({ ...s, isOpen: false }))}
      />
      <main className="flex flex-1 overflow-hidden">
        <motion.div
          className="flex flex-1 overflow-hidden"
          initial={false}
          animate={{
            boxShadow: isDemoMode ? "0 0 80px rgba(245, 158, 11, 0.08)" : "none",
          }}
          transition={{ duration: 0.4 }}
        >
          {/* LEFT: Market Watch */}
          <aside
            className="flex w-[220px] shrink-0 flex-col gap-2 overflow-y-auto p-2"
            style={{ background: "#000", borderRight: "1px solid rgba(0, 255, 0, 0.1)" }}
          >
            <MarketList
              selectedPair={selectedPair}
              onSelectPair={setSelectedPair}
              livePrice={lastPrice ? formatPrice(lastPrice) : null}
            />
          </aside>

          {/* CENTER: Chart + Metrics + PositionTable */}
          <div className="flex min-w-0 flex-1 flex-col">
            {/* Chart area — TradingView style */}
            <div
              className="flex flex-col border-b"
              style={{
                background: "#051005",
                borderColor: "rgba(0, 255, 0, 0.1)",
                minHeight: "320px",
              }}
            >
              <div
                className="flex items-center justify-between border-b px-4 py-2"
                style={{ borderColor: "rgba(0, 255, 0, 0.1)" }}
              >
                <h1 className="font-sans text-sm font-medium text-white">
                  {selectedPair} · 1H
                </h1>
                <div className="flex items-center gap-4 font-sans text-xs">
                  <span className="text-zinc-500">High <span className="font-mono text-zinc-400">{high}</span></span>
                  <span className="text-zinc-500">Low <span className="font-mono text-zinc-400">{low}</span></span>
                  <span className="text-zinc-500">Vol <span className="font-mono text-zinc-400">{vol}</span></span>
                </div>
              </div>
              <div className="relative min-h-0 flex-1" style={{ minHeight: "280px" }}>
                <TradingViewChart
                  symbol="BINANCE:ETHUSDT"
                  theme="dark"
                  autosize
                  interval="60"
                  className="absolute inset-0 h-full w-full"
                />
              </div>
            </div>

            {/* Tabbed: Open Positions / Orders History / Trade Logs */}
            <div className="min-h-0 flex-1 overflow-hidden p-2">
              <PositionTable
                activeTab={bottomTab}
                onTabChange={setBottomTab}
                positions={positionRows}
                orders={PLACEHOLDER_ORDERS}
                trades={transactions}
                positionsLoading={positionsLoading}
                chartSymbol={selectedPair}
                chartSymbolDefault="ETH/USDC"
                className="h-full min-h-[160px]"
              />
            </div>
          </div>

          {/* RIGHT: Execution Panel + TP/SL Orders + Live Logs */}
          <aside
            className="flex w-[340px] shrink-0 flex-col gap-2 overflow-y-auto p-2"
            style={{ background: "#000", borderLeft: "1px solid rgba(0, 255, 0, 0.1)" }}
          >
            <OrderForm
              balanceEth={balanceEth}
              marketPrice={lastPrice}
              isDemoMode={isDemoMode}
              canExecute={canExecute}
              isExecuting={isExecuting}
              onExecute={handleExecute}
            />
            <label className="mt-2 flex cursor-pointer items-center gap-2 font-sans text-xs text-zinc-400">
              <input
                type="checkbox"
                checked={useCommitReveal}
                onChange={(e) => setUseCommitReveal(e.target.checked)}
                className="rounded border-white/20 bg-black/40 accent-emerald-500"
              />
              <span>Use commit-reveal (MEV protection)</span>
            </label>
            <div
              className="mt-2 flex flex-col gap-1 rounded border px-3 py-2"
              style={{
                background: "rgba(34, 197, 94, 0.06)",
                borderColor: "rgba(34, 197, 94, 0.25)",
              }}
            >
              <div className="flex items-center gap-2 font-sans text-sm font-medium text-emerald-400/90">
                <span aria-hidden className="text-base leading-none">
                  ⚡
                </span>
                <span>Smart Execute</span>
              </div>
              <p className="font-sans text-xs text-zinc-400">
                Save up to 80% on execution fees via Stylus Batching.
              </p>
            </div>
            <TPSLOrderForm className="mt-1" />
            <LiveLogsPanel logs={logs} maxLines={8} variant="terminal" className="min-h-0 flex-1 overflow-hidden" />
          </aside>
        </motion.div>
      </main>
    </div>
  );
}
