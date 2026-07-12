"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { Loader2, RefreshCw } from "lucide-react";
import { useSession } from "@/components/SessionProvider";
import { HistoryGrid, GridSkeleton } from "@/components/VideoGrid";
import type { HistoryEntry, Paged } from "@/lib/types";

export default function HistoryPage() {
  const { authenticated, loading: sessionLoading, historyCount, syncing, refresh } =
    useSession();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [page, setPage] = useState(1);
  const [hasNext, setHasNext] = useState(true);
  const [loading, setLoading] = useState(false);
  const [initialized, setInitialized] = useState(false);
  const [total, setTotal] = useState(0);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const sentinel = useRef<HTMLDivElement>(null);

  const load = useCallback(async (p: number, replace = false) => {
    setLoading(true);
    try {
      const res = await fetch(`/api/history?page=${p}&limit=60`, { cache: "no-store" });
      if (!res.ok) throw new Error();
      const data = (await res.json()) as Paged<HistoryEntry>;
      setEntries((prev) => (replace ? data.items : [...prev, ...data.items]));
      setHasNext(data.pagination?.hasNext ?? false);
      setTotal(data.pagination?.total ?? 0);
    } catch {
      setHasNext(false);
    } finally {
      setLoading(false);
      setInitialized(true);
    }
  }, []);

  useEffect(() => {
    if (authenticated && !initialized) void load(1, true);
  }, [authenticated, initialized, load]);

  useEffect(() => {
    const el = sentinel.current;
    if (!el) return;
    const io = new IntersectionObserver(
      (e) => {
        if (e[0].isIntersecting && hasNext && !loading && initialized) {
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

  const runSync = async (full: boolean) => {
    setSyncMsg("Syncing…");
    try {
      const res = await fetch(`/api/history/sync${full ? "?full=1" : ""}`, {
        method: "POST",
      });
      const data = await res.json();
      if (data.status === "ok") {
        setSyncMsg(`Added ${data.newCount} new · ${data.seenCount} already saved`);
        setPage(1);
        setInitialized(false);
        await load(1, true);
      } else {
        setSyncMsg(data.message ?? "Sync failed");
      }
    } catch {
      setSyncMsg("Sync failed");
    } finally {
      void refresh();
    }
  };

  if (sessionLoading) return <GridSkeleton count={10} />;

  if (!authenticated) {
    return (
      <div className="py-24 text-center">
        <h1 className="text-2xl font-bold">Watch history</h1>
        <p className="mt-3 text-muted">
          <Link href="/login" className="text-accent underline">
            Sign in
          </Link>{" "}
          to sync and permanently keep your PMVHaven watch history.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Watch history</h1>
          <p className="mt-1 text-sm text-muted">
            {total.toLocaleString()} videos kept permanently
            {historyCount !== total && ` · ${historyCount.toLocaleString()} in library`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => runSync(false)}
            disabled={syncing || loading}
            className="flex items-center gap-2 rounded-lg border border-border bg-surface px-3.5 py-2 text-sm font-medium transition hover:bg-surface-2 disabled:opacity-60"
          >
            <RefreshCw size={15} className={syncing ? "animate-spin" : ""} />
            Sync new
          </button>
          <button
            onClick={() => runSync(true)}
            disabled={syncing || loading}
            className="rounded-lg border border-border bg-surface px-3.5 py-2 text-sm font-medium transition hover:bg-surface-2 disabled:opacity-60"
          >
            Full resync
          </button>
        </div>
      </div>

      {syncMsg && <p className="text-sm text-muted">{syncMsg}</p>}

      {!initialized ? (
        <GridSkeleton count={10} />
      ) : entries.length === 0 ? (
        <div className="py-16 text-center text-muted">
          <p>No history yet.</p>
          <p className="mt-2 text-sm">
            Click <strong>Full resync</strong> to import your retained PMVHaven
            history, then it&apos;s kept here forever.
          </p>
        </div>
      ) : (
        <>
          <HistoryGrid entries={entries} />
          <div ref={sentinel} className="h-10" />
          {loading && (
            <div className="flex justify-center py-4 text-muted">
              <Loader2 className="animate-spin" />
            </div>
          )}
        </>
      )}
    </div>
  );
}
