# Oak Ecosystem UI — Monorepo Apps

Each subdomain is a **separate Next.js app** in this folder, sharing the same UI kit (`@oak-protocol/ui`) and stack: **Next.js**, **Tailwind**, **Framer Motion**, **shadcn-style components**.

## Apps

| App | Subdomain | Description |
|-----|-----------|-------------|
| **profile** | profile.oak.trade | Universal Dashboard: PnL, trade history, active orders, staking rewards (data via subgraph + wallet connect) |
| **leaderboard** | leaderboard.oak.trade | Top Traders & Top Farmers; scores = Volume × Recency; data from Cron job → `data/rankings.json` → API |
| **bridge** | bridge.oak.trade | Bridge assets to Arbitrum via LayerZero Stargate + links to Axelar Satellite & Arbitrum Official Bridge |

## Running locally

From **repo root** (after `npm install` and `npm run build:ui`):

```bash
npm run dev:profile     # http://localhost:3001
npm run dev:leaderboard # http://localhost:3002
npm run dev:bridge      # http://localhost:3003
```

## Leaderboard Cron (Volume × Recency)

Rankings are recomputed by a Cron job that fetches traders from the Oak subgraph and writes `data/rankings.json`. The app’s `GET /api/rankings` serves this file when present; on Vercel (no file) it recomputes from the subgraph on the fly.

**Run manually:**

```bash
npm run rankings
# or: cd apps/leaderboard && SUBGRAPH_URL=https://... node scripts/recompute-rankings.js
```

**Vercel Cron:** add to `vercel.json` in `apps/leaderboard`:

```json
{
  "crons": [{ "path": "/api/rankings", "schedule": "0 * * * *" }]
}
```

Or run the script in **GitHub Actions** on a schedule and commit `data/rankings.json`.

## Environment

- **profile**: `NEXT_PUBLIC_OAK_SUBGRAPH_URL`, `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID`
- **leaderboard**: `NEXT_PUBLIC_OAK_SUBGRAPH_URL` or `SUBGRAPH_URL` (for cron script)
- **bridge**: no required env (Stargate widget loads from CDN)

## Shared UI

All apps use `@oak-protocol/ui` (see `packages/ui`): `Card`, `Button`, `Skeleton`, `Badge`, `OakSiteHeader`, Tailwind preset with Oak theme (e.g. `oak-bg`, `oak-accent`, `rounded-oak`).

Build the UI package first: `npm run build:ui`.
