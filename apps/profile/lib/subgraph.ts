/**
 * Subgraph client for profile.oak.trade.
 * Fetches: PnL, trade history, active orders, staking rewards.
 * Replace SUBGRAPH_URL with your deployed Oak subgraph endpoint.
 */

const SUBGRAPH_URL =
  process.env.NEXT_PUBLIC_OAK_SUBGRAPH_URL ||
  "https://api.studio.thegraph.com/query/placeholder/oak-protocol/version/latest";

export interface TradeRow {
  id: string;
  timestamp: number;
  user: string;
  tokenIn: string;
  tokenOut: string;
  amountIn: string;
  amountOut: string;
  fee: string;
}

export interface OrderRow {
  id: string;
  orderId: string;
  owner: string;
  tokenIn: string;
  tokenOut: string;
  amountOut: string;
  triggerPrice: string;
  orderType: number;
  status: number;
  createdAt: number;
}

export interface PositionRow {
  id: string;
  positionId: string;
  owner: string;
  baseToken: string;
  quoteToken: string;
  size: string;
  entryPrice: string;
  pnl: string;
  status: number;
}

export interface StakingRewardRow {
  id: string;
  user: string;
  amount: string;
  timestamp: number;
  eventType: string;
}

export interface UserStats {
  pnlUsd: string;
  totalVolumeUsd: string;
  tradeCount: number;
}

const USER_STATS_QUERY = `
  query UserStats($user: String!) {
    user(id: $user) {
      id
      totalVolumeUsd
      pnlUsd
      tradeCount
    }
  }
`;

const TRADES_QUERY = `
  query UserTrades($user: String!, $first: Int!) {
    swaps(where: { sender: $user }, first: $first, orderBy: timestamp, orderDirection: desc) {
      id
      timestamp
      sender
      tokenIn
      tokenOut
      amountIn
      amountOut
      fee
    }
  }
`;

const ORDERS_QUERY = `
  query UserOrders($user: String!, $first: Int!) {
    orders(where: { owner: $user, status: 0 }, first: $first) {
      id
      orderId
      owner
      tokenIn
      tokenOut
      amountOut
      triggerPrice
      orderType
      status
      createdAt
    }
  }
`;

const POSITIONS_QUERY = `
  query UserPositions($user: String!, $first: Int!) {
    positions(where: { owner: $user, status: 0 }, first: $first) {
      id
      positionId
      owner
      baseToken
      quoteToken
      size
      entryPrice
      pnl
      status
    }
  }
`;

const STAKING_REWARDS_QUERY = `
  query UserStakingRewards($user: String!, $first: Int!) {
    emissionEvents(where: { user: $user, moduleId: "1" }, first: $first, orderBy: timestamp, orderDirection: desc) {
      id
      user
      amount
      timestamp
      eventType
    }
  }
`;

async function fetchSubgraph<T>(query: string, variables: Record<string, unknown>): Promise<T> {
  const res = await fetch(SUBGRAPH_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });
  if (!res.ok) throw new Error(`Subgraph error: ${res.status}`);
  const json = await res.json();
  if (json.errors) throw new Error(json.errors[0]?.message || "GraphQL error");
  return json.data as T;
}

export async function fetchUserStats(user: string): Promise<UserStats | null> {
  try {
    const data = await fetchSubgraph<{ user: UserStats | null }>(USER_STATS_QUERY, { user: user.toLowerCase() });
    return data?.user ?? null;
  } catch {
    return null;
  }
}

export async function fetchUserTrades(user: string, first = 20): Promise<TradeRow[]> {
  try {
    const data = await fetchSubgraph<{ swaps: TradeRow[] }>(TRADES_QUERY, {
      user: user.toLowerCase(),
      first,
    });
    return data?.swaps ?? [];
  } catch {
    return [];
  }
}

export async function fetchUserOrders(user: string, first = 50): Promise<OrderRow[]> {
  try {
    const data = await fetchSubgraph<{ orders: OrderRow[] }>(ORDERS_QUERY, {
      user: user.toLowerCase(),
      first,
    });
    return data?.orders ?? [];
  } catch {
    return [];
  }
}

export async function fetchUserPositions(user: string, first = 20): Promise<PositionRow[]> {
  try {
    const data = await fetchSubgraph<{ positions: PositionRow[] }>(POSITIONS_QUERY, {
      user: user.toLowerCase(),
      first,
    });
    return data?.positions ?? [];
  } catch {
    return [];
  }
}

export async function fetchStakingRewards(user: string, first = 20): Promise<StakingRewardRow[]> {
  try {
    const data = await fetchSubgraph<{ emissionEvents: StakingRewardRow[] }>(STAKING_REWARDS_QUERY, {
      user: user.toLowerCase(),
      first,
    });
    return data?.emissionEvents ?? [];
  } catch {
    return [];
  }
}
