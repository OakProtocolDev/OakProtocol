"use client";

import Link from "next/link";
import { motion } from "framer-motion";

const FOOTER_LINKS = [
  { label: "GitHub", href: "https://github.com", external: true },
  { label: "Docs", href: "#", external: false },
  { label: "Twitter", href: "https://twitter.com", external: true },
] as const;

const springTap = { scale: 0.98, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

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
        <p className="text-sm text-oak-text-muted">
          © {new Date().getFullYear()} Oak Protocol. MEV-protected DEX on Arbitrum.
        </p>
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
