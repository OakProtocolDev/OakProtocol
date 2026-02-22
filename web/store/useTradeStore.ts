/**
 * Oak Protocol — Global Trade Store (Blitzscale)
 * Single source of truth for demo mode, virtual balances, and trade history.
 * Balances persist to localStorage; demo mode and transactions are in-memory per session.
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type BalanceKey = "ETH" | "USDC" | "OAK";

export interface Balances {
  ETH: number;
  USDC: number;
  OAK: number;
}

export interface TradeRecord {
  id: string;
  timestamp: number;
  side: "buy" | "sell";
  amountIn: string;
  amountOut: string;
  token0Symbol: string;
  token1Symbol: string;
  txHash: string;
  /** True when executed in Demo Mode (virtual execution) */
  isDemo: boolean;
}

interface TradeState {
  /** When true, UI shows virtual balances and trades are simulated (no chain tx). */
  isDemoMode: boolean;
  /** Virtual balances for demo mode. Persisted to localStorage. */
  balances: Balances;
  /** All trades this session (demo + real). Real trades would be appended from chain events in production. */
  transactions: TradeRecord[];
}

interface TradeActions {
  setDemoMode: (value: boolean) => void;
  setBalances: (balances: Partial<Balances>) => void;
  /** Deduct from one balance and add to another (e.g. swap ETH → USDC). */
  applySwap: (from: BalanceKey, to: BalanceKey, amountFrom: number, amountTo: number) => void;
  addTransaction: (trade: TradeRecord) => void;
  resetDemoBalances: () => void;
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const DEFAULT_BALANCES: Balances = {
  ETH: 1.5,
  USDC: 10000,
  OAK: 0,
};

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

/** Persist only balances to localStorage so demo balances survive refresh. */
const BALANCES_STORAGE_KEY = "oak-demo-balances";

export const useTradeStore = create<TradeState & TradeActions>()(
  persist(
    (set) => ({
      isDemoMode: true,
      balances: DEFAULT_BALANCES,
      transactions: [],

      setDemoMode: (value) => set({ isDemoMode: value }),

      setBalances: (next) =>
        set((s) => ({
          balances: { ...s.balances, ...next },
        })),

      applySwap: (from, to, amountFrom, amountTo) =>
        set((s) => ({
          balances: {
            ...s.balances,
            [from]: Math.max(0, s.balances[from] - amountFrom),
            [to]: s.balances[to] + amountTo,
          },
        })),

      addTransaction: (trade) =>
        set((s) => ({
          transactions: [trade, ...s.transactions],
        })),

      resetDemoBalances: () => set({ balances: DEFAULT_BALANCES }),
    }),
    {
      name: BALANCES_STORAGE_KEY,
      partialize: (state) => ({ balances: state.balances }),
    }
  )
);

/** Helper: get display balance for a token in the current mode. In demo mode uses store; otherwise use on-chain balance (passed in). */
export function getDisplayBalance(
  token: BalanceKey,
  onChainBalance: string,
  isDemoMode: boolean,
  storeBalances: Balances
): string {
  if (isDemoMode) {
    const n = storeBalances[token];
    return n >= 0 ? n.toFixed(6) : "0";
  }
  return onChainBalance;
}
