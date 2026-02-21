/**
 * Placeholder data and handlers for Oak Protocol UI.
 * Replace with real hooks/utils that read from contract and execute commit/reveal swaps.
 */

export interface PlaceholderPoolData {
  token0Symbol: string;
  token1Symbol: string;
  token0Balance: string;
  estimatedOutput: string;
  isLoadingQuote: boolean;
  reserve0: string;
  reserve1: string;
  twapPrice: string;
  twap24h: string;
  apyOrFees: string;
  isLoadingPool: boolean;
  error: string | null;
}

export function getPlaceholderPoolData(): PlaceholderPoolData {
  return {
    token0Symbol: "ETH",
    token1Symbol: "USDC",
    token0Balance: "0",
    estimatedOutput: "0",
    isLoadingQuote: false,
    reserve0: "—",
    reserve1: "—",
    twapPrice: "—",
    twap24h: "—",
    apyOrFees: "0.3% fee",
    isLoadingPool: false,
    error: null,
  };
}

export function getPlaceholderSwapHandler(): (
  amountIn: string,
  minAmountOut: string,
  deadline: number
) => Promise<void> {
  return async (amountIn, minAmountOut, deadline) => {
    // TODO: wire to useSwap / commit_swap + reveal_swap flow
    await new Promise((r) => setTimeout(r, 1500));
    console.log("Swap placeholder", { amountIn, minAmountOut, deadline });
  };
}
