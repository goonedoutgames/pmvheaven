import Database from "better-sqlite3";
import fs from "node:fs";
import path from "node:path";

/**
 * Single shared better-sqlite3 connection. The database file lives in ./data
 * (gitignored) so the permanent watch history survives restarts and is never
 * subject to PMVHaven's rolling retention window.
 */

const DATA_DIR = process.env.PH_DATA_DIR
  ? path.resolve(process.env.PH_DATA_DIR)
  : path.join(process.cwd(), "data");

const DB_PATH = path.join(DATA_DIR, "pmvheaven.db");

let db: Database.Database | null = null;

export function getDb(): Database.Database {
  if (db) return db;

  fs.mkdirSync(DATA_DIR, { recursive: true });

  db = new Database(DB_PATH);
  db.pragma("journal_mode = WAL");
  db.pragma("foreign_keys = ON");
  migrate(db);
  return db;
}

function migrate(d: Database.Database) {
  d.exec(`
    CREATE TABLE IF NOT EXISTS settings (
      key   TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );

    -- The single connected PMVHaven account (single-user app). Stores the
    -- encrypted credentials (for silent re-auth) and the captured session
    -- cookies used to call authenticated endpoints.
    CREATE TABLE IF NOT EXISTS account (
      id             INTEGER PRIMARY KEY CHECK (id = 1),
      pmv_user_id    TEXT,
      username       TEXT,
      email          TEXT,
      avatar_url     TEXT,
      enc_email      TEXT,
      enc_password   TEXT,
      cookies        TEXT,
      last_login_at  INTEGER,
      created_at     INTEGER NOT NULL
    );

    -- App-side sessions (the httpOnly cookie our browser holds).
    CREATE TABLE IF NOT EXISTS app_sessions (
      token       TEXT PRIMARY KEY,
      created_at  INTEGER NOT NULL,
      expires_at  INTEGER NOT NULL
    );

    -- Cached video metadata so history/favorites render fast and offline.
    CREATE TABLE IF NOT EXISTS videos (
      id                TEXT PRIMARY KEY,
      title             TEXT,
      uploader          TEXT,
      uploader_username TEXT,
      thumbnail_url     TEXT,
      preview_url       TEXT,
      views             INTEGER,
      duration          TEXT,
      duration_seconds  INTEGER,
      aspect_ratio      REAL,
      likes             INTEGER,
      dislikes          INTEGER,
      rating            REAL,
      tags              TEXT,
      upload_date       TEXT,
      json              TEXT,
      updated_at        INTEGER NOT NULL
    );

    -- The permanent, append-only watch history. Keyed by video so re-watches
    -- update watched_at/progress rather than duplicating.
    CREATE TABLE IF NOT EXISTS watch_history (
      video_id     TEXT PRIMARY KEY,
      watched_at   INTEGER NOT NULL,
      progress     REAL DEFAULT 0,
      source       TEXT DEFAULT 'sync',
      first_seen_at INTEGER NOT NULL,
      FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_history_watched_at ON watch_history(watched_at DESC);

    CREATE TABLE IF NOT EXISTS favorites (
      video_id   TEXT PRIMARY KEY,
      added_at   INTEGER NOT NULL,
      FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS watch_later (
      video_id   TEXT PRIMARY KEY,
      added_at   INTEGER NOT NULL,
      FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS sync_log (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      kind        TEXT NOT NULL,
      started_at  INTEGER NOT NULL,
      finished_at INTEGER,
      status      TEXT NOT NULL DEFAULT 'running',
      new_count   INTEGER DEFAULT 0,
      seen_count  INTEGER DEFAULT 0,
      message     TEXT
    );
  `);
}

export function getSetting(key: string): string | null {
  const row = getDb()
    .prepare("SELECT value FROM settings WHERE key = ?")
    .get(key) as { value: string } | undefined;
  return row?.value ?? null;
}

export function setSetting(key: string, value: string): void {
  getDb()
    .prepare(
      "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .run(key, value);
}
