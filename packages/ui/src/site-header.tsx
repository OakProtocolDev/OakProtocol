import * as React from "react";
import { cn } from "./cn";
import { buttonVariants } from "./button";

export interface OakSiteHeaderProps {
  /** Current subdomain / app name for highlight */
  current?: "profile" | "leaderboard" | "bridge" | "home";
  className?: string;
}

const LINKS = [
  { href: "https://oak.trade", label: "Oak", key: "home" as const },
  { href: "https://profile.oak.trade", label: "Profile", key: "profile" as const },
  { href: "https://leaderboard.oak.trade", label: "Leaderboard", key: "leaderboard" as const },
  { href: "https://bridge.oak.trade", label: "Bridge", key: "bridge" as const },
];

export function OakSiteHeader({ current, className }: OakSiteHeaderProps) {
  return (
    <header
      className={cn(
        "sticky top-0 z-50 w-full border-b border-oak-border bg-oak-bg/95 backdrop-blur supports-[backdrop-filter]:bg-oak-bg/80",
        className
      )}
    >
      <div className="container flex h-14 items-center justify-between gap-4 px-4">
        <nav className="flex items-center gap-6">
          {LINKS.map(({ href, label, key }) => {
            const isActive = current === key;
            return (
              <a
                key={key}
                href={href}
                target="_blank"
                rel="noopener noreferrer"
                className={cn(
                  "text-sm font-medium transition-colors hover:text-oak-accent",
                  isActive ? "text-oak-accent" : "text-oak-text-secondary"
                )}
              >
                {label}
              </a>
            );
          })}
        </nav>
        <a
          href="https://app.oak.trade"
          target="_blank"
          rel="noopener noreferrer"
          className={cn(buttonVariants({ variant: "outline", size: "sm" }))}
        >
          Open App
        </a>
      </div>
    </header>
  );
}
