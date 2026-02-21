"use client";

import { useEffect, useRef, useState } from "react";

/** Line point for lightweight-charts (time as UTCTimestamp = seconds since epoch) */
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
  bg: "#0a0a0b",
  card: "#161618",
  border: "#2a2a2e",
  text: "#a1a1aa",
  accent: "#22c55e",
} as const;

/**
 * GMX-style dark line chart using oak palette.
 * Lightweight-charts is loaded only on the client to avoid "LineSeries is not defined" / addSeries SSR issues.
 */
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
        // @ts-expect-error lightweight-charts Time type is strict; our UTCTimestamp number is valid at runtime
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
    // @ts-expect-error lightweight-charts Time type is strict; our UTCTimestamp number is valid at runtime
    seriesRef.current.setData(data);
  }, [data]);

  return (
    <div className="w-full max-w-md rounded-oak-lg border border-oak-border bg-oak-bg-card shadow-oak">
      <div className="flex items-center justify-between px-4 pt-4">
        <div>
          <h2 className="text-sm font-medium text-oak-text-secondary">Price Chart</h2>
          <p className="text-xs text-oak-text-muted">Placeholder data · 1H</p>
        </div>
      </div>
      <div className="mt-3 px-3 pb-4">
        <div
          ref={containerRef}
          className="w-full rounded-md bg-oak-bg-elevated"
          style={{ height: `${CHART_HEIGHT}px` }}
        />
      </div>
    </div>
  );
}
