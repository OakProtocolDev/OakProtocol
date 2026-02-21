"use client";

import Link from "next/link";
import { ConnectButton } from "@rainbow-me/rainbowkit";

export function Header() {
  return (
    <header className="sticky top-0 z-50 w-full border-b border-oak-border bg-oak-bg/80 backdrop-blur-md transition-colors duration-oak">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <div className="flex items-center gap-6">
          <Link
            href="/"
            className="text-xl font-semibold tracking-tight text-oak-text-primary transition-opacity hover:opacity-90"
          >
            Oak Protocol
          </Link>
          <nav className="flex items-center gap-4">
            <Link
              href="/"
              className="text-sm text-oak-text-secondary transition-colors hover:text-oak-accent"
            >
              Swap
            </Link>
            <Link
              href="/trading"
              className="text-sm text-oak-text-secondary transition-colors hover:text-oak-accent"
            >
              Trading
            </Link>
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
