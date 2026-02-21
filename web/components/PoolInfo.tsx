"use client";

import { motion } from "framer-motion";

export interface PoolInfoProps {
  reserve0?: string;
  reserve1?: string;
  token0Symbol?: string;
  token1Symbol?: string;
  twapPrice?: string;
  twap24h?: string;
  apyOrFees?: string;
  isLoading?: boolean;
}

export function PoolInfo({
  reserve0 = "—",
  reserve1 = "—",
  token0Symbol = "TOKEN0",
  token1Symbol = "TOKEN1",
  twapPrice = "—",
  twap24h = "—",
  apyOrFees = "0.3% fee",
  isLoading = false,
}: PoolInfoProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.25, 0.46, 0.45, 0.94] }}
      className="glass-card w-full max-w-md overflow-hidden"
      style={{
        background: "rgba(15, 22, 19, 0.55)",
        backdropFilter: "blur(20px)",
        border: "1px solid rgba(34, 197, 94, 0.08)",
      }}
    >
      <div className="p-5 sm:p-6">
        <h2 className="mb-4 text-lg font-medium text-oak-text-primary">Pool Info</h2>

        {isLoading ? (
          <div className="space-y-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-10 animate-pulse rounded bg-oak-bg-elevated/80" />
            ))}
          </div>
        ) : (
          <dl className="space-y-4">
            <div className="flex justify-between text-sm">
              <dt className="text-oak-text-secondary">Liquidity ({token0Symbol})</dt>
              <dd className="font-medium text-oak-text-primary">{reserve0}</dd>
            </div>
            <div className="flex justify-between text-sm">
              <dt className="text-oak-text-secondary">Liquidity ({token1Symbol})</dt>
              <dd className="font-medium text-oak-text-primary">{reserve1}</dd>
            </div>
            <div className="border-t border-oak-border/60 pt-4">
              <div className="flex justify-between text-sm">
                <dt className="text-oak-text-secondary">TWAP Price</dt>
                <dd className="font-medium text-oak-text-primary">{twapPrice}</dd>
              </div>
              <div className="mt-2 flex justify-between text-sm">
                <dt className="text-oak-text-secondary">24h Avg</dt>
                <dd className="font-medium text-oak-text-primary">{twap24h}</dd>
              </div>
            </div>
            <div className="flex justify-between text-sm">
              <dt className="text-oak-text-secondary">Fees</dt>
              <dd className="font-medium text-oak-text-primary">{apyOrFees}</dd>
            </div>
          </dl>
        )}
      </div>
    </motion.div>
  );
}
