"use client";

import Link from "next/link";

const FOOTER_LINKS = [
  { label: "GitHub", href: "https://github.com", external: true },
  { label: "Docs", href: "#", external: false },
  { label: "Twitter", href: "https://twitter.com", external: true },
] as const;

export function Footer() {
  return (
    <footer className="mt-auto w-full border-t border-oak-border bg-oak-bg-elevated">
      <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-4 py-6 sm:flex-row sm:px-6 lg:px-8">
        <p className="text-sm text-oak-text-muted">
          © {new Date().getFullYear()} Oak Protocol. MEV-protected DEX on Arbitrum.
        </p>
        <nav className="flex items-center gap-6" aria-label="Footer links">
          {FOOTER_LINKS.map(({ label, href, external }) => (
            <Link
              key={label}
              href={href}
              target={external ? "_blank" : undefined}
              rel={external ? "noopener noreferrer" : undefined}
              className="text-sm text-oak-text-secondary transition-colors hover:text-oak-text-primary"
            >
              {label}
            </Link>
          ))}
        </nav>
      </div>
    </footer>
  );
}
