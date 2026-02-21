"use client";

import { useMemo } from "react";
import { motion } from "framer-motion";
import { Header } from "@/components/Header";
import { SwapWidget } from "@/components/SwapWidget";
import { PoolInfo } from "@/components/PoolInfo";
import { PriceChart, type ChartLinePoint } from "@/components/PriceChart";
import { Footer } from "@/components/Footer";
import {
  getPlaceholderPoolData,
  getPlaceholderSwapHandler,
} from "@/lib/placeholders";

function getPlaceholderChartData(): ChartLinePoint[] {
  const base = Math.floor(Date.now() / 1000) - 24 * 3600;
  return Array.from({ length: 40 }).map((_, idx) => ({
    time: base + idx * 3600,
    value: 1000 + Math.sin(idx / 4) * 20 + idx * 2,
  }));
}

export default function HomePage() {
  const poolData = useMemo(() => getPlaceholderPoolData(), []);
  const swapHandler = useMemo(() => getPlaceholderSwapHandler(), []);
  const placeholderChartData = useMemo(() => getPlaceholderChartData(), []);

  return (
    <div className="flex min-h-screen flex-col">
      <Header />
      <main className="mx-auto flex w-full max-w-6xl flex-1 flex-col items-center gap-8 px-4 py-8 sm:px-6 sm:py-10 lg:px-8 lg:py-14 xl:max-w-7xl 2xl:max-w-[1600px] 2xl:px-10">
        <section className="grid w-full grid-cols-1 gap-6 sm:gap-8 md:grid-cols-2 md:gap-8 xl:grid-cols-[1.4fr_1fr_0.9fr] xl:gap-8 2xl:gap-10 items-start">
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, ease: [0.25, 0.46, 0.45, 0.94] }}
            className="w-full flex justify-center lg:justify-start"
          >
            <PriceChart data={placeholderChartData} />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.08, ease: [0.25, 0.46, 0.45, 0.94] }}
            className="w-full flex justify-center"
          >
            <SwapWidget
              token0Symbol={poolData.token0Symbol}
              token1Symbol={poolData.token1Symbol}
              token0Balance={poolData.token0Balance}
              estimatedOutput={poolData.estimatedOutput}
              isLoadingQuote={poolData.isLoadingQuote}
              onSwap={swapHandler}
              error={poolData.error}
            />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.12, ease: [0.25, 0.46, 0.45, 0.94] }}
            className="w-full flex justify-center"
          >
            <PoolInfo
              reserve0={poolData.reserve0}
              reserve1={poolData.reserve1}
              token0Symbol={poolData.token0Symbol}
              token1Symbol={poolData.token1Symbol}
              twapPrice={poolData.twapPrice}
              twap24h={poolData.twap24h}
              apyOrFees={poolData.apyOrFees}
              isLoading={poolData.isLoadingPool}
            />
          </motion.div>
        </section>
      </main>
      <Footer />
    </div>
  );
}
