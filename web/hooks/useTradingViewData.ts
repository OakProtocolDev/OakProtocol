"use client";

import { useCallback, useEffect, useState } from "react";

/** Single OHLC candle (TradingView / lightweight-charts friendly). */
export interface OHLCBar {
  time: number; // Unix timestamp (seconds)
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

export type OHLCInterval = "1m" | "3m" | "5m" | "15m" | "30m" | "1h" | "2h" | "4h" | "1d";

const BINANCE_KLINES = "https://api.binance.com/api/v3/klines";

/** Map symbol like "ETH/USDC" or "ETHUSDT" to Binance symbol. */
function toBinanceSymbol(symbol: string): string {
  const normalized = symbol.replace("/", "").replace("-", "").toUpperCase();
  if (normalized.endsWith("USDT") || normalized.endsWith("BUSD") || normalized.endsWith("USDC")) {
    return normalized;
  }
  if (normalized === "ETH") return "ETHUSDT";
  if (normalized === "BTC") return "BTCUSDT";
  return `${normalized}USDT`;
}

/**
 * Fetches OHLC data for a given symbol (Binance Spot).
 * Use for TradingView-style charts, mini charts in PositionTable, or price alert checks.
 */
export function useTradingViewData(
  symbol: string,
  interval: OHLCInterval = "1h",
  limit: number = 100
) {
  const [ohlc, setOhlc] = useState<OHLCBar[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchKlines = useCallback(async () => {
    if (!symbol?.trim()) {
      setOhlc([]);
      setIsLoading(false);
      return;
    }
    setIsLoading(true);
    setError(null);
    const binanceSymbol = toBinanceSymbol(symbol);
    const url = `${BINANCE_KLINES}?symbol=${encodeURIComponent(binanceSymbol)}&interval=${interval}&limit=${Math.min(Math.max(limit, 1), 1000)}`;
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const raw = (await res.json()) as [number, string, string, string, string, string, number, ...unknown[]][];
      const bars: OHLCBar[] = raw.map(([openTime, o, h, l, c, vol]) => ({
        time: Math.floor(openTime / 1000),
        open: parseFloat(o),
        high: parseFloat(h),
        low: parseFloat(l),
        close: parseFloat(c),
        volume: parseFloat(vol),
      }));
      setOhlc(bars);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to fetch OHLC");
      setOhlc([]);
    } finally {
      setIsLoading(false);
    }
  }, [symbol, interval, limit]);

  useEffect(() => {
    fetchKlines();
  }, [fetchKlines]);

  return {
    ohlc,
    isLoading,
    error,
    refetch: fetchKlines,
    /** Binance symbol used for the request (e.g. for TradingView widget). */
    binanceSymbol: symbol?.trim() ? toBinanceSymbol(symbol) : "ETHUSDT",
  };
}
