import "server-only";
import { getDb } from "./db";
import { getWatchHistory, isConnected } from "./pmvhaven";
import { upsertHistory } from "./repo";

/**
 * Pages through PMVHaven's `/user/watch-history` and permanently upserts each
 * entry into local SQLite. A "full" sync walks the entire retained window
 * (used on first connect); an incremental sync stops after a page yields no
 * new rows (history is newest-first, so older pages are already captured).
 */

const PAGE_LIMIT = 60;
const MAX_PAGES = 500; // safety ceiling (~30k entries)

let running = false;

export interface SyncResult {
  status: "ok" | "skipped" | "error";
  newCount: number;
  seenCount: number;
  pages: number;
  message?: string;
}

export function isSyncing(): boolean {
  return running;
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

export async function syncWatchHistory(full = false): Promise<SyncResult> {
  if (!isConnected()) {
    return { status: "skipped", newCount: 0, seenCount: 0, pages: 0, message: "Not connected" };
  }
  if (running) {
    return { status: "skipped", newCount: 0, seenCount: 0, pages: 0, message: "Sync already running" };
  }
  running = true;

  const db = getDb();
  const logId = (
    db
      .prepare(
        "INSERT INTO sync_log (kind, started_at, status) VALUES (?, ?, 'running')",
      )
      .run(full ? "history:full" : "history:incremental", Date.now())
      .lastInsertRowid as number
  );

  let newCount = 0;
  let seenCount = 0;
  let page = 1;

  try {
    for (; page <= MAX_PAGES; page++) {
      const { items, pagination } = await getWatchHistory(page, PAGE_LIMIT);
      if (!items.length) break;

      let pageNew = 0;
      const tx = db.transaction(() => {
        for (const entry of items) {
          if (!entry.video?.id) continue;
          const inserted = upsertHistory(entry, "sync");
          if (inserted) {
            newCount++;
            pageNew++;
          } else {
            seenCount++;
          }
        }
      });
      tx();

      // Incremental: once a whole page is already known, everything older is too.
      if (!full && pageNew === 0) break;
      if (!pagination.hasNext) break;
    }

    db.prepare(
      "UPDATE sync_log SET finished_at = ?, status = 'ok', new_count = ?, seen_count = ? WHERE id = ?",
    ).run(Date.now(), newCount, seenCount, logId);

    return { status: "ok", newCount, seenCount, pages: page };
  } catch (err) {
    const message = err instanceof Error ? err.message : "Unknown error";
    db.prepare(
      "UPDATE sync_log SET finished_at = ?, status = 'error', new_count = ?, seen_count = ?, message = ? WHERE id = ?",
    ).run(Date.now(), newCount, seenCount, message, logId);
    return { status: "error", newCount, seenCount, pages: page, message };
  } finally {
    running = false;
  }
}
