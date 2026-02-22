"use client";

import React from "react";
import { motion } from "framer-motion";
import { useTradeStore } from "@/store/useTradeStore";

/**
 * Full-screen tint overlay that shifts between Amber (Demo) and Green (Real).
 * Zero pointer-events so it does not block interaction.
 */
export function ThemeTint({ children }: { children: React.ReactNode }) {
  const isDemoMode = useTradeStore((s) => s.isDemoMode);

  return (
    <React.Fragment>
      {children}
      <motion.div
        className="fixed inset-0 z-[1] pointer-events-none"
        initial={false}
        animate={{
          backgroundColor: isDemoMode
            ? "rgba(245, 158, 11, 0.04)"
            : "rgba(34, 197, 94, 0.03)",
        }}
        transition={{ duration: 0.5, ease: [0.25, 0.46, 0.45, 0.94] }}
        aria-hidden
      />
    </React.Fragment>
  );
}
