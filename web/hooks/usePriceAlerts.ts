"use client";

import { useCallback, useEffect, useRef, useState } from "react";

/** Single price alert condition (stored off-chain; see PRICE_ALERTS_ARCHITECTURE.md). */
export interface PriceAlert {
  id: string;
  symbol: string;
  condition: "above" | "below";
  targetPrice: number;
  createdAt: number;
  /** Set when condition was met (client-side demo). */
  triggeredAt?: number;
}

const STORAGE_KEY = "oak_price_alerts";

function loadAlerts(): PriceAlert[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as PriceAlert[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveAlerts(alerts: PriceAlert[]) {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(alerts));
  } catch {
    // ignore
  }
}

/**
 * Client-side only: store and list price alerts (localStorage).
 * For production, replace with API calls to your backend; worker checks price and sends notifications (no tx spam).
 * See web/docs/PRICE_ALERTS_ARCHITECTURE.md.
 */
export function usePriceAlerts() {
  const [alerts, setAlerts] = useState<PriceAlert[]>([]);

  useEffect(() => {
    setAlerts(loadAlerts());
  }, []);

  const addAlert = useCallback((symbol: string, condition: "above" | "below", targetPrice: number) => {
    const next: PriceAlert = {
      id: crypto.randomUUID(),
      symbol,
      condition,
      targetPrice,
      createdAt: Date.now(),
    };
    setAlerts((prev) => {
      const list = [...prev, next];
      saveAlerts(list);
      return list;
    });
  }, []);

  const removeAlert = useCallback((id: string) => {
    setAlerts((prev) => {
      const list = prev.filter((a) => a.id !== id);
      saveAlerts(list);
      return list;
    });
  }, []);

  const markTriggered = useCallback((id: string) => {
    setAlerts((prev) => {
      const list = prev.map((a) => (a.id === id ? { ...a, triggeredAt: Date.now() } : a));
      saveAlerts(list);
      return list;
    });
  }, []);

  return {
    alerts,
    addAlert,
    removeAlert,
    markTriggered,
  };
}

/**
 * Check if any alert condition is met by current price (for in-app demo).
 * Call from a component that has current price (e.g. from useBinanceData or useTradingViewData).
 */
export function usePriceAlertCheck(
  currentPrice: number | null,
  alerts: PriceAlert[],
  onTriggered: (alert: PriceAlert) => void
) {
  const triggeredRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (currentPrice == null) return;
    alerts.forEach((alert) => {
      if (alert.triggeredAt) return;
      const met =
        alert.condition === "above"
          ? currentPrice >= alert.targetPrice
          : currentPrice <= alert.targetPrice;
      if (met && !triggeredRef.current.has(alert.id)) {
        triggeredRef.current.add(alert.id);
        onTriggered(alert);
      }
    });
  }, [currentPrice, alerts, onTriggered]);
}
