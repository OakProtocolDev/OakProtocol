/**
 * Leaderboard data: Top Traders / Top Farmers.
 * Populated by Cron job (scripts/recompute-rankings.js) that writes data/rankings.json.
 * Score = Volume × Recency (time decay ~1 week).
 */

export interface RankingEntry {
  address: string;
  volumeUsd: number;
  score: number;
  lastActivityAt?: number;
}

export interface RankingsData {
  traders: RankingEntry[];
  farmers: RankingEntry[];
  updatedAt: number;
}
