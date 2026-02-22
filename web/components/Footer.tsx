"use client";

import Link from "next/link";
import { motion } from "framer-motion";

const FOOTER_LINKS = [
  { label: "GitHub", href: "https://github.com", external: true },
  { label: "Docs", href: "#", external: false },
  { label: "Twitter", href: "https://twitter.com", external: true },
] as const;

const springTap = {
  scale: 0.98,
  transition: { type: "spring" as const, stiffness: 500, damping: 30 },
};

export function Footer() {
  return (
    <footer
      className="mt-auto w-full border-t border-oak-border/60"
      style={{
        background: "rgba(12, 18, 16, 0.5)",
        backdropFilter: "blur(20px)",
        WebkitBackdropFilter: "blur(20px)",
      }}
    >
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-4 py-6 sm:flex-row sm:px-6 lg:px-8">
        <div className="flex flex-col items-center gap-2 sm:flex-row sm:gap-4">
          <p className="text-sm text-oak-text-muted">
            © {new Date().getFullYear()} Oak Protocol. MEV-protected DEX on Arbitrum.
          </p>
          {/* Security Shield — Oak Sentinel */}
          <div
            className="flex items-center gap-2 rounded-lg border border-oak-border/50 bg-oak-bg-elevated/50 px-3 py-1.5 text-xs"
            style={{
              boxShadow: "0 0 12px rgba(34, 197, 94, 0.06)",
            }}
          >
            <svg
              className="h-4 w-4 shrink-0 text-oak-accent"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z"
              />
            </svg>
            <span className="text-oak-text-secondary">
              Oak Sentinel Active: Zero-Knowledge Commitments + WASM Isolation.
            </span>
          </div>
        </div>
        <nav className="flex items-center gap-6" aria-label="Footer links">
          {FOOTER_LINKS.map(({ label, href, external }) => (
            <motion.span key={label} whileTap={springTap}>
              <Link
                href={href}
                target={external ? "_blank" : undefined}
                rel={external ? "noopener noreferrer" : undefined}
                className="text-sm text-oak-text-secondary transition-colors hover:text-oak-accent"
              >
                {label}
              </Link>
            </motion.span>
          ))}
        </nav>
      </div>
    </footer>
  );
}
