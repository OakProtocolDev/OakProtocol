"use client";

import { useCallback } from "react";
import { useWriteContract } from "wagmi";
import {
  OAK_CONTRACT_ADDRESS,
  oakOrderAbi,
} from "@/lib/oakContract";

/**
 * Hook for one-click close position (market sell full amount).
 */
export function useClosePosition() {
  const { writeContractAsync, isPending } = useWriteContract();

  const closePosition = useCallback(
    async (
      amountIn: string,
      tokenFrom: `0x${string}`,
      tokenTo: `0x${string}`,
      minAmountOut: string
    ) => {
      if (OAK_CONTRACT_ADDRESS === "0x0000000000000000000000000000000000000000") {
        throw new Error("Contract not configured");
      }
      return writeContractAsync({
        address: OAK_CONTRACT_ADDRESS as `0x${string}`,
        abi: oakOrderAbi,
        functionName: "close_position_market",
        args: [
          BigInt(amountIn),
          tokenFrom,
          tokenTo,
          BigInt(minAmountOut),
        ],
      });
    },
    [writeContractAsync]
  );

  return {
    closePosition,
    isClosing: isPending,
  };
}
