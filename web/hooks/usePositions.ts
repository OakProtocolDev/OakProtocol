"use client";

import { useCallback, useMemo } from "react";
import { useAccount, useReadContract, useReadContracts, useWriteContract } from "wagmi";
import {
  OAK_CONTRACT_ADDRESS,
  oakOrderAbi,
} from "@/lib/oakContract";

const CONTRACT = OAK_CONTRACT_ADDRESS as `0x${string}`;
const ZERO = "0x0000000000000000000000000000000000000000";

export interface PositionData {
  positionId: bigint;
  owner: `0x${string}`;
  baseToken: `0x${string}`;
  quoteToken: `0x${string}`;
  size: bigint;
  entryPrice: bigint;
  tpPrice: bigint;
  slPrice: bigint;
  trailingDeltaBps: bigint;
  trailingPeakPrice: bigint;
  initialCollateral: bigint;
  marginAdded: bigint;
  openedAt: bigint;
  status: bigint; // 0 Open, 1 Closed
}

export interface PositionHealth {
  liquidationPrice: bigint;
  healthFactorBps: bigint; // 10000 = 1.0; > 10000 = healthy
}

/**
 * Fetch next_position_id from contract.
 */
export function useNextPositionId() {
  const { data: nextId, isLoading, refetch } = useReadContract({
    address: OAK_CONTRACT_ADDRESS !== ZERO ? CONTRACT : undefined,
    abi: oakOrderAbi,
    functionName: "get_next_position_id",
  });
  return { nextId: nextId ?? 0n, isLoading, refetch };
}

/**
 * Fetch a single position by ID.
 */
export function usePosition(positionId: bigint | undefined) {
  const { data, isLoading, error, refetch } = useReadContract({
    address: OAK_CONTRACT_ADDRESS !== ZERO ? CONTRACT : undefined,
    abi: oakOrderAbi,
    functionName: "get_position",
    args: positionId !== undefined ? [positionId] : undefined,
  });
  if (!data) return { position: undefined, isLoading, error, refetch };
  const [
    owner,
    baseToken,
    quoteToken,
    size,
    entryPrice,
    tpPrice,
    slPrice,
    trailingDeltaBps,
    trailingPeakPrice,
    initialCollateral,
    marginAdded,
    openedAt,
    status,
  ] = data as readonly [string, string, string, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint];
  const position: PositionData = {
    positionId: positionId!,
    owner: owner as `0x${string}`,
    baseToken: baseToken as `0x${string}`,
    quoteToken: quoteToken as `0x${string}`,
    size,
    entryPrice,
    tpPrice,
    slPrice,
    trailingDeltaBps,
    trailingPeakPrice,
    initialCollateral,
    marginAdded,
    openedAt,
    status,
  };
  return { position, isLoading, error, refetch };
}

/**
 * Fetch all open positions for the connected user.
 * Scans position IDs from 1 to next_position_id - 1 and filters by owner and status.
 */
