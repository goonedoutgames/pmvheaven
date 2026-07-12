"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { Loader2, RefreshCw } from "lucide-react";
import { useSession } from "@/components/SessionProvider";
import { HistoryGrid, GridSkeleton } from "@/components/VideoGrid";
import type { HistoryEntry, Paged } from "@/lib/types";

interface SyncProgress {
  phase: string;
  processed: number;
  total: number;
  newCount: number;
  totalRetained: number;
  message?: string;
}

const PHASE_LABEL: Record<string, string> = {
  starting: "Starting…",
  fetching: "Fetching history from PMVHaven…",
  hydrating: "Loading video details…",
  saving: "Saving to your permanent library…",
  done: "Finishing up…",
};

export default function HistoryPage() {
  const { authenticated, loading: sessionLoading, historyCount, refresh } = useSession();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [page, setPage] = useState(1);
  const [hasNext, setHasNext] = useState(true);
  const [loading, setLoading] = useState(false);
  const [initialized, setInitialized] = useState(false);
  const [total, setTotal] = useState(0);
  const [retained, setRetained] = useState(0);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [prog, setProg] = useState<SyncProgress | null>(null);
  const [busy, setBusy] = useState(false);
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

  const runSync = async () => {
    if (busy) return;
    setBusy(true);
    setSyncMsg(null);
    setProg({ phase: "starting", processed: 0, total: 0, newCount: 0, totalRetained: 0 });

    let stop = false;
    const poll = async () => {
      while (!stop) {
        try {
          const s = await fetch("/api/history/sync", { cache: "no-store" }).then((r) =>
            r.json(),
          );
          if (s.progress) setProg(s.progress);
        } catch {
          /* ignore */
        }
        await new Promise((r) => setTimeout(r, 500));
      }
    };
    void poll();

    try {
      const res = await fetch("/api/history/sync", { method: "POST" });
      const data = await res.json();
      if (data.status === "ok") {
        setRetained(data.totalRetained ?? 0);
        setSyncMsg(
          `Imported ${data.newCount} new (${data.seenCount} already saved). PMVHaven retains ${data.totalRetained?.toLocaleString?.() ?? data.totalRetained} total.`,
        );
      } else {
        setSyncMsg(data.message ?? "Sync failed");
      }
    } catch {
      setSyncMsg("Sync failed");
    } finally {
      stop = true;
      setProg(null);
      setPage(1);
      await load(1, true);
      void refresh();
      setBusy(false);
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

  const pct =
    prog && prog.total > 0 ? Math.round((prog.processed / prog.total) * 100) : null;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Watch history</h1>
          <p className="mt-1 text-sm text-muted">
            {(total || historyCount).toLocaleString()} kept permanently
            {retained > total &&
              ` · PMVHaven retains ${retained.toLocaleString()} (only the latest ~500 are importable)`}
          </p>
        </div>
        <button
          onClick={runSync}
          disabled={busy}
          className="flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
        >
          <RefreshCw size={15} className={busy ? "animate-spin" : ""} />
          {busy ? "Syncing…" : "Sync history"}
        </button>
      </div>

      {prog && (
        <div className="flex flex-col gap-2 rounded-xl border border-border bg-surface p-4">
          <div className="flex items-center justify-between text-sm">
            <span className="flex items-center gap-2">
              <Loader2 size={15} className="animate-spin text-accent" />
              {PHASE_LABEL[prog.phase] ?? prog.message ?? "Working…"}
            </span>
            <span className="tabular-nums text-muted">
              {prog.total > 0
                ? `${prog.processed}/${prog.total}${pct !== null ? ` · ${pct}%` : ""}`
                : ""}
            </span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-surface-2">
            <div
              className="h-full rounded-full bg-gradient-to-r from-accent to-accent-2 transition-all"
              style={{ width: `${pct ?? 8}%` }}
            />
          </div>
          {prog.newCount > 0 && (
            <p className="text-xs text-muted">{prog.newCount} new saved so far</p>
          )}
        </div>
      )}

      {syncMsg && !prog && <p className="text-sm text-muted">{syncMsg}</p>}

      {!initialized ? (
        <GridSkeleton count={10} />
      ) : entries.length === 0 ? (
        <div className="py-16 text-center text-muted">
          <p>No history yet.</p>
          <p className="mt-2 text-sm">
            Click <strong>Sync history</strong> to import your retained PMVHaven
            history — it&apos;s then kept here permanently.
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
