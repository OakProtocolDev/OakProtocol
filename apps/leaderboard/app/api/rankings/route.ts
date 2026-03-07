/**
 * GET /api/rankings
 * Returns Top Traders and Top Farmers (Volume × Recency).
 * Populated by Cron job (scripts/recompute-rankings.js); falls back to subgraph or placeholder.
 */

import { NextResponse } from "next/server";
import { readFileSync, existsSync } from "fs";
import { join } from "path";
import type { RankingsData } from "@/lib/rankings";

const LAMBDA = 1 / (24 * 3600 * 7); // decay over ~1 week

function recencyDecay(nowSec: number, lastActivitySec: number): number {
  const age = Math.max(0, nowSec - lastActivitySec);
  return Math.exp(-LAMBDA * age);
}

function computeScores<T extends { volumeUsd?: number; lastActivityAt?: number }>(
  entries: T[]
): (T & { score: number })[] {
  const now = Math.floor(Date.now() / 1000);
  return entries
    .map((e) => ({
      ...e,
      score: (e.volumeUsd ?? 0) * recencyDecay(now, e.lastActivityAt ?? now),
    }))
    .sort((a, b) => b.score - a.score)
    .slice(0, 100) as (T & { score: number })[];
}

function getPlaceholderRankings(): RankingsData {
  const now = Math.floor(Date.now() / 1000);
  const placeholderTraders = [
    { address: "0x1234567890123456789012345678901234567890", volumeUsd: 150000, lastActivityAt: now - 3600 },
    { address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd", volumeUsd: 98000, lastActivityAt: now - 86400 },
    { address: "0x9876543210987654321098765432109876543210", volumeUsd: 72000, lastActivityAt: now - 172800 },
  ];
  const traders = computeScores(placeholderTraders);
  const farmers = computeScores([]);
  return {
    traders: traders.map((t) => ({ address: t.address, volumeUsd: t.volumeUsd ?? 0, score: t.score, lastActivityAt: t.lastActivityAt })),
    farmers,
    updatedAt: Date.now(),
  };
}

const SUBGRAPH_URL = process.env.SUBGRAPH_URL || process.env.NEXT_PUBLIC_OAK_SUBGRAPH_URL;

async function fetchRankingsFromSubgraph(): Promise<RankingsData | null> {
  if (!SUBGRAPH_URL) return null;
  try {
    const query = `
      query { traders(first: 200, orderBy: totalVolumeUsd, orderDirection: desc) {
        id
        totalVolumeUsd
        lastTradeTimestamp
      }}
    `;
    const res = await fetch(SUBGRAPH_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query }),
    });
    const json = await res.json();
    const list = json?.data?.traders ?? [];
    const entries = list.map((t: { id: string; totalVolumeUsd?: string; lastTradeTimestamp?: string }) => ({
      address: t.id,
      volumeUsd: parseFloat(t.totalVolumeUsd || "0"),
      lastActivityAt: parseInt(t.lastTradeTimestamp || "0", 10) || Math.floor(Date.now() / 1000),
    }));
    const traders = computeScores(entries);
    const farmers = computeScores([]);
    return {
      traders: traders.map((t) => ({ address: t.address, volumeUsd: t.volumeUsd, score: t.score, lastActivityAt: t.lastActivityAt })),
      farmers,
      updatedAt: Date.now(),
    };
  } catch {
    return null;
  }
}

/** Prefer cron-written file, then subgraph on-the-fly, then placeholder. */
async function getRankingsData(): Promise<RankingsData> {
  const dataPath = join(process.cwd(), "data", "rankings.json");
  if (existsSync(dataPath)) {
    try {
      const raw = readFileSync(dataPath, "utf-8");
      const data = JSON.parse(raw) as RankingsData;
      if (data?.traders && Array.isArray(data.traders) && typeof data.updatedAt === "number") {
        return data;
      }
    } catch {
      // fall through
    }
  }
  const fromSubgraph = await fetchRankingsFromSubgraph();
  if (fromSubgraph) return fromSubgraph;
  return getPlaceholderRankings();
}

export async function GET() {
  try {
    const data = await getRankingsData();
    return NextResponse.json(data);
  } catch (e) {
    return NextResponse.json(
      { error: "Failed to load rankings" },
      { status: 500 }
    );
  }
}
