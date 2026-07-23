use crate::paths;
use once_cell::sync::OnceCell;
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, Mutex};

static DB: OnceCell<Arc<Mutex<Connection>>> = OnceCell::new();

pub fn init_db() -> anyhow::Result<Arc<Mutex<Connection>>> {
    if let Some(existing) = DB.get() {
        return Ok(existing.clone());
    }
    let path = paths::v2_db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        ",
    )?;
    migrate(&conn)?;
    let arc = Arc::new(Mutex::new(conn));
    match DB.set(arc.clone()) {
        Ok(()) => Ok(arc),
        Err(_) => Ok(DB.get().expect("db set race").clone()),
    }
}

pub fn db() -> Arc<Mutex<Connection>> {
    if let Some(existing) = DB.get() {
        return existing.clone();
    }
    init_db().expect("failed to initialize database")
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
          key   TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );

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

        CREATE TABLE IF NOT EXISTS watch_history (
          video_id      TEXT PRIMARY KEY,
          watched_at    INTEGER NOT NULL,
          progress      REAL DEFAULT 0,
          source        TEXT DEFAULT 'sync',
          first_seen_at INTEGER NOT NULL,
          FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_history_watched_at ON watch_history(watched_at DESC);

        CREATE TABLE IF NOT EXISTS favorites (
          video_id TEXT PRIMARY KEY,
          added_at INTEGER NOT NULL,
          FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS watch_later (
          video_id TEXT PRIMARY KEY,
          added_at INTEGER NOT NULL,
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
        "#,
    )?;
    Ok(())
}

pub fn get_setting(key: &str) -> Option<String> {
    let db = db();
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn set_setting(key: &str, value: &str) {
    let db = db();
    let conn = db.lock().unwrap();
    let _ = conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    );
}
