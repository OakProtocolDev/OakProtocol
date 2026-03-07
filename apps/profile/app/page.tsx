"use client";

import { motion } from "framer-motion";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useAccount } from "wagmi";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Skeleton,
  OakSiteHeader,
} from "@oak-protocol/ui";
import { useQuery } from "@tanstack/react-query";
import {
  fetchUserStats,
  fetchUserTrades,
  fetchUserOrders,
  fetchUserPositions,
  fetchStakingRewards,
  type TradeRow,
  type OrderRow,
  type PositionRow,
  type StakingRewardRow,
} from "@/lib/subgraph";

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.06 },
  },
};

const item = {
  hidden: { opacity: 0, y: 12 },
  show: { opacity: 1, y: 0 },
};

function formatUsd(value: string | number): string {
  const n = typeof value === "string" ? parseFloat(value) : value;
  if (Number.isNaN(n)) return "$0.00";
  return new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", minimumFractionDigits: 2 }).format(n);
}

function formatShortAddress(addr: string): string {
  if (!addr) return "—";
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

export default function ProfileDashboardPage() {
  const { address } = useAccount();
  const user = address ?? "";

  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ["profile-stats", user],
    queryFn: () => fetchUserStats(user),
    enabled: !!user,
  });

  const { data: trades = [], isLoading: tradesLoading } = useQuery({
    queryKey: ["profile-trades", user],
    queryFn: () => fetchUserTrades(user, 10),
    enabled: !!user,
  });

  const { data: orders = [], isLoading: ordersLoading } = useQuery({
    queryKey: ["profile-orders", user],
    queryFn: () => fetchUserOrders(user, 20),
    enabled: !!user,
  });

  const { data: positions = [], isLoading: positionsLoading } = useQuery({
    queryKey: ["profile-positions", user],
    queryFn: () => fetchUserPositions(user, 10),
    enabled: !!user,
  });

  const { data: stakingRewards = [], isLoading: stakingLoading } = useQuery({
    queryKey: ["profile-staking", user],
    queryFn: () => fetchStakingRewards(user, 10),
    enabled: !!user,
  });

  return (
    <div className="min-h-screen bg-oak-bg text-oak-text-primary">
      <OakSiteHeader current="profile" />
      <main className="p-4 md:p-6">
        <motion.div
          variants={container}
          initial="hidden"
          animate="show"
          className="max-w-6xl mx-auto space-y-6"
        >
          <motion.header variants={item} className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
            <div>
              <h1 className="text-2xl md:text-3xl font-bold tracking-tight">Universal Dashboard</h1>
              <p className="text-oak-text-secondary mt-0.5">
                PnL, trade history, active orders & staking rewards · profile.oak.trade
              </p>
              {user && (
                <p className="text-oak-text-muted text-sm mt-1 font-mono">{formatShortAddress(user)}</p>
              )}
            </div>
            <ConnectButton />
          </motion.header>

          {!user && (
            <motion.div variants={item}>
              <Card>
                <CardContent className="py-8 text-center text-oak-text-secondary">
                  Connect your wallet to view your PnL, trades, orders and staking rewards.
                </CardContent>
              </Card>
            </motion.div>
          )}

          {user && (
        <>
        {/* Stats row */}
        <motion.section variants={item} className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Total PnL</CardDescription>
              <CardTitle className="text-xl">
                {statsLoading ? <Skeleton className="h-7 w-24" /> : formatUsd(stats?.pnlUsd ?? 0)}
              </CardTitle>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Volume (USD)</CardDescription>
              <CardTitle className="text-xl">
                {statsLoading ? <Skeleton className="h-7 w-24" /> : formatUsd(stats?.totalVolumeUsd ?? 0)}
              </CardTitle>
            </CardHeader>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Trades</CardDescription>
              <CardTitle className="text-xl">
                {statsLoading ? <Skeleton className="h-7 w-8" /> : stats?.tradeCount ?? 0}
              </CardTitle>
            </CardHeader>
          </Card>
        </motion.section>

        {/* Trades & Orders */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <motion.div variants={item}>
            <Card>
              <CardHeader>
                <CardTitle>Recent Trades</CardTitle>
                <CardDescription>From subgraph</CardDescription>
              </CardHeader>
              <CardContent>
                {tradesLoading ? (
                  <div className="space-y-2">
                    {[1, 2, 3].map((i) => (
                      <Skeleton key={i} className="h-8 w-full" />
                    ))}
                  </div>
                ) : trades.length === 0 ? (
                  <p className="text-oak-text-muted text-sm">No trades yet. Connect wallet and trade on Oak.</p>
                ) : (
                  <ul className="space-y-2">
                    {trades.slice(0, 5).map((t: TradeRow) => (
                      <li
                        key={t.id}
                        className="flex justify-between text-sm border-b border-oak-border pb-2 last:border-0"
                      >
                        <span className="text-oak-text-secondary">
                          {t.tokenIn?.slice(0, 6)}… → {t.tokenOut?.slice(0, 6)}…
                        </span>
                        <span>{formatUsd(t.amountOut)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </motion.div>

          <motion.div variants={item}>
            <Card>
              <CardHeader>
                <CardTitle>Active Orders</CardTitle>
                <CardDescription>Limit / TP / SL</CardDescription>
              </CardHeader>
              <CardContent>
                {ordersLoading ? (
                  <div className="space-y-2">
                    {[1, 2, 3].map((i) => (
                      <Skeleton key={i} className="h-8 w-full" />
                    ))}
                  </div>
                ) : orders.length === 0 ? (
                  <p className="text-oak-text-muted text-sm">No open orders.</p>
                ) : (
                  <ul className="space-y-2">
                    {orders.slice(0, 5).map((o: OrderRow) => (
                      <li
                        key={o.id}
                        className="flex justify-between text-sm border-b border-oak-border pb-2 last:border-0"
                      >
                        <span className="text-oak-text-secondary">
                          Order #{o.orderId} · {o.orderType === 0 ? "Limit" : o.orderType === 1 ? "TP" : "SL"}
                        </span>
                        <span>@{formatUsd(o.triggerPrice)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </motion.div>
        </div>

        {/* Positions & Staking */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <motion.div variants={item}>
            <Card>
              <CardHeader>
                <CardTitle>Open Positions</CardTitle>
                <CardDescription>With PnL</CardDescription>
              </CardHeader>
              <CardContent>
                {positionsLoading ? (
                  <div className="space-y-2">
                    {[1, 2, 3].map((i) => (
                      <Skeleton key={i} className="h-8 w-full" />
                    ))}
                  </div>
                ) : positions.length === 0 ? (
                  <p className="text-oak-text-muted text-sm">No open positions.</p>
                ) : (
                  <ul className="space-y-2">
                    {positions.slice(0, 5).map((p: PositionRow) => (
                      <li
                        key={p.id}
                        className="flex justify-between text-sm border-b border-oak-border pb-2 last:border-0"
                      >
                        <span className="text-oak-text-secondary">#{p.positionId}</span>
                        <span>
                          Size: {p.size}
                          {p.pnl != null && p.pnl !== "" && (
                            <span className={Number(p.pnl) >= 0 ? " text-oak-accent" : " text-oak-error"}>
                              {" "}· PnL {formatUsd(p.pnl)}
                            </span>
                          )}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </motion.div>

          <motion.div variants={item}>
            <Card>
              <CardHeader>
                <CardTitle>Staking Rewards</CardTitle>
                <CardDescription>Current rewards</CardDescription>
              </CardHeader>
              <CardContent>
                {stakingLoading ? (
                  <div className="space-y-2">
                    {[1, 2, 3].map((i) => (
                      <Skeleton key={i} className="h-8 w-full" />
                    ))}
                  </div>
                ) : stakingRewards.length === 0 ? (
                  <p className="text-oak-text-muted text-sm">No rewards yet. Stake LP tokens to earn.</p>
                ) : (
                  <ul className="space-y-2">
                    {stakingRewards.slice(0, 5).map((r: StakingRewardRow) => (
                      <li
                        key={r.id}
                        className="flex justify-between text-sm border-b border-oak-border pb-2 last:border-0"
                      >
                        <span className="text-oak-text-secondary">{r.eventType}</span>
                        <span className="text-oak-accent">{r.amount}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </motion.div>
        </div>
        </>
          )}
      </motion.div>
      </main>
    </div>
  );
}
