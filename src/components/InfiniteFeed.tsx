"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import type { Paged, VideoSummary } from "@/lib/types";
import { VideoGrid, GridSkeleton } from "./VideoGrid";

/**
 * Generic infinite-scroll grid. Given a base endpoint (e.g. `/api/feed`) and
 * query params, it loads pages as the sentinel scrolls into view.
 */
export function InfiniteFeed({
  endpoint,
  params,
  emptyLabel = "Nothing here yet.",
}: {
  endpoint: string;
  params: Record<string, string>;
  emptyLabel?: string;
}) {
  const [items, setItems] = useState<VideoSummary[]>([]);
  const [page, setPage] = useState(1);
  const [hasNext, setHasNext] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);
  const sentinel = useRef<HTMLDivElement>(null);
  const seen = useRef(new Set<string>());

  // Reset when the endpoint/params change (new search or filter).
  const paramsKey = JSON.stringify({ endpoint, params });
  useEffect(() => {
    setItems([]);
    setPage(1);
    setHasNext(true);
    setError(null);
    setInitialized(false);
    seen.current = new Set();
  }, [paramsKey]);

  const load = useCallback(
    async (p: number) => {
      setLoading(true);
      setError(null);
      try {
        const qs = new URLSearchParams({ ...params, page: String(p), limit: "32" });
        const res = await fetch(`${endpoint}?${qs}`, { cache: "no-store" });
        const data = (await res.json()) as Paged<VideoSummary> & { error?: string };
        if (data.error) throw new Error(data.error);
        const fresh = (data.items ?? []).filter((v) => {
          if (seen.current.has(v.id)) return false;
          seen.current.add(v.id);
          return true;
        });
        setItems((prev) => [...prev, ...fresh]);
        setHasNext(data.pagination?.hasNext ?? false);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to load");
        setHasNext(false);
      } finally {
        setLoading(false);
        setInitialized(true);
      }
    },
    [endpoint, params],
  );

  useEffect(() => {
    if (!initialized && page === 1) void load(1);
  }, [initialized, page, load]);

  useEffect(() => {
    const el = sentinel.current;
    if (!el) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasNext && !loading && initialized) {
          const next = page + 1;
          setPage(next);
          void load(next);
        }
      },
      { rootMargin: "600px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [hasNext, loading, page, load, initialized]);

  if (!initialized && loading) return <GridSkeleton count={15} />;

  return (
    <div className="flex flex-col gap-6">
      {items.length > 0 && <VideoGrid videos={items} />}
      {initialized && items.length === 0 && !loading && (
        <p className="py-16 text-center text-muted">{error ?? emptyLabel}</p>
      )}
      {error && items.length > 0 && (
        <p className="text-center text-sm text-rose-400">{error}</p>
      )}
      <div ref={sentinel} className="h-10" />
      {loading && initialized && (
        <div className="flex justify-center py-4 text-muted">
          <Loader2 className="animate-spin" />
        </div>
      )}
    </div>
  );
}
