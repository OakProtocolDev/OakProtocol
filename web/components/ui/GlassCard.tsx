"use client";

import { motion } from "framer-motion";

export interface GlassCardProps {
  children: React.ReactNode;
  className?: string;
  elevated?: boolean;
  hover?: boolean;
}

export function GlassCard({
  children,
  className = "",
  elevated = false,
  hover = false,
}: GlassCardProps) {
  const Comp = motion.div;
  return (
    <Comp
      className={`rounded-oak-lg ${elevated ? "glass-card-elevated" : "glass-card"} ${className}`}
      initial={false}
      whileHover={hover ? { scale: 1.005 } : undefined}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
    >
      {children}
    </Comp>
  );
}
