"use client";

import { useEffect, useRef, useState } from "react";

export interface TradingViewChartProps {
  symbol?: string;
  theme?: "light" | "dark";
  autosize?: boolean;
  interval?: "1" | "3" | "5" | "15" | "30" | "60" | "120" | "180" | "240" | "D" | "W";
  className?: string;
  style?: React.CSSProperties;
}

const WIDGET_SCRIPT = "https://s3.tradingview.com/external-embedding/embed-widget-advanced-chart.js";

/**
 * TradingView Advanced Real-Time Chart Widget.
 * Renders only on the client to avoid SSR/window errors.
 * Supports drawing tools and indicators (MACD, RSI, etc.) built into the widget.
 */
export function TradingViewChart({
  symbol = "BINANCE:ETHUSDT",
  theme = "dark",
  autosize = true,
  interval = "60",
  className = "",
  style,
}: TradingViewChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!mounted || typeof window === "undefined" || !containerRef.current) return;

    const container = containerRef.current;

    // TradingView expects: container > __widget div + script (src + config as innerHTML)
    const config = {
      autosize,
      symbol,
      interval,
      timezone: "Etc/UTC",
      theme,
      style: "1", // Candlesticks
      locale: "en",
      allow_symbol_change: true,
      calendar: false,
      support_host: "https://www.tradingview.com",
    };

    const script = document.createElement("script");
    script.type = "text/javascript";
    script.src = WIDGET_SCRIPT;
    script.async = true;
    script.textContent = JSON.stringify(config);

    container.appendChild(script);

    return () => {
      script.remove();
      const widget = container.querySelector(".tradingview-widget-container__widget");
      if (widget) widget.innerHTML = "";
    };
  }, [mounted, symbol, theme, autosize, interval]);

  if (!mounted) {
    return (
      <div
        className={`flex items-center justify-center bg-oak-bg-elevated ${className}`}
        style={{ minHeight: 400, ...style }}
      >
        <div className="h-8 w-32 animate-pulse rounded bg-oak-border" />
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className={`tradingview-widget-container ${className}`}
      style={{ height: "100%", width: "100%", ...style }}
    >
      <div
        className="tradingview-widget-container__widget"
        style={{ height: "calc(100% - 32px)", width: "100%" }}
      />
    </div>
  );
}
