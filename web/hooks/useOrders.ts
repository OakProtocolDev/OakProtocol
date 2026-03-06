"use client";

import { useCallback } from "react";
import { useAccount, useReadContract, useWriteContract } from "wagmi";
import {
  OAK_CONTRACT_ADDRESS,
  oakOrderAbi,
  type OrderType,
} from "@/lib/oakContract";

/**
 * Hook for TP/SL/Limit orders: place, cancel, execute, get order.
 * When contract address is zero, calls are no-ops (demo / unconfigured).
 */
export function useOrders() {
  const { address } = useAccount();
  const { writeContractAsync } = useWriteContract();

  const placeOrder = useCallback(
    async (
      tokenIn: `0x${string}`,
      tokenOut: `0x${string}`,
      amountOut: string,
      triggerPrice: string,
      orderType: OrderType,
      ocoWithOrderId: bigint = 0n
    ) => {
      if (OAK_CONTRACT_ADDRESS === "0x0000000000000000000000000000000000000000") {
        throw new Error("Contract not configured");
      }
      const hash = await writeContractAsync({
        address: OAK_CONTRACT_ADDRESS as `0x${string}`,
        abi: oakOrderAbi,
        functionName: "place_order",
        args: [
          tokenIn,
          tokenOut,
          BigInt(amountOut),
          BigInt(triggerPrice),
          BigInt(orderType),
          ocoWithOrderId,
        ],
      });
      return hash;
    },
    [writeContractAsync]
  );

  const cancelOrder = useCallback(
    async (orderId: bigint) => {
      if (OAK_CONTRACT_ADDRESS === "0x0000000000000000000000000000000000000000") {
        throw new Error("Contract not configured");
      }
      return writeContractAsync({
        address: OAK_CONTRACT_ADDRESS as `0x${string}`,
        abi: oakOrderAbi,
        functionName: "cancel_order",
        args: [orderId],
      });
    },
    [writeContractAsync]
  );

  const executeOrder = useCallback(
    async (orderId: bigint, minAmountOut: string) => {
      if (OAK_CONTRACT_ADDRESS === "0x0000000000000000000000000000000000000000") {
        throw new Error("Contract not configured");
      }
      return writeContractAsync({
        address: OAK_CONTRACT_ADDRESS as `0x${string}`,
        abi: oakOrderAbi,
        functionName: "execute_order",
        args: [orderId, BigInt(minAmountOut)],
      });
    },
    [writeContractAsync]
  );

  return {
    placeOrder,
    cancelOrder,
    executeOrder,
    isConnected: !!address,
  };
}

/**
 * Read a single order by ID.
 */
export function useOrder(orderId: bigint | undefined) {
  const { data, isLoading, error, refetch } = useReadContract({
    address:
      OAK_CONTRACT_ADDRESS !== "0x0000000000000000000000000000000000000000"
        ? (OAK_CONTRACT_ADDRESS as `0x${string}`)
        : undefined,
    abi: oakOrderAbi,
    functionName: "get_order",
    args: orderId !== undefined ? [orderId] : undefined,
  });

  return {
    order: data,
    isLoading,
    error,
    refetch,
  };
}

/**
 * Read current price for a pair (token_in, token_out).
 */
export function useCurrentPrice(
  tokenIn: `0x${string}` | undefined,
  tokenOut: `0x${string}` | undefined
) {
  const { data, isLoading, error, refetch } = useReadContract({
    address:
      OAK_CONTRACT_ADDRESS !== "0x0000000000000000000000000000000000000000"
        ? (OAK_CONTRACT_ADDRESS as `0x${string}`)
        : undefined,
    abi: oakOrderAbi,
    functionName: "get_current_price",
    args:
      tokenIn && tokenOut ? [tokenIn, tokenOut] : undefined,
  });

  return {
    price: data,
    isLoading,
    error,
    refetch,
  };
}
