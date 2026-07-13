import "server-only";
import { getDb } from "./db";
import type { HistoryEntry, VideoSummary } from "./types";

/**
 * Local persistence for cached video metadata plus the permanent
 * history / favorites / watch-later tables. Everything here is our own
 * SQLite copy that outlives PMVHaven's rolling retention window.
 */

interface VideoRow {
  id: string;
  title: string;
  uploader: string;
  uploader_username: string;
  thumbnail_url: string;
  preview_url: string | null;
  views: number;
  duration: string;
  duration_seconds: number;
  aspect_ratio: number;
  likes: number;
  dislikes: number;
  rating: number;
  tags: string;
  upload_date: string;
}

function rowToSummary(r: VideoRow): VideoSummary {
  return {
    id: r.id,
    title: r.title,
    uploader: r.uploader,
    uploaderUsername: r.uploader_username,
    thumbnailUrl: r.thumbnail_url,
    previewUrl: r.preview_url ?? undefined,
    views: r.views,
    duration: r.duration,
    durationSeconds: r.duration_seconds,
    aspectRatio: r.aspect_ratio,
    likes: r.likes,
    dislikes: r.dislikes,
    rating: r.rating,
    tags: r.tags ? JSON.parse(r.tags) : [],
    uploadDate: r.upload_date,
  };
}

export function cacheVideo(v: VideoSummary): void {
  getDb()
    .prepare(
      `INSERT INTO videos (id, title, uploader, uploader_username, thumbnail_url,
         preview_url, views, duration, duration_seconds, aspect_ratio, likes,
         dislikes, rating, tags, upload_date, updated_at)
       VALUES (@id, @title, @uploader, @uploader_username, @thumbnail_url,
         @preview_url, @views, @duration, @duration_seconds, @aspect_ratio, @likes,
         @dislikes, @rating, @tags, @upload_date, @updated_at)
       ON CONFLICT(id) DO UPDATE SET
         title=excluded.title, uploader=excluded.uploader,
         uploader_username=excluded.uploader_username,
         thumbnail_url=excluded.thumbnail_url, preview_url=excluded.preview_url,
         views=excluded.views, duration=excluded.duration,
         duration_seconds=excluded.duration_seconds, aspect_ratio=excluded.aspect_ratio,
         likes=excluded.likes, dislikes=excluded.dislikes, rating=excluded.rating,
         tags=excluded.tags, upload_date=excluded.upload_date,
         updated_at=excluded.updated_at`,
    )
    .run({
      id: v.id,
      title: v.title,
      uploader: v.uploader,
      uploader_username: v.uploaderUsername,
      thumbnail_url: v.thumbnailUrl,
      preview_url: v.previewUrl ?? null,
      views: v.views,
      duration: v.duration,
      duration_seconds: v.durationSeconds,
      aspect_ratio: v.aspectRatio,
      likes: v.likes,
      dislikes: v.dislikes,
      rating: v.rating,
      tags: JSON.stringify(v.tags ?? []),
      upload_date: v.uploadDate,
      updated_at: Date.now(),
    });
}

export function cacheVideos(vs: VideoSummary[]): void {
  const tx = getDb().transaction((items: VideoSummary[]) => {
    for (const v of items) if (v.id) cacheVideo(v);
  });
  tx(vs);
}

export function getCachedSummary(id: string): VideoSummary | null {
  const row = getDb().prepare("SELECT * FROM videos WHERE id = ?").get(id) as
    | VideoRow
    | undefined;
  return row ? rowToSummary(row) : null;
}

/* ------------------------------- history --------------------------------- */

/** Upsert a watched entry. Returns true if it was newly inserted. */
export function upsertHistory(
  entry: HistoryEntry,
  source: "sync" | "local" = "sync",
): boolean {
  const db = getDb();
  cacheVideo(entry.video);
  const watchedAt = Date.parse(entry.watchedAt) || Date.now();
  const existing = db
    .prepare("SELECT video_id FROM watch_history WHERE video_id = ?")
    .get(entry.video.id);
  db.prepare(
    `INSERT INTO watch_history (video_id, watched_at, progress, source, first_seen_at)
     VALUES (?, ?, ?, ?, ?)
     ON CONFLICT(video_id) DO UPDATE SET
       watched_at = MAX(watch_history.watched_at, excluded.watched_at),
       progress = MAX(watch_history.progress, excluded.progress)`,
  ).run(entry.video.id, watchedAt, entry.progress, source, Date.now());
  return !existing;
}

export function getHistoryPage(
  page = 1,
  limit = 60,
): { items: HistoryEntry[]; total: number } {
  const db = getDb();
  const total = (
    db.prepare("SELECT COUNT(*) AS c FROM watch_history").get() as { c: number }
  ).c;
  const offset = (page - 1) * limit;
  const rows = db
    .prepare(
      `SELECT h.watched_at AS watched_at, h.progress AS progress, v.*
       FROM watch_history h JOIN videos v ON v.id = h.video_id
       ORDER BY h.watched_at DESC LIMIT ? OFFSET ?`,
    )
    .all(limit, offset) as Array<VideoRow & { watched_at: number; progress: number }>;
  return {
    total,
    items: rows.map((r) => ({
      video: rowToSummary(r),
      watchedAt: new Date(r.watched_at).toISOString(),
      progress: r.progress,
    })),
  };
}

export function historyCount(): number {
  return (
    getDb().prepare("SELECT COUNT(*) AS c FROM watch_history").get() as { c: number }
  ).c;
}

/** Map of every watched video id -> max progress (0..1). Powers "Watched" badges. */
export function watchedProgressMap(): Record<string, number> {
  const rows = getDb()
    .prepare("SELECT video_id, progress FROM watch_history")
    .all() as Array<{ video_id: string; progress: number }>;
  const map: Record<string, number> = {};
  for (const r of rows) map[r.video_id] = r.progress ?? 0;
  return map;
}

/* --------------------------- favorites / later --------------------------- */

type Bucket = "favorites" | "watch_later";

export function setLocalBucket(bucket: Bucket, video: VideoSummary, on: boolean): void {
  const db = getDb();
  if (on) {
    cacheVideo(video);
    db.prepare(
      `INSERT INTO ${bucket} (video_id, added_at) VALUES (?, ?)
       ON CONFLICT(video_id) DO NOTHING`,
    ).run(video.id, Date.now());
  } else {
    db.prepare(`DELETE FROM ${bucket} WHERE video_id = ?`).run(video.id);
  }
}

export function getBucket(bucket: Bucket): VideoSummary[] {
  const rows = getDb()
    .prepare(
      `SELECT v.* FROM ${bucket} b JOIN videos v ON v.id = b.video_id
       ORDER BY b.added_at DESC`,
    )
    .all() as VideoRow[];
  return rows.map(rowToSummary);
}

export function bucketIds(bucket: Bucket): Set<string> {
  const rows = getDb().prepare(`SELECT video_id FROM ${bucket}`).all() as Array<{
    video_id: string;
  }>;
  return new Set(rows.map((r) => r.video_id));
}
