"use client";

import { motion } from "framer-motion";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  OakSiteHeader,
  Skeleton,
} from "@oak-protocol/ui";
import { useRankings } from "@/lib/useRankings";
import type { RankingEntry } from "@/lib/rankings";

function formatAddress(addr: string): string {
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

function formatUsd(n: number): string {
  return new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", maximumFractionDigits: 0 }).format(n);
}

function RankTable({
  title,
  entries,
  isLoading,
}: {
  title: string;
  entries: RankingEntry[];
  isLoading?: boolean;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>Score = Volume × Recency (decay ~1 week)</CardDescription>
      </CardHeader>
      <CardContent>
        <ul className="space-y-2">
          {isLoading ? (
            [...Array(5)].map((_, i) => (
              <li key={i} className="flex items-center justify-between py-2">
                <Skeleton className="h-5 w-32" />
                <Skeleton className="h-5 w-24" />
              </li>
            ))
          ) : entries.length === 0 ? (
            <li className="text-oak-text-muted text-sm">No data yet. Run cron to populate.</li>
          ) : (
            entries.slice(0, 20).map((e, i) => (
              <motion.li
                key={e.address}
                initial={{ opacity: 0, x: -8 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: i * 0.03 }}
                className="flex items-center justify-between py-2 border-b border-oak-border last:border-0"
              >
                <span className="flex items-center gap-3">
                  <span className="text-oak-text-muted w-6">#{i + 1}</span>
                  <span className="font-mono text-sm">{formatAddress(e.address)}</span>
                </span>
                <span className="text-oak-text-secondary text-sm">
                  {formatUsd(e.volumeUsd)} · {e.score.toFixed(1)}
                </span>
              </motion.li>
            ))
          )}
        </ul>
      </CardContent>
    </Card>
  );
}

export default function LeaderboardPage() {
  const { traders, farmers, updatedAt, isLoading } = useRankings();

  return (
    <div className="min-h-screen bg-oak-bg text-oak-text-primary">
      <OakSiteHeader current="leaderboard" />
      <main className="p-4 md:p-6">
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          className="max-w-4xl mx-auto space-y-6"
        >
          <header>
            <h1 className="text-2xl md:text-3xl font-bold tracking-tight">Leaderboard</h1>
            <p className="text-oak-text-secondary mt-1">
              Top Traders & Top Farmers · leaderboard.oak.trade
            </p>
            <p className="text-oak-text-muted text-sm mt-1">
              Score = Volume × Recency. Last update:{" "}
              {isLoading ? <Skeleton className="inline-block h-4 w-36 align-middle" /> : new Date(updatedAt).toLocaleString()}
            </p>
          </header>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <RankTable title="Top Traders" entries={traders} isLoading={isLoading} />
            <RankTable title="Top Farmers" entries={farmers} isLoading={isLoading} />
          </div>
        </motion.div>
      </main>
    </div>
  );
}
