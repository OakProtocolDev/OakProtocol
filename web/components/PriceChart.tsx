"use client";

import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";

export interface ChartLinePoint {
  time: number;
  value: number;
}

export interface PriceChartProps {
  data?: ChartLinePoint[];
  height?: number;
}

const CHART_HEIGHT = 260;
const OAK_COLORS = {
  bg: "#050807",
  card: "#0f1613",
  border: "#1a2520",
  text: "#a1a1aa",
  accent: "#22c55e",
} as const;

export function PriceChart({ data, height = CHART_HEIGHT }: PriceChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<ReturnType<typeof import("lightweight-charts").createChart> | null>(null);
  const seriesRef = useRef<ReturnType<
    ReturnType<typeof import("lightweight-charts").createChart>["addSeries"]
  > | null>(null);
  const cleanupRef = useRef<(() => void) | null>(null);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!mounted || typeof window === "undefined" || !containerRef.current) return;

    let chart: ReturnType<typeof import("lightweight-charts").createChart> | null = null;
    let cancelled = false;

    const init = async () => {
      const lw = await import("lightweight-charts");
      if (cancelled) return;
      const { createChart, LineSeries } = lw;
      if (!containerRef.current) return;

      chart = createChart(containerRef.current, {
        height,
        layout: {
          background: { color: OAK_COLORS.bg },
          textColor: OAK_COLORS.text,
        },
        grid: {
          vertLines: { color: OAK_COLORS.card },
          horzLines: { color: OAK_COLORS.card },
        },
        rightPriceScale: { borderColor: OAK_COLORS.border },
        timeScale: { borderColor: OAK_COLORS.border },
        crosshair: {
          vertLine: {
            color: OAK_COLORS.accent,
            width: 1,
            style: 0,
            labelBackgroundColor: OAK_COLORS.accent,
          },
          horzLine: {
            color: OAK_COLORS.accent,
            width: 1,
            style: 0,
            labelBackgroundColor: OAK_COLORS.accent,
          },
          mode: 1,
        },
        localization: { priceFormatter: (p: number) => p.toFixed(4) },
      });

      const series = chart.addSeries(LineSeries, {
        color: OAK_COLORS.accent,
        lineWidth: 2,
        priceLineVisible: false,
        lastValueVisible: true,
        crosshairMarkerVisible: true,
        crosshairMarkerRadius: 3,
      });

      if (cancelled) {
        chart.remove();
        return () => {};
      }

      chartRef.current = chart;
      seriesRef.current = series;

      if (data && data.length > 0) {
        // @ts-expect-error lightweight-charts Time type
        series.setData(data);
      }

      const onResize = () => {
        if (containerRef.current && chart) {
          chart.applyOptions({ width: containerRef.current.clientWidth });
        }
      };
      onResize();
      window.addEventListener("resize", onResize);

      const cleanupFn = () => {
        window.removeEventListener("resize", onResize);
        chart?.remove();
        chartRef.current = null;
        seriesRef.current = null;
      };
      cleanupRef.current = cleanupFn;
      return cleanupFn;
    };

    init();

    return () => {
      cancelled = true;
      const fn = cleanupRef.current;
      cleanupRef.current = null;
      if (fn) fn();
    };
  }, [mounted, height]);

  useEffect(() => {
    if (!seriesRef.current || !data || data.length === 0) return;
    // @ts-expect-error lightweight-charts Time type
    seriesRef.current.setData(data);
  }, [data]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.25, 0.46, 0.45, 0.94] }}
      className="glass-card w-full max-w-md overflow-hidden"
      style={{
        background: "rgba(15, 22, 19, 0.55)",
        backdropFilter: "blur(20px)",
        border: "1px solid rgba(34, 197, 94, 0.08)",
      }}
    >
      <div className="flex items-center justify-between px-4 pt-4">
        <div>
          <h2 className="text-sm font-medium text-oak-text-secondary">Price Chart</h2>
          <p className="text-xs text-oak-text-muted">Placeholder data · 1H</p>
        </div>
      </div>
      <div className="mt-3 px-3 pb-4">
        <div
          ref={containerRef}
          className="w-full rounded-md bg-oak-bg-elevated/50 transition-colors hover:bg-oak-bg-elevated/70"
          style={{ height: `${CHART_HEIGHT}px` }}
        />
      </div>
    </motion.div>
  );
}
