"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { ConnectButton } from "@rainbow-me/rainbowkit";

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

export function Header() {
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