export function useOpenPositions() {
  const { address } = useAccount();
  const { nextId, isLoading: loadingNext, refetch: refetchNext } = useNextPositionId();

  const ids = useMemo(() => {
    if (nextId === 0n) return [];
    const list: bigint[] = [];
    for (let i = 1n; i < nextId; i++) list.push(i);
    return list;
  }, [nextId]);

  const contracts = useMemo(
    () =>
      ids.map((id) => ({
        address: CONTRACT,
        abi: oakOrderAbi,
        functionName: "get_position" as const,
        args: [id] as const,
      })),
    [ids]
  );

  const { data: results, isLoading: loadingPositions, refetch: refetchPositions } = useReadContracts({
    contracts: OAK_CONTRACT_ADDRESS !== ZERO && ids.length > 0 ? contracts : [],
  });

  const positions = useMemo((): PositionData[] => {
    if (!address || !results?.length) return [];
    const list: PositionData[] = [];
    results.forEach((r, i) => {
      if (r.status !== "success" || !r.result) return;
      const res = r.result as readonly [string, string, string, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint];
      const [owner, baseToken, quoteToken, size, entryPrice, tpPrice, slPrice, trailingDeltaBps, trailingPeakPrice, initialCollateral, marginAdded, openedAt, status] = res;
      const id = ids[i];
      if (id === undefined) return;
      if (String(owner).toLowerCase() !== address.toLowerCase()) return;
      if (status !== 0n) return; // only open
      list.push({
        positionId: id,
        owner: owner as `0x${string}`,
        baseToken: baseToken as `0x${string}`,
        quoteToken: quoteToken as `0x${string}`,
        size,
        entryPrice,
        tpPrice,
        slPrice,
        trailingDeltaBps,
        trailingPeakPrice,
        initialCollateral,
        marginAdded,
        openedAt,
        status,
      });
    });
    return list;
  }, [address, results, ids]);

  const refetch = useCallback(() => {
    refetchNext?.();
    refetchPositions?.();
  }, [refetchNext, refetchPositions]);

  return {
    positions,
    isLoading: loadingNext || loadingPositions,
    refetch,
  };
}

/**
 * Write hooks for positions.
 */
export function usePositionActions() {
  const { writeContractAsync, isPending } = useWriteContract();

  const openPosition = async (
    baseToken: `0x${string}`,
    quoteToken: `0x${string}`,
    size: string,
    entryPrice: string,
    initialCollateral: string = "0"
  ) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "open_position",
      args: [baseToken, quoteToken, BigInt(size), BigInt(entryPrice), BigInt(initialCollateral)],
    });
  };

  const addMargin = async (positionId: bigint, amount: string) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "add_margin",
      args: [positionId, BigInt(amount)],
    });
  };

  const setPositionTpSl = async (
    positionId: bigint,
    tpPrice: string,
    slPrice: string
  ) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "set_position_tp_sl",
      args: [positionId, BigInt(tpPrice), BigInt(slPrice)],
    });
  };

  const closePosition = async (positionId: bigint, minAmountOut: string) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "close_position",
      args: [positionId, BigInt(minAmountOut)],
    });
  };

  const executePositionTpSl = async (positionId: bigint, minAmountOut: string) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "execute_position_tp_sl",
      args: [positionId, BigInt(minAmountOut)],
    });
  };

  const setPositionTrailingStop = async (positionId: bigint, trailingDeltaBps: string) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "set_position_trailing_stop",
      args: [positionId, BigInt(trailingDeltaBps)],
    });
  };

  const updateTrailingStop = async (positionId: bigint, newPrice: string, minAmountOut: string) => {
    if (OAK_CONTRACT_ADDRESS === ZERO) throw new Error("Contract not configured");
    return writeContractAsync({
      address: CONTRACT,
      abi: oakOrderAbi,
      functionName: "update_trailing_stop",
      args: [positionId, BigInt(newPrice), BigInt(minAmountOut)],
    });
  };

  return {
    openPosition,
    addMargin,
    setPositionTpSl,
    setPositionTrailingStop,
    updateTrailingStop,
    closePosition,
    executePositionTpSl,
    isPending,
  };
}

/**
 * Read liquidation price and health factor for a position.
 * health_factor_bps: 10000 = 1.0; > 10000 = healthy; <= 10000 = at or below liquidation.
 */
export function usePositionHealth(positionId: bigint | undefined) {
  const { data, isLoading, error, refetch } = useReadContract({
    address: OAK_CONTRACT_ADDRESS !== ZERO ? CONTRACT : undefined,
    abi: oakOrderAbi,
    functionName: "get_position_health",
    args: positionId !== undefined ? [positionId] : undefined,
  });
  const health: PositionHealth | undefined = data
    ? { liquidationPrice: (data as [bigint, bigint])[0], healthFactorBps: (data as [bigint, bigint])[1] }
    : undefined;
  return { health, isLoading, error, refetch };
}
