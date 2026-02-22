"use client";

import { useEffect, useRef } from "react";
import { motion } from "framer-motion";

export interface LogEntry {
  id: string;
  timestamp: string;
  message: string;
  level?: "info" | "success" | "muted" | "warn" | "scan";
}

export interface LiveLogsPanelProps {
  logs: LogEntry[];
  maxLines?: number;
  className?: string;
  /** Use terminal aesthetic: #051005 panel, 1px green border, blur 12px */
  variant?: "default" | "terminal";
}

/** Resolve log row color/emphasis by level. */
function logLevelClass(level?: LogEntry["level"]): string {
  switch (level) {
    case "success":
      return "text-oak-accent";
    case "warn":
      return "text-oak-warning";
    case "scan":
      return "text-amber-400/90";
    case "muted":
      return "text-oak-text-muted/70";
    default:
      return "text-oak-text-secondary";
  }
}

export function LiveLogsPanel({
  logs,
  maxLines = 8,
  className = "",
  variant = "default",
}: LiveLogsPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const isTerminal = variant === "terminal";

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  const displayLogs = logs.slice(-maxLines);

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay: 0.1 }}
      className={`overflow-hidden rounded-lg font-mono ${className}`}
      style={{
        background: isTerminal ? "#051005" : "rgba(15, 22, 19, 0.5)",
        backdropFilter: isTerminal ? "blur(12px)" : "blur(20px)",
        WebkitBackdropFilter: isTerminal ? "blur(12px)" : "blur(20px)",
        border: isTerminal ? "1px solid rgba(0, 255, 0, 0.1)" : "1px solid rgba(34, 197, 94, 0.08)",
      }}
    >
      <div
        className={`flex items-center justify-between px-3 py-2 ${!isTerminal ? "border-b border-oak-border/60" : ""}`}
        style={isTerminal ? { borderBottom: "1px solid rgba(0, 255, 0, 0.1)" } : undefined}
      >
        <span className={`text-xs font-medium ${isTerminal ? "text-zinc-500" : "text-oak-text-muted"}`}>
          Commit-Reveal · MEV Shield
        </span>
        <span
          className="relative flex h-2 w-2 items-center justify-center"
          aria-hidden
        >
          <span
            className="absolute h-2 w-2 animate-ping rounded-full bg-oak-accent/60"
            style={{
              animationDuration: "2s",
              boxShadow:
                "0 0 8px rgba(34, 197, 94, 0.6), 0 0 16px rgba(34, 197, 94, 0.3)",
            }}
          />
          <span
            className="relative h-1.5 w-1.5 rounded-full bg-oak-accent"
            style={{
              boxShadow:
                "0 0 8px rgba(34, 197, 94, 0.8), 0 0 12px rgba(34, 197, 94, 0.4)",
            }}
          />
        </span>
      </div>
      <div
        ref={scrollRef}
        className="max-h-32 overflow-y-auto px-3 py-2 text-xs"
      >
        {displayLogs.length === 0 ? (
          <p className="text-oak-text-muted/60">
            Swap to see MEV-protected commit-reveal logs…
          </p>
        ) : (
          <div className="space-y-1">
            {displayLogs.map((log) => (
              <motion.div
                key={log.id}
                initial={{ opacity: 0, y: 4, scale: 0.98 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                transition={{ duration: 0.2 }}
                className={`flex gap-2 ${logLevelClass(log.level)}`}
              >
                <span className="shrink-0 text-oak-text-muted/50">
                  [{log.timestamp}]
                </span>
                <span>{log.message}</span>
              </motion.div>
            ))}
          </div>
        )}
      </div>
    </motion.div>
  );
}
