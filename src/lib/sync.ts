import "server-only";
import { getDb } from "./db";
import { fetchRemoteHistory, getVideosBulk, isConnected } from "./pmvhaven";
import { cacheVideos, getCachedSummary, upsertHistory } from "./repo";

/**
 * Imports PMVHaven watch history into permanent local SQLite.
 *
 * The history lives in the user object from `/auth/session`
 * (`watchHistory` + `watchProgress`). Free accounts only expose the most
 * recent ~500 entries, so this captures that window; because we accumulate
 * locally and never prune, re-running over time builds a permanent archive
 * that outgrows PMVHaven's rolling limit.
 */

const BULK_CHUNK = 100;

let running = false;

export interface SyncProgress {
  phase: "starting" | "fetching" | "hydrating" | "saving" | "done";
  processed: number;
  total: number;
  newCount: number;
  totalRetained: number;
  message?: string;
}

let progress: SyncProgress | null = null;

export function isSyncing(): boolean {
  return running;
}

export function syncProgress(): SyncProgress | null {
  return progress;
}

export interface SyncResult {
  status: "ok" | "skipped" | "error";
  newCount: number;
  seenCount: number;
  totalRetained: number;
  message?: string;
}

export function lastSync(): {
  finishedAt: number | null;
  newCount: number;
  status: string;
} | null {
  const row = getDb()
    .prepare(
      "SELECT finished_at, new_count, status FROM sync_log WHERE kind LIKE 'history%' ORDER BY id DESC LIMIT 1",
    )
    .get() as
    | { finished_at: number | null; new_count: number; status: string }
    | undefined;
  if (!row) return null;
  return { finishedAt: row.finished_at, newCount: row.new_count, status: row.status };
}

export async function syncWatchHistory(): Promise<SyncResult> {
  if (!isConnected()) {
    return { status: "skipped", newCount: 0, seenCount: 0, totalRetained: 0, message: "Not connected" };
  }
  if (running) {
    return {
      status: "skipped",
      newCount: 0,
      seenCount: 0,
      totalRetained: 0,
      message: "Sync already running",
    };
  }
  running = true;
  progress = { phase: "starting", processed: 0, total: 0, newCount: 0, totalRetained: 0 };

  const db = getDb();
  const logId = db
    .prepare("INSERT INTO sync_log (kind, started_at, status) VALUES (?, ?, 'running')")
    .run("history:sync", Date.now()).lastInsertRowid as number;

  let newCount = 0;
  let seenCount = 0;

  try {
    // 1. Pull the exposed history window from the session object.
    progress = { ...progress!, phase: "fetching", message: "Fetching history from PMVHaven…" };
    const remote = await fetchRemoteHistory();
    const total = remote.entries.length;
    progress = { ...progress!, total, totalRetained: remote.totalRetained };

    if (total === 0) {
      db.prepare(
        "UPDATE sync_log SET finished_at = ?, status = 'ok', new_count = 0, seen_count = 0 WHERE id = ?",
      ).run(Date.now(), logId);
      progress = { ...progress!, phase: "done" };
      return { status: "ok", newCount: 0, seenCount: 0, totalRetained: remote.totalRetained };
    }

    // 2. Hydrate any videos we don't already have cached (batched).
    progress = { ...progress!, phase: "hydrating", message: "Loading video details…" };
    const uncached = remote.entries
      .map((e) => e.videoId)
      .filter((id) => !getCachedSummary(id));

    for (let i = 0; i < uncached.length; i += BULK_CHUNK) {
      const chunk = uncached.slice(i, i + BULK_CHUNK);
      const videos = await getVideosBulk(chunk);
      cacheVideos(videos);
      progress = {
        ...progress!,
        processed: Math.min(uncached.length, i + chunk.length),
        total: Math.max(total, uncached.length),
      };
    }

    // 3. Upsert every entry into the permanent history table.
    progress = { ...progress!, phase: "saving", processed: 0, total, message: "Saving to library…" };
    let processed = 0;
    const tx = db.transaction((entries: typeof remote.entries) => {
      for (const e of entries) {
        const summary = getCachedSummary(e.videoId);
        if (!summary) {
          // Video may have been deleted from PMVHaven; skip.
          processed++;
          continue;
        }
        const inserted = upsertHistory(
          { video: summary, watchedAt: e.watchedAt, progress: e.progress },
          "sync",
        );
        if (inserted) newCount++;
        else seenCount++;
        processed++;
      }
    });
    tx(remote.entries);
    progress = { ...progress!, processed, newCount, phase: "done" };

    db.prepare(
      "UPDATE sync_log SET finished_at = ?, status = 'ok', new_count = ?, seen_count = ? WHERE id = ?",
    ).run(Date.now(), newCount, seenCount, logId);

    return { status: "ok", newCount, seenCount, totalRetained: remote.totalRetained };
  } catch (err) {
    const message = err instanceof Error ? err.message : "Unknown error";
    db.prepare(
      "UPDATE sync_log SET finished_at = ?, status = 'error', new_count = ?, seen_count = ?, message = ? WHERE id = ?",
    ).run(Date.now(), newCount, seenCount, message, logId);
    progress = { ...progress!, phase: "done", message };
    return { status: "error", newCount, seenCount, totalRetained: 0, message };
  } finally {
    running = false;
  }
}
