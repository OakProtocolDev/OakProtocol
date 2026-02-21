"use client";

export interface PoolInfoProps {
  /** Reserve of token0 (from contract/hooks) */
  reserve0?: string;
  /** Reserve of token1 (from contract/hooks) */
  reserve1?: string;
  /** Token0 symbol */
  token0Symbol?: string;
  /** Token1 symbol */
  token1Symbol?: string;
  /** Current TWAP price (e.g. "1.05") */
  twapPrice?: string;
  /** 24h average TWAP (placeholder) */
  twap24h?: string;
  /** APY or fee info (placeholder) */
  apyOrFees?: string;
  /** Loading state */
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
    <div className="w-full max-w-md rounded-oak-lg border border-oak-border bg-oak-bg-card p-5 sm:p-6">
      <h2 className="mb-4 text-lg font-medium text-oak-text-primary">Pool Info</h2>

      {isLoading ? (
        <div className="space-y-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-10 animate-pulse rounded bg-oak-bg-elevated" />
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
          <div className="border-t border-oak-border pt-4">
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
  );
}
