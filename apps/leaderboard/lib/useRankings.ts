"use client";

import { useState, useEffect } from "react";
import type { RankingsData } from "./rankings";

export function useRankings() {
  const [data, setData] = useState<RankingsData | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    fetch("/api/rankings")
      .then((res) => (res.ok ? res.json() : Promise.reject(new Error("Failed to fetch"))))
      .then((d: RankingsData) => {
        if (!cancelled) setData(d);
      })
      .catch(() => {
        if (!cancelled) setData({ traders: [], farmers: [], updatedAt: Date.now() });
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => { cancelled = true; };
  }, []);

  return {
    traders: data?.traders ?? [],
    farmers: data?.farmers ?? [],
    updatedAt: data?.updatedAt ?? 0,
    isLoading,
  };
}
