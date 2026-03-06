# Price Alerts — Architecture (No Tx Spam)

## Problem

We need to store user conditions for price notifications (e.g. "notify when ETH ≥ 4000" or "when ETH ≤ 3500") **without spamming the chain**: each on-chain write is a transaction (gas, UX, rate limits).

---

## Option A: Off-chain only (recommended)

**Where:** Backend DB (Postgres, etc.) or serverless (e.g. Vercel + KV/DB).

**Schema (example):**

- `user_id` (wallet address or app user id)
- `symbol` (e.g. "ETHUSDT")
- `condition`: `"above" | "below"`
- `target_price` (decimal string or 18-decimal stored)
- `created_at`, `notified_at` (null until fired)
- Optional: `position_id` (link to Oak position), `channel` (email / push / in-app)

**Flow:**

1. User creates alert in the app → **API call** to your backend; backend writes one row. No blockchain tx.
2. **Worker / cron** (e.g. every 1–5 min) fetches current price (Binance, your DEX, oracles), runs `SELECT * FROM alerts WHERE notified_at IS NULL AND ((condition = 'above' AND current_price >= target_price) OR (condition = 'below' AND current_price <= target_price))`, sends notification, sets `notified_at`.
3. Optionally: one-time alert (delete after notify) or recurring (reset `notified_at` or use a separate "last_triggered_at").

**Pros:** No gas, no chain load, easy to add many alerts per user, rich payload (email, push).  
**Cons:** Backend required; alerts are not trustless (you could prove creation via signed message if needed).

---

## Option B: Hybrid (proof on-chain, data off-chain)

**Idea:** User signs a **message** (e.g. EIP-191 or EIP-712) that encodes `(symbol, condition, target_price, nonce)`. No transaction. Backend stores the **signature** and the decoded payload; worker checks price and sends notification. Optionally store **hash(signature)** in a contract or emit as event in a batch tx later for audit.

**Pros:** Cryptographic proof that the user requested the alert; no per-alert tx.  
**Cons:** More implementation work; still need backend + worker.

---

## Option C: On-chain storage (not recommended for many alerts)

Store in contract: e.g. `mapping(user => Alert[])` or a single slot per user with packed data. Each create/update/delete = one tx. Fine for a **small** number of alerts per user (e.g. 1–3) and if you want alerts to be fully on-chain and executable by keepers. For "notify me" only, this is usually overkill and expensive.

---

## Recommendation

- **MVP / most products:** **Option A** — backend DB + worker. No transactions for alert creation; no spam.
- **If you need proof of creation:** **Option B** — signed message + backend; optionally store commitment on-chain in batch.
- **If you need on-chain execution (e.g. auto TP/SL):** you already have **orders** and **position TP/SL** in the contract; use those. Price alerts for "notify only" stay off-chain.

---

## Frontend (Oak)

- **Demo / no backend:** Store alert conditions in **localStorage** (or in-memory). A hook (e.g. `usePriceAlerts`) can read current price from `useBinanceData` or `useTradingViewData` and compare; show in-app toast when condition met. Not persisted across devices and not reliable when tab is closed.
- **With backend:** Frontend calls `POST /api/alerts` with `{ symbol, condition, target_price }` (and auth); worker runs periodically and sends notifications; frontend can list/delete via API.

This document lives in `web/docs/` as a reference for implementing the backend and worker when you add them.
