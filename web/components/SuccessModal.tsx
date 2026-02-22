"use client";

import { useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import confetti from "canvas-confetti";

const ARBISCAN_BASE = "https://arbiscan.io/tx";

export interface SuccessModalProps {
  isOpen: boolean;
  txHash: string;
  amountIn?: string;
  amountOut?: string;
  token0Symbol?: string;
  token1Symbol?: string;
  /** When true, show DEMO badge and treat tx as simulated (realistic fake Arbiscan link). */
  isDemo?: boolean;
  onClose: () => void;
}

/** Trigger a premium confetti burst (gold/green) when demo success. */
function fireConfetti() {
  const count = 120;
  const defaults = { origin: { y: 0.6 }, zIndex: 9999 };

  function fire(particleRatio: number, opts: confetti.Options) {
    confetti({
      ...defaults,
      ...opts,
      particleCount: Math.floor(count * particleRatio),
    });
  }

  fire(0.25, { spread: 26, startVelocity: 55, colors: ["#22c55e", "#16a34a"] });
  fire(0.2, { spread: 60, startVelocity: 45, colors: ["#f59e0b", "#fbbf24"] });
  fire(0.35, { spread: 100, decay: 0.91, scalar: 0.8 });
  fire(0.1, { spread: 120, startVelocity: 25, decay: 0.92, scalar: 1.2 });
  fire(0.1, { spread: 120, startVelocity: 45 });
}

export function SuccessModal({
  isOpen,
  txHash,
  amountIn = "0",
  amountOut = "0",
  token0Symbol = "ETH",
  token1Symbol = "USDC",
  isDemo = false,
  onClose,
}: SuccessModalProps) {
  const hasFiredConfetti = useRef(false);

  // Confetti on open (once per open)
  useEffect(() => {
    if (isOpen && !hasFiredConfetti.current) {
      hasFiredConfetti.current = true;
      fireConfetti();
    }
    if (!isOpen) hasFiredConfetti.current = false;
  }, [isOpen]);

  const explorerUrl = `${ARBISCAN_BASE}/${txHash}`;
  const shareText = `Just swapped ${amountIn} ${token0Symbol} → ${amountOut} ${token1Symbol} on @OakProtocol 🍃 MEV-protected, Stylus-efficient. ${explorerUrl}`;
  const shareUrl = `https://twitter.com/intent/tweet?text=${encodeURIComponent(shareText)}`;

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="fixed inset-0 z-50 bg-black/70 backdrop-blur-md"
            onClick={onClose}
            aria-hidden="true"
          />
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 8 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 8 }}
              transition={{
                type: "spring",
                duration: 0.4,
                bounce: 0.2,
              }}
              className="relative w-full max-w-md overflow-hidden rounded-oak-lg shadow-[0_0_60px_rgba(34,197,94,0.15)]"
              style={{
                background: "rgba(15, 22, 19, 0.95)",
                backdropFilter: "blur(24px)",
                border: "1px solid rgba(34, 197, 94, 0.15)",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-oak-accent/50 to-transparent" />

              <div className="p-6 sm:p-8">
                {/* DEMO badge when simulated */}
                {isDemo && (
                  <div className="mb-3 flex justify-center">
                    <span
                      className="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold"
                      style={{
                        background: "rgba(245, 158, 11, 0.2)",
                        color: "#fbbf24",
                        boxShadow: "0 0 12px rgba(245, 158, 11, 0.3)",
                      }}
                    >
                      DEMO
                    </span>
                  </div>
                )}

                <motion.div
                  initial={{ scale: 0 }}
                  animate={{ scale: 1 }}
                  transition={{ delay: 0.1, type: "spring", stiffness: 200 }}
                  className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-oak-accent/20"
                >
                  <svg
                    className="h-7 w-7 text-oak-accent"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={2.5}
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                </motion.div>

                <h2 className="text-center text-xl font-semibold text-oak-text-primary">
                  Transaction Successful
                </h2>
                <p className="mt-1 text-center text-sm text-oak-text-secondary">
                  {isDemo
                    ? "Simulated swap completed (Demo Mode)"
                    : "Your swap was MEV-protected via commit-reveal"}
                </p>

                <div className="mt-5 rounded-oak border border-oak-border bg-oak-bg-elevated p-4">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-oak-text-muted">{token0Symbol}</span>
                    <span className="font-mono text-oak-text-primary">
                      {amountIn}
                    </span>
                  </div>
                  <div className="my-2 flex justify-center text-oak-text-muted">
                    →
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-oak-text-muted">{token1Symbol}</span>
                    <span className="font-mono text-oak-text-primary">
                      {amountOut}
                    </span>
                  </div>
                </div>

                <a
                  href={explorerUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-4 flex items-center justify-between rounded-oak border border-oak-border bg-oak-bg-elevated px-4 py-3 transition-colors hover:border-oak-accent/50 hover:bg-oak-bg-hover"
                >
                  <span className="truncate font-mono text-xs text-oak-text-secondary">
                    {txHash}
                  </span>
                  <svg
                    className="ml-2 h-4 w-4 shrink-0 text-oak-accent"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                    />
                  </svg>
                </a>

                <div className="mt-5 flex gap-3">
                  <motion.a
                    href={shareUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    whileTap={{ scale: 0.98 }}
                    className="flex flex-1 items-center justify-center gap-2 rounded-oak border border-oak-border/60 bg-oak-bg-elevated/80 py-3 text-sm font-medium text-oak-text-primary transition-colors hover:border-oak-accent/30 hover:bg-oak-accent/5"
                  >
                    <svg
                      className="h-5 w-5"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
                    </svg>
                    Share on X
                  </motion.a>
                  <motion.button
                    type="button"
                    onClick={onClose}
                    whileTap={{ scale: 0.98 }}
                    className="flex flex-1 items-center justify-center rounded-oak bg-oak-accent py-3 text-sm font-medium text-white transition-colors hover:bg-oak-accent-hover"
                  >
                    Done
                  </motion.button>
                </div>
              </div>
            </motion.div>
          </div>
        </>
      )}
    </AnimatePresence>
  );
}
