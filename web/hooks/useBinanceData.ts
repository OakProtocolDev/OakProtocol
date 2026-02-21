"use client";

import { useEffect, useState, useCallback, useRef } from "react";

/** Binance ticker stream: last price, 24h stats */
export interface BinanceTicker {
  c: string; // Last price
  b: string; // Best bid
  B: string; // Best bid qty
  a: string; // Best ask
  A: string; // Best ask qty
  // ... other fields
}

/** Binance partial book depth: bids/asks [price, qty][] */
export interface BinanceDepth {
  lastUpdateId: number;
  bids: [string, string][];
  asks: [string, string][];
}

export interface UseBinanceDataResult {
  /** Last price from ticker stream */
  lastPrice: string | null;
  /** Order book bids [price, qty][] - sorted highest to lowest */
  bids: [string, string][];
  /** Order book asks [price, qty][] - sorted lowest to highest */
  asks: [string, string][];
  /** True until first message received from both streams */
  isConnecting: boolean;
  /** Connection error message if any */
  error: string | null;
}

const TICKER_URL = "wss://stream.binance.com:9443/ws/ethusdt@ticker";
const DEPTH_URL = "wss://stream.binance.com:9443/ws/ethusdt@depth20@100ms";

function formatPrice(p: string): string {
  const n = parseFloat(p);
  if (n >= 1000) return n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

function formatQty(q: string): string {
  const n = parseFloat(q);
  if (n >= 1000) return n.toLocaleString("en-US", { maximumFractionDigits: 0 });
  if (n >= 1) return n.toFixed(2);
  return n.toFixed(4);
}

/** Format [price, qty] for display and compute total (price * qty) */
export function formatOrderBookRow(
  [price, qty]: [string, string]
): { price: string; amount: string; total: string } {
  const p = parseFloat(price);
  const q = parseFloat(qty);
  const total = p * q;
  return {
    price: formatPrice(price),
    amount: formatQty(qty),
    total: total.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }),
  };
}

/**
 * Live Binance WebSocket data for ETH/USDT.
 * Connects to ticker (last price) and depth20@100ms (order book).
 * Cleans up on unmount to prevent memory leaks.
 */
export function useBinanceData(): UseBinanceDataResult {
  const [lastPrice, setLastPrice] = useState<string | null>(null);
  const [bids, setBids] = useState<[string, string][]>([]);
  const [asks, setAsks] = useState<[string, string][]>([]);
  const [isConnecting, setIsConnecting] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const tickerReceived = useRef(false);
  const depthReceived = useRef(false);
  const mounted = useRef(true);

  const updateConnecting = useCallback(() => {
    if (tickerReceived.current && depthReceived.current && mounted.current) {
      setIsConnecting(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    setError(null);
    tickerReceived.current = false;
    depthReceived.current = false;
    setIsConnecting(true);

    // --- Ticker WebSocket ---
    const tickerWs = new WebSocket(TICKER_URL);

    tickerWs.onmessage = (event) => {
      if (!mounted.current) return;
      try {
        const data = JSON.parse(event.data) as BinanceTicker;
        if (data.c != null) {
          setLastPrice(data.c);
          if (!tickerReceived.current) {
            tickerReceived.current = true;
            updateConnecting();
          }
        }
      } catch {
        // ignore parse errors
      }
    };

    tickerWs.onerror = () => {
      if (mounted.current) setError("Ticker connection error");
    };

    // --- Depth WebSocket ---
    const depthWs = new WebSocket(DEPTH_URL);

    depthWs.onmessage = (event) => {
      if (!mounted.current) return;
      try {
        const data = JSON.parse(event.data) as BinanceDepth;
        if (Array.isArray(data.bids) && Array.isArray(data.asks)) {
          setBids(data.bids);
          setAsks(data.asks);
          if (!depthReceived.current) {
            depthReceived.current = true;
            updateConnecting();
          }
        }
      } catch {
        // ignore parse errors
      }
    };

    depthWs.onerror = () => {
      if (mounted.current) setError("Depth connection error");
    };

    // Cleanup on unmount - prevent memory leaks and state updates after unmount
    return () => {
      mounted.current = false;
      if (tickerWs.readyState === WebSocket.OPEN || tickerWs.readyState === WebSocket.CONNECTING) {
        tickerWs.close();
      }
      if (depthWs.readyState === WebSocket.OPEN || depthWs.readyState === WebSocket.CONNECTING) {
        depthWs.close();
      }
    };
  }, [updateConnecting]);

  return {
    lastPrice,
    bids,
    asks,
    isConnecting,
    error,
  };
}
