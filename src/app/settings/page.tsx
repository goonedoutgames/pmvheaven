"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import Link from "next/link";
import { Database, LogOut, MonitorPlay, RefreshCw } from "lucide-react";
import { useSession } from "@/components/SessionProvider";
import { usePlayer } from "@/components/PlayerProvider";
import { GridSkeleton } from "@/components/VideoGrid";

export default function SettingsPage() {
  const router = useRouter();
  const { authenticated, loading, user, historyCount, lastSync, syncing, refresh } =
    useSession();
  const { separateWindow, setSeparateWindow } = usePlayer();
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  if (loading) return <GridSkeleton count={4} />;

  const playbackSection = (
    <section className="flex flex-col gap-4 rounded-2xl border border-border bg-surface p-5">
      <div className="flex items-center gap-2 font-semibold">
        <MonitorPlay size={18} /> Playback
      </div>
      <label className="flex cursor-pointer items-start justify-between gap-4">
        <div>
          <p className="text-sm font-medium">Play videos in a separate window</p>
          <p className="text-xs text-muted">
            Streams open in their own window so browsing, queuing and navigation
            stay completely independent of what&apos;s playing. When off, the
            player docks into a side rail and the app makes room for it.
          </p>
        </div>
        <button
          role="switch"
          aria-checked={separateWindow}
          onClick={() => setSeparateWindow(!separateWindow)}
          className={`relative mt-1 h-6 w-11 shrink-0 rounded-full transition ${
            separateWindow ? "bg-accent" : "bg-surface-2"
          }`}
        >
          <span
            className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all ${
              separateWindow ? "left-[22px]" : "left-0.5"
            }`}
          />
        </button>
      </label>
    </section>
  );

  if (!authenticated) {
    return (
      <div className="mx-auto flex max-w-2xl flex-col gap-6 py-4">
        <h1 className="text-2xl font-bold tracking-tight">Settings</h1>
        {playbackSection}
        <p className="text-muted">
          <Link href="/login" className="text-accent underline">
            Sign in
          </Link>{" "}
          to manage your account and sync history.
        </p>
      </div>
    );
  }

  const resync = async () => {
    setBusy(true);
    setMsg("Syncing history…");
    try {
      const res = await fetch("/api/history/sync", { method: "POST" });
      const data = await res.json();
      setMsg(
        data.status === "ok"
          ? `Imported ${data.newCount} new (${data.seenCount} already saved). PMVHaven retains ${data.totalRetained?.toLocaleString?.() ?? data.totalRetained} total.`
          : (data.message ?? "Sync failed"),
      );
    } catch {
      setMsg("Sync failed");
    } finally {
      setBusy(false);
      void refresh();
    }
  };

  const logout = async () => {
    setBusy(true);
    await fetch("/api/auth/logout", { method: "POST" });
    await refresh();
    router.push("/");
  };

  const lastSyncLabel = lastSync?.finishedAt
    ? new Date(lastSync.finishedAt).toLocaleString()
    : "never";

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-6 py-4">
      <h1 className="text-2xl font-bold tracking-tight">Settings</h1>

      <section className="flex items-center gap-4 rounded-2xl border border-border bg-surface p-5">
        {user?.avatarUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={user.avatarUrl}
            alt={user.username}
            className="h-14 w-14 rounded-full object-cover ring-1 ring-border"
          />
        ) : (
          <span className="grid h-14 w-14 place-items-center rounded-full bg-surface-2 text-xl font-bold">
            {user?.username?.[0]?.toUpperCase() ?? "?"}
          </span>
        )}
        <div>
          <p className="text-lg font-semibold">{user?.username}</p>
          {user?.email && <p className="text-sm text-muted">{user.email}</p>}
          <p className="mt-1 text-xs text-emerald-400">Connected to PMVHaven</p>
        </div>
      </section>

      <section className="flex flex-col gap-4 rounded-2xl border border-border bg-surface p-5">
        <div className="flex items-center gap-2 font-semibold">
          <Database size={18} /> Permanent library
        </div>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <p className="text-2xl font-bold">{historyCount.toLocaleString()}</p>
            <p className="text-muted">Videos in local history</p>
          </div>
          <div>
            <p className="text-sm font-medium">{lastSyncLabel}</p>
            <p className="text-muted">Last sync ({lastSync?.status ?? "—"})</p>
          </div>
        </div>
        <p className="text-xs text-muted">
          Your watch history is stored permanently in a local SQLite database and
          is never subject to PMVHaven&apos;s rolling retention limit. Run a full
          resync periodically to capture everything currently retained on their
          side.
        </p>
        <button
          onClick={resync}
          disabled={busy || syncing}
          className="flex w-fit items-center gap-2 rounded-lg border border-border bg-background px-4 py-2 text-sm font-medium transition hover:bg-surface-2 disabled:opacity-60"
        >
          <RefreshCw size={15} className={busy || syncing ? "animate-spin" : ""} />
          Sync history now
        </button>
        {msg && <p className="text-sm text-muted">{msg}</p>}
      </section>

      {playbackSection}

      <button
        onClick={logout}
        disabled={busy}
        className="flex w-fit items-center gap-2 rounded-lg border border-rose-500/40 bg-rose-500/10 px-4 py-2 text-sm font-medium text-rose-400 transition hover:bg-rose-500/20 disabled:opacity-60"
      >
        <LogOut size={15} /> Sign out
      </button>
    </div>
  );
}
