/**
 * Placeholder for Oak Protocol pool & swap hooks.
 *
 * TODO: Implement with wagmi/viem to:
 * - useOakPoolReserves() -> reserve0, reserve1, twapPrice from contract
 * - useTokenBalance(token, address) -> balance for connected wallet
 * - useSwap() -> commit_swap + reveal_swap (or single flow) with proper error mapping
 *
 * SECURITY (when implementing):
 * - Never use private keys or secrets in the frontend; only user-signed txs via wagmi (writeContract/sendTransaction).
 * - Validate chain (e.g. useChainId) before sending; reject if not Arbitrum Sepolia.
 * - Map contract revert reasons to user-facing messages; do not expose raw calldata or internal errors.
 */

export function useOakPoolReserves() {
  return {
    reserve0: undefined as string | undefined,
    reserve1: undefined as string | undefined,
    twapPrice: undefined as string | undefined,
    isLoading: true,
    error: null as Error | null,
  };
}

export function useTokenBalance(_tokenAddress: string, _account: string | undefined) {
  return { balance: "0", isLoading: false, error: null as Error | null };
}

export function useSwap() {
  return {
    swap: async (
      _amountIn: string,
      _minAmountOut: string,
      _deadline: number
    ): Promise<void> => {
      // no-op until contract calls are wired
    },
    isSwapping: false,
    error: null as string | null,
  };
}
