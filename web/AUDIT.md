# Oak Protocol – Full-stack audit summary

## 1. Runtime fixes (PriceChart)

- **Issue:** `LineSeries is not defined` / `chart.addLineSeries is not a function` with lightweight-charts in Next.js.
- **Cause:** v5 uses `chart.addSeries(LineSeries, options)` and the library was being loaded in a context where it failed (SSR or bundling).
- **Fix:** 
  - Chart is created only on the client after a `mounted` state is true.
  - `lightweight-charts` is loaded via **dynamic `import("lightweight-charts")`** inside `useEffect`, so it never runs on the server.
  - Cleanup and `cancelled` flag prevent setting refs or leaving listeners after unmount.
- **Types:** `ChartLinePoint` uses `time: number` (UTCTimestamp). `@ts-expect-error` is used where the library’s `Time` type doesn’t accept `number` in the signature (valid at runtime).

## 2. Server / Client boundary (Next.js 14)

- **Issue:** Event handlers (e.g. `onSwap`) must not be passed from Server Components to Client Components.
- **Fix:** 
  - `app/page.tsx` is a **Client Component** (`"use client"` at top). All data and the swap handler are created in the client tree, so no functions are serialized from server to client.
  - `Header`, `SwapWidget`, `PoolInfo`, `PriceChart`, `Footer` are Client Components where needed (they use hooks or browser APIs).
- **Placeholder handler:** `getPlaceholderSwapHandler()` is called inside the page component and passed to `SwapWidget`; both run on the client.

## 3. Dependencies

- **Peer dependencies:** RainbowKit and wagmi peers (`@tanstack/react-query`, `react`, `react-dom`, `viem`, `wagmi`) are already in `package.json`.
- **Optional peer warnings:** If you see warnings for `pino-pretty` or `@react-native-async-storage/async-storage`, they are optional. To install them:
  ```bash
  npm install pino-pretty @react-native-async-storage/async-storage --save-optional
  ```
- **Install command:** From repo root, `cd web && npm install`.

## 4. Layout and theme

- **Responsive grid:** 
  - Mobile: 1 column (Chart → Swap → Pool stacked).
  - `md`: 2 columns (Chart | Swap), Pool full width below.
  - `xl`: 3 columns (Chart | Swap | Pool). 
  - `2xl`: same 3 columns with larger max-width and padding.
- **Oak palette:** All UI uses Tailwind `oak-*` classes from `tailwind.config.ts` (`oak-bg`, `oak-bg-elevated`, `oak-bg-card`, `oak-border`, `oak-text-*`, `oak-accent`). PriceChart uses the same hex values in its config for consistency.

## 5. Web3 security (web)

- **wagmi.ts**
  - Only `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` is used (client-safe).
  - No private keys or secrets; single chain (Arbitrum Sepolia).
- **providers.tsx**
  - WagmiProvider + QueryClientProvider + RainbowKitProvider; no sensitive state.
- **useOakPool.ts**
  - Placeholder only. Comments added: when implementing, use only user-signed txs (e.g. `writeContract` / `sendTransaction`), validate chain, and map revert reasons to user-facing messages. No private keys in the frontend.
- **Env:** Only `NEXT_PUBLIC_*` vars are used; no server-only secrets in client bundle.

## 6. Contracts (Rust/Stylus) – recap

From prior audit of `src/`:

- **Reentrancy:** Lock taken at start of `reveal_swap`, `add_liquidity`, `withdraw_treasury_fees`, `flash_swap`; released at end. CEI respected (effects before token transfers).
- **Arithmetic:** Uses `checked_*` and explicit overflow/division-by-zero handling in logic, TWAP, and fee math.
- **Access control:** `only_owner` used for `set_fee`, `pause`, `unpause`, `withdraw_treasury_fees`.
- **Commit–reveal:** Hash and delay checks; commitment cleared before external calls; salt in hash to avoid collision abuse.
- **TWAP:** Zero reserves and zero `time_elapsed` handled; cumulative price updated before reserve changes; multiple swaps in same block safe.
- **Slippage / deadline:** Strict checks; deadline is “revert when block > deadline” (inclusive of block equals deadline is allowed).
- **Flash swap:** k-invariant with fee enforced; no free use of liquidity.

No contract code was changed in this audit; only web fixes and docs were added.
