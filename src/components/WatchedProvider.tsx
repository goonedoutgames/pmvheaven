"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

/**
 * Global, app-wide source of "which videos has the user watched" (from the
 * permanent local history). Loaded once and consulted directly by VideoCard so
 * every surface — grids, rails, related, search, infinite scroll — gets the
 * "Watched" badge without threading props. Updates live when a video is watched.
 */
interface WatchedState {
  isWatched: (id: string) => boolean;
  /** Watch progress 0..1 for a video, or undefined if never watched. */
  progress: (id: string) => number | undefined;
  markWatched: (id: string, progress?: number) => void;
}

const WatchedContext = createContext<WatchedState | null>(null);

export function WatchedProvider({ children }: { children: React.ReactNode }) {
  const [map, setMap] = useState<Record<string, number>>({});

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/history/watched", { cache: "no-store" });
        if (!res.ok) return;
        const data = (await res.json()) as { watched?: Record<string, number> };
        if (!cancelled && data.watched) setMap(data.watched);
      } catch {
        /* ignore — badges just won't show */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const isWatched = useCallback((id: string) => id in map, [map]);
  const progress = useCallback((id: string) => map[id], [map]);
  const markWatched = useCallback((id: string, p = 0) => {
    setMap((prev) => {
      const existing = prev[id];
      const next = existing === undefined ? p : Math.max(existing, p);
      if (existing !== undefined && next === existing) return prev;
      return { ...prev, [id]: next };
    });
  }, []);

  const value = useMemo<WatchedState>(
    () => ({ isWatched, progress, markWatched }),
    [isWatched, progress, markWatched],
  );

  return <WatchedContext.Provider value={value}>{children}</WatchedContext.Provider>;
}

export function useWatched(): WatchedState {
  const ctx = useContext(WatchedContext);
  if (!ctx) {
    // Safe no-op fallback if used outside the provider.
    return {
      isWatched: () => false,
      progress: () => undefined,
      markWatched: () => {},
    };
  }
  return ctx;
}
