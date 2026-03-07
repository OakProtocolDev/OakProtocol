# Ecosystem UI Layer

Oak Protocol’s frontend is a **Next.js monorepo** with one app per subdomain. All apps share a common UI kit (Tailwind + Framer Motion + shadcn-style components).

---

## Monorepo layout (modular: one folder per subdomain)

Each subdomain is a **separate Next.js app** in its own folder; shared UI lives in `packages/ui` (shadcn-style + Oak theme).

```
apps/
  profile/       → profile.oak.trade  (Universal Dashboard)
  leaderboard/   → leaderboard.oak.trade (Top Traders / Top Farmers)
  bridge/        → bridge.oak.trade   (LayerZero / Axelar → Arbitrum)
packages/
  ui/            → @oak-protocol/ui  (shared components, Tailwind preset)
```

- **Root**: `package.json` with `workspaces: ["apps/*", "packages/*"]`.
- **Each app**: Next.js 14, Tailwind, Framer Motion, `@oak-protocol/ui`. No cross-app imports; only `@oak-protocol/ui` is shared.
- **Build order**: build `packages/ui` first, then apps.

---

## 1. profile.oak.trade — Universal Dashboard

**Role**: Single dashboard for PnL, trade history, active orders, open positions, and staking rewards.

**Data source**: Oak **subgraph** (GraphQL). Set `NEXT_PUBLIC_OAK_SUBGRAPH_URL` to the deployed subgraph endpoint.

**Features**:
- **User stats**: Total PnL (USD), total volume (USD), trade count.
- **Recent trades**: From `swaps` (sender = user), with tokenIn/tokenOut, amountOut, fee.
- **Active orders**: Limit / TP / SL with status open; trigger price and order type.
- **Open positions**: With size, entry price, and PnL (from subgraph `positions.pnl`).
- **Staking rewards**: From `emissionEvents` (moduleId = 1: Staking); amount and event type.

**Tech**: `@tanstack/react-query` for subgraph queries; Framer Motion for staggered list animations; `@oak-protocol/ui` Card, etc.

**Subgraph schema expectations** (for reference): entities such as `User` (id, totalVolumeUsd, pnlUsd, tradeCount), `Swap`, `Order`, `Position`, `EmissionEvent` with fields used in `apps/profile/lib/subgraph.ts`.

---

## 2. leaderboard.oak.trade — Dynamic rankings

**Role**: **Top Traders** and **Top Farmers** with scores = **Volume × Recency** (time decay).

**Score formula**:  
`score = volume_usd * exp(-λ * (now - last_activity_ts))`  
with λ ≈ 1/(7 days) so activity decays over ~1 week.

**Data flow**:
1. **Cron job** (`apps/leaderboard/scripts/recompute-rankings.js`) runs on a schedule (e.g. hourly via GitHub Actions or local runner).
2. Script fetches **traders** from the subgraph (`traders { id, totalVolumeUsd, lastTradeTimestamp }`) and optionally **farmers** (`liquidityProviders { id, totalLiquidityUsd, lastActivityTimestamp }` if the subgraph exposes them), computes scores, and writes **`data/rankings.json`**.
3. **API** `GET /api/rankings`: (1) reads `data/rankings.json` if present; (2) otherwise fetches from subgraph and computes on the fly (for serverless/Vercel); (3) falls back to placeholder.
4. **Frontend** `useRankings()` fetches `/api/rankings` and displays two tables: Top Traders, Top Farmers.

**Cron usage**:
```bash
cd apps/leaderboard && node scripts/recompute-rankings.js
# Or from repo root: npm run rankings
```
Set `SUBGRAPH_URL` or `NEXT_PUBLIC_OAK_SUBGRAPH_URL` so the script can pull trader/farmer data. The script writes `data/rankings.json`; the API serves this file when present (e.g. self-hosted or GitHub Actions that commit the file).

**Vercel Cron:** `apps/leaderboard/vercel.json` defines an hourly cron (`0 * * * *`) that hits `GET /api/rankings`. On Vercel (no persistent file), the API recomputes rankings from the subgraph on each request when `data/rankings.json` is missing.

---

## 3. bridge.oak.trade — Cross-chain to Arbitrum

**Role**: Seamless **inflow of assets to Arbitrum** via LayerZero (Stargate) and links to Axelar / native bridge.

**Implementation**:
- **LayerZero Stargate**: Embedded widget (`@layerzerolabs/stargate-ui` custom element). User selects source chain and asset; destination is Arbitrum (chain ID 42161). The widget accepts `theme="dark"` and optional `data-destination-chain-id={42161}` for prefill when supported.
- **Links**: Stargate Transfer, Satellite (Axelar), Arbitrum Official Bridge for users who prefer a specific provider. Framer Motion staggered animations for cards.

**Tech**: Next.js, `next/script` for lazy-loading the Stargate script; Framer Motion for page entrance; `@oak-protocol/ui` for layout.

**Optional**: For deeper Axelar integration, use Axelar SDK or Satellite API to prefill destination (Arbitrum) and show status; the current design keeps Axelar as an external link.

---

## 4. Shared UI kit (@oak-protocol/ui)

**Location**: `packages/ui`.

**Exports**: `cn`, `Button`, `buttonVariants`, `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardContent`, `CardFooter`, `Skeleton`, `Badge`, `badgeVariants`, `OakSiteHeader`.

**Styling**: Tailwind preset in `packages/ui/tailwind.preset.js` defines Oak theme:
- Colors: `oak-bg`, `oak-bg-elevated`, `oak-bg-card`, `oak-border`, `oak-text-primary`, `oak-text-secondary`, `oak-text-muted`, `oak-accent`.
- Border radius: `rounded-oak`, `rounded-oak-lg`.
- Shadow: `shadow-oak`, `shadow-oak-glow`.

Each app’s `tailwind.config.ts` uses this preset and includes `../../packages/ui/src/**/*` in `content`.

**Convention**: shadcn/ui-style (CVA + `cn`). Add new components (e.g. Skeleton, Badge, Table) in `packages/ui` and re-export from `index.tsx`.

---

## 5. Running the apps

From repo root (with workspaces installed):

```bash
npm install
npm run build:ui
npm run dev:profile    # http://localhost:3001
npm run dev:leaderboard # http://localhost:3002
npm run dev:bridge     # http://localhost:3003
```

Subdomains in production: point `profile.oak.trade`, `leaderboard.oak.trade`, `bridge.oak.trade` to the respective Next.js app (e.g. Vercel project per app or path-based routing).

---

## 6. Summary

| Subdomain            | App        | Data / trigger              | Stack                    |
|----------------------|------------|-----------------------------|--------------------------|
| profile.oak.trade    | profile    | Subgraph (PnL, trades, orders, staking) | Next.js, React Query, Framer Motion, @oak-protocol/ui |
| leaderboard.oak.trade | leaderboard| Cron → rankings.json → API | Next.js, API route, Framer Motion, @oak-protocol/ui |
| bridge.oak.trade     | bridge     | LayerZero Stargate + links (Axelar, Arbitrum) | Next.js, Framer Motion, @oak-protocol/ui |

All UI is **modular** (one folder per subdomain) with a **shared UI kit** and consistent Tailwind + Framer Motion usage.
