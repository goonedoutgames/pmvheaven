use crate::models::{HistoryEntry, VideoSummary};
use crate::services::db::db;
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<VideoSummary> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(VideoSummary {
        id: row.get("id")?,
        title: row.get("title")?,
        uploader: row.get("uploader")?,
        uploader_username: row.get("uploader_username")?,
        thumbnail_url: row.get("thumbnail_url")?,
        preview_url: row.get::<_, Option<String>>("preview_url")?,
        views: row.get::<_, i64>("views")? as u64,
        duration: row.get("duration")?,
        duration_seconds: row.get::<_, i64>("duration_seconds")? as u32,
        aspect_ratio: row.get("aspect_ratio")?,
        likes: row.get::<_, i64>("likes")? as u64,
        dislikes: row.get::<_, i64>("dislikes")? as u64,
        rating: row.get("rating")?,
        tags,
        upload_date: row.get("upload_date")?,
        ..Default::default()
    })
}

pub fn cache_video(v: &VideoSummary) {
    if v.id.is_empty() {
        return;
    }
    let db = db();
    let conn = db.lock().unwrap();
    let _ = conn.execute(
        r#"INSERT INTO videos (id, title, uploader, uploader_username, thumbnail_url,
             preview_url, views, duration, duration_seconds, aspect_ratio, likes,
             dislikes, rating, tags, upload_date, updated_at)
           VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
           ON CONFLICT(id) DO UPDATE SET
             title=excluded.title, uploader=excluded.uploader,
             uploader_username=excluded.uploader_username,
             thumbnail_url=excluded.thumbnail_url, preview_url=excluded.preview_url,
             views=excluded.views, duration=excluded.duration,
             duration_seconds=excluded.duration_seconds, aspect_ratio=excluded.aspect_ratio,
             likes=excluded.likes, dislikes=excluded.dislikes, rating=excluded.rating,
             tags=excluded.tags, upload_date=excluded.upload_date,
             updated_at=excluded.updated_at"#,
        params![
            v.id,
            v.title,
            v.uploader,
            v.uploader_username,
            v.thumbnail_url,
            v.preview_url,
            v.views as i64,
            v.duration,
            v.duration_seconds as i64,
            v.aspect_ratio,
            v.likes as i64,
            v.dislikes as i64,
            v.rating,
            serde_json::to_string(&v.tags).unwrap_or_else(|_| "[]".into()),
            v.upload_date,
            now_ms(),
        ],
    );
}

pub fn cache_videos(vs: &[VideoSummary]) {
    for v in vs {
        cache_video(v);
    }
}

pub fn get_cached_summary(id: &str) -> Option<VideoSummary> {
    let db = db();
    let conn = db.lock().unwrap();
    conn.query_row("SELECT * FROM videos WHERE id = ?1", params![id], |r| {
        row_to_summary(r)
    })
    .optional()
    .ok()
    .flatten()
}

/// Upsert a watched entry. Returns true if newly inserted.
pub fn upsert_history(entry: &HistoryEntry, source: &str) -> bool {
    cache_video(&entry.video);
    let watched_at = chrono::DateTime::parse_from_rfc3339(&entry.watched_at)
        .map(|d| d.timestamp_millis())
        .unwrap_or_else(|_| now_ms());

    let db = db();
    let conn = db.lock().unwrap();
    let existing: bool = conn
        .query_row(
            "SELECT 1 FROM watch_history WHERE video_id = ?1",
            params![entry.video.id],
            |_| Ok(true),
        )
        .optional()
        .ok()
        .flatten()
        .unwrap_or(false);

    let _ = conn.execute(
        r#"INSERT INTO watch_history (video_id, watched_at, progress, source, first_seen_at)
           VALUES (?1, ?2, ?3, ?4, ?5)
           ON CONFLICT(video_id) DO UPDATE SET
             watched_at = MAX(watch_history.watched_at, excluded.watched_at),
             progress = MAX(watch_history.progress, excluded.progress)"#,
        params![entry.video.id, watched_at, entry.progress, source, now_ms()],
    );
    !existing
}

pub fn get_history_page(page: u32, limit: u32) -> (Vec<HistoryEntry>, u64) {
    let db = db();
    let conn = db.lock().unwrap();
    let total: u64 = conn
        .query_row("SELECT COUNT(*) FROM watch_history", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as u64;
    let offset = (page.saturating_sub(1) as u64) * limit as u64;
    let mut stmt = conn
        .prepare(
            r#"SELECT h.watched_at AS watched_at, h.progress AS progress, v.*
               FROM watch_history h JOIN videos v ON v.id = h.video_id
               ORDER BY h.watched_at DESC LIMIT ?1 OFFSET ?2"#,
        )
        .unwrap();
    let items = stmt
        .query_map(params![limit as i64, offset as i64], |r| {
            let watched_at: i64 = r.get("watched_at")?;
            let progress: f64 = r.get("progress")?;
            let video = row_to_summary(r)?;
            Ok(HistoryEntry {
                video,
                watched_at: chrono::DateTime::from_timestamp_millis(watched_at)
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_default(),
                progress,
            })
        })
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect();
    (items, total)
}

pub fn history_count() -> u64 {
    let db = db();
    let conn = db.lock().unwrap();
    conn.query_row("SELECT COUNT(*) FROM watch_history", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as u64
}

pub fn watched_progress_map() -> HashMap<String, f64> {
    let db = db();
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT video_id, progress FROM watch_history")
        .unwrap();
    let mut map = HashMap::new();
    if let Ok(rows) = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
    }) {
        for row in rows.flatten() {
            map.insert(row.0, row.1);
        }
    }
    map
}

#[derive(Clone, Copy)]
pub enum Bucket {
    Favorites,
    WatchLater,
}

impl Bucket {
    fn table(self) -> &'static str {
        match self {
            Self::Favorites => "favorites",
            Self::WatchLater => "watch_later",
        }
    }
}

pub fn set_local_bucket(bucket: Bucket, video: &VideoSummary, on: bool) {
    if on {
        cache_video(video);
        let db = db();
        let conn = db.lock().unwrap();
        let sql = format!(
            "INSERT INTO {} (video_id, added_at) VALUES (?1, ?2) ON CONFLICT(video_id) DO NOTHING",
            bucket.table()
        );
        let _ = conn.execute(&sql, params![video.id, now_ms()]);
    } else {
        let db = db();
        let conn = db.lock().unwrap();
        let sql = format!("DELETE FROM {} WHERE video_id = ?1", bucket.table());
        let _ = conn.execute(&sql, params![video.id]);
    }
}

pub fn get_bucket(bucket: Bucket) -> Vec<VideoSummary> {
    let db = db();
    let conn = db.lock().unwrap();
    let sql = format!(
        "SELECT v.* FROM {} b JOIN videos v ON v.id = b.video_id ORDER BY b.added_at DESC",
        bucket.table()
    );
    let mut stmt = conn.prepare(&sql).unwrap();
    stmt.query_map([], |r| row_to_summary(r))
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn bucket_ids(bucket: Bucket) -> std::collections::HashSet<String> {
    let db = db();
    let conn = db.lock().unwrap();
    let sql = format!("SELECT video_id FROM {}", bucket.table());
    let mut stmt = conn.prepare(&sql).unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect()
}
