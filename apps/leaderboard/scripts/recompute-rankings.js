/**
 * Cron job: recompute leaderboard rankings (Volume × Recency).
 * Run periodically (e.g. every hour) via GitHub Actions or Vercel Cron.
 *
 * Formula: score = volume_usd * exp(-lambda * (now - last_activity_ts))
 * Data source: Subgraph or indexer API. Output: write to data/rankings.json or POST to API.
 *
 * Usage:
 *   node scripts/recompute-rankings.js
 *   # Or: SUBGRAPH_URL=https://... node scripts/recompute-rankings.js
 */

const fs = require("fs");
const path = require("path");

const LAMBDA = 1 / (24 * 3600 * 7); // decay over ~1 week in seconds

function recencyDecay(nowSec, lastActivitySec) {
  const age = Math.max(0, nowSec - lastActivitySec);
  return Math.exp(-LAMBDA * age);
}

function computeScores(entries) {
  const now = Math.floor(Date.now() / 1000);
  return entries
    .map((e) => ({
      ...e,
      score: (e.volumeUsd || 0) * recencyDecay(now, e.lastActivityAt || now),
    }))
    .sort((a, b) => b.score - a.score)
    .slice(0, 100);
}

async function fetchFromSubgraph(url, query) {
  try {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query }),
    });
    const json = await res.json();
    return json?.data || null;
  } catch (e) {
    console.error("Subgraph fetch failed:", e.message);
    return null;
  }
}

async function fetchTradersFromSubgraph() {
  const url = process.env.SUBGRAPH_URL || process.env.NEXT_PUBLIC_OAK_SUBGRAPH_URL;
  if (!url) return null;
  const query = `
    query { traders(first: 200, orderBy: totalVolumeUsd, orderDirection: desc) {
      id
      totalVolumeUsd
      lastTradeTimestamp
    }}
  `;
  const data = await fetchFromSubgraph(url, query);
  const list = data?.traders || [];
  return list.map((t) => ({
    address: t.id,
    volumeUsd: parseFloat(t.totalVolumeUsd || 0),
    lastActivityAt: parseInt(t.lastTradeTimestamp || "0", 10) || Math.floor(Date.now() / 1000),
  }));
}

/** Optional: Top Farmers from subgraph (e.g. liquidityProviders or addLiquidity events). */
async function fetchFarmersFromSubgraph() {
  const url = process.env.SUBGRAPH_URL || process.env.NEXT_PUBLIC_OAK_SUBGRAPH_URL;
  if (!url) return [];
  // If your subgraph has liquidityProviders: { id, totalLiquidityUsd, lastActivityTimestamp }
  const query = `
    query { liquidityProviders(first: 200, orderBy: totalLiquidityUsd, orderDirection: desc) {
      id
      totalLiquidityUsd
      lastActivityTimestamp
    }}
  `;
  const data = await fetchFromSubgraph(url, query);
  const list = data?.liquidityProviders || [];
  return list.map((f) => ({
    address: f.id,
    volumeUsd: parseFloat(f.totalLiquidityUsd || 0),
    lastActivityAt: parseInt(f.lastActivityTimestamp || "0", 10) || Math.floor(Date.now() / 1000),
  }));
}

async function main() {
  let traderEntries = await fetchTradersFromSubgraph();
  if (!traderEntries || traderEntries.length === 0) {
    const now = Math.floor(Date.now() / 1000);
    traderEntries = [
      { address: "0x1234567890123456789012345678901234567890", volumeUsd: 150000, lastActivityAt: now - 3600 },
      { address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd", volumeUsd: 98000, lastActivityAt: now - 86400 },
      { address: "0x9876543210987654321098765432109876543210", volumeUsd: 72000, lastActivityAt: now - 172800 },
    ];
  }

  const traders = computeScores(traderEntries);
  const farmerEntries = await fetchFarmersFromSubgraph();
  const farmers = computeScores(farmerEntries);

  const output = {
    traders,
    farmers,
    updatedAt: Date.now(),
  };

  const dataDir = path.join(process.cwd(), "data");
  if (!fs.existsSync(dataDir)) fs.mkdirSync(dataDir, { recursive: true });
  const outPath = path.join(dataDir, "rankings.json");
  fs.writeFileSync(outPath, JSON.stringify(output, null, 2), "utf-8");
  console.log("Wrote", outPath);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
