"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useTradeStore } from "@/store/useTradeStore";

const springTap = {
  scale: 0.98,
  transition: { type: "spring" as const, stiffness: 500, damping: 30 },
};

export function Header() {
  const isDemoMode = useTradeStore((s) => s.isDemoMode);
  const setDemoMode = useTradeStore((s) => s.setDemoMode);

  return (
    <header
      className="sticky top-0 z-50 w-full border-b border-oak-border/60 transition-colors duration-300"
      style={{
        background: "rgba(5, 8, 7, 0.7)",
        backdropFilter: "blur(20px)",
        WebkitBackdropFilter: "blur(20px)",
        boxShadow: "0 1px 0 rgba(34, 197, 94, 0.06)",
      }}
    >
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <div className="flex items-center gap-6">
          <Link
            href="/"
            className="text-xl font-semibold tracking-tight text-oak-text-primary transition-opacity hover:opacity-90"
          >
            Oak Protocol
          </Link>
          <nav className="flex items-center gap-1">
            <motion.div whileTap={springTap}>
              <Link
                href="/"
                className="rounded-lg px-3 py-2 text-sm text-oak-text-secondary transition-colors hover:bg-oak-accent/10 hover:text-oak-accent"
              >
                Swap
              </Link>
            </motion.div>
            <motion.div whileTap={springTap}>
              <Link
                href="/trading"
                className="rounded-lg px-3 py-2 text-sm text-oak-text-secondary transition-colors hover:bg-oak-accent/10 hover:text-oak-accent"
              >
                Trading
              </Link>
            </motion.div>
          </nav>
        </div>
        <div className="flex items-center gap-4">
          {/* Premium DEMO MODE toggle — gold/amber neon glow when ON */}
          <motion.button
            type="button"
            role="switch"
            aria-checked={isDemoMode}
            aria-label="Toggle Demo Mode"
            onClick={() => setDemoMode(!isDemoMode)}
            whileTap={springTap}
            className="relative flex items-center gap-2 rounded-lg border px-3 py-2 text-sm font-medium transition-all duration-300"
            style={{
              borderColor: isDemoMode
                ? "rgba(245, 158, 11, 0.5)"
                : "rgba(34, 197, 94, 0.2)",
              background: isDemoMode
                ? "rgba(245, 158, 11, 0.12)"
                : "rgba(34, 197, 94, 0.06)",
              boxShadow: isDemoMode
                ? "0 0 20px rgba(245, 158, 11, 0.35), 0 0 40px rgba(245, 158, 11, 0.15)"
                : "0 0 0 1px rgba(34, 197, 94, 0.1)",
            }}
          >
            <span
              className="h-2 w-2 rounded-full"
              style={{
                background: isDemoMode ? "#f59e0b" : "#22c55e",
                boxShadow: isDemoMode
                  ? "0 0 8px rgba(245, 158, 11, 0.8)"
                  : "0 0 6px rgba(34, 197, 94, 0.6)",
              }}
            />
            <span
              className={
                isDemoMode
                  ? "text-amber-400"
                  : "text-oak-text-secondary"
              }
            >
              DEMO MODE
            </span>
          </motion.button>
          <ConnectButton
            chainStatus="icon"
            showBalance={false}
            accountStatus="address"
          />
        </div>
      </div>
    </header>
  );
}
