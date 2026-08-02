use crate::models::{HistoryEntry, SyncProgress, SyncResult};
use crate::services::db::db;
use crate::services::pmv::{is_connected, shared_client};
use crate::services::repo::{cache_videos, get_cached_summary, upsert_history};
use rusqlite::params;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static RUNNING: AtomicBool = AtomicBool::new(false);
static PROGRESS: Mutex<Option<SyncProgress>> = Mutex::new(None);

pub fn is_syncing() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

pub fn sync_progress() -> Option<SyncProgress> {
    PROGRESS.lock().unwrap().clone()
}

pub fn last_sync() -> Option<(Option<i64>, i64, String)> {
    let db = db();
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT finished_at, new_count, status FROM sync_log WHERE kind LIKE 'history%' ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .ok()
}

fn set_progress(p: SyncProgress) {
    *PROGRESS.lock().unwrap() = Some(p);
}

pub async fn sync_watch_history() -> SyncResult {
    if !is_connected() {
        return SyncResult {
            status: "skipped".into(),
            new_count: 0,
            seen_count: 0,
            total_retained: 0,
            message: Some("Not connected".into()),
        };
    }
    if RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return SyncResult {
            status: "skipped".into(),
            new_count: 0,
            seen_count: 0,
            total_retained: 0,
            message: Some("Sync already running".into()),
        };
    }

    set_progress(SyncProgress {
        phase: "starting".into(),
        processed: 0,
        total: 0,
        new_count: 0,
        total_retained: 0,
        message: None,
    });

    let log_id: i64 = {
        let db = db();
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_log (kind, started_at, status) VALUES (?1, ?2, 'running')",
            params!["history:sync", chrono::Utc::now().timestamp_millis()],
        )
        .ok();
        conn.last_insert_rowid()
    };

    let mut new_count = 0u64;
    let mut seen_count = 0u64;

    let result = async {
        set_progress(SyncProgress {
            phase: "fetching".into(),
            processed: 0,
            total: 0,
            new_count: 0,
            total_retained: 0,
            message: Some("Fetching history from PMVHaven…".into()),
        });

        let client = shared_client();
        let remote = client.fetch_remote_history().await.map_err(|e| e.to_string())?;
        let total = remote.entries.len() as u64;

        set_progress(SyncProgress {
            phase: "fetching".into(),
            processed: 0,
            total,
            new_count: 0,
            total_retained: remote.total_retained,
            message: Some("Fetching history from PMVHaven…".into()),
        });

        if total == 0 {
            let db = db();
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE sync_log SET finished_at = ?1, status = 'ok', new_count = 0, seen_count = 0 WHERE id = ?2",
                params![chrono::Utc::now().timestamp_millis(), log_id],
            );
            set_progress(SyncProgress {
                phase: "done".into(),
                processed: 0,
                total: 0,
                new_count: 0,
                total_retained: remote.total_retained,
                message: None,
            });
            return Ok::<SyncResult, String>(SyncResult {
                status: "ok".into(),
                new_count: 0,
                seen_count: 0,
                total_retained: remote.total_retained,
                message: None,
            });
        }

        set_progress(SyncProgress {
            phase: "hydrating".into(),
            processed: 0,
            total,
            new_count: 0,
            total_retained: remote.total_retained,
            message: Some("Loading video details…".into()),
        });

        let uncached: Vec<String> = remote
            .entries
            .iter()
            .map(|e| e.video_id.clone())
            .filter(|id| get_cached_summary(id).is_none())
            .collect();

        for (i, chunk) in uncached.chunks(100).enumerate() {
            let videos = client
                .get_videos_bulk(&chunk.to_vec())
                .await
                .unwrap_or_default();
            cache_videos(&videos);
            set_progress(SyncProgress {
                phase: "hydrating".into(),
                processed: ((i + 1) * 100).min(uncached.len()) as u64,
                total: total.max(uncached.len() as u64),
                new_count: 0,
                total_retained: remote.total_retained,
                message: Some("Loading video details…".into()),
            });
        }

        set_progress(SyncProgress {
            phase: "saving".into(),
            processed: 0,
            total,
            new_count: 0,
            total_retained: remote.total_retained,
            message: Some("Saving to library…".into()),
        });

        let mut processed = 0u64;
        for e in &remote.entries {
            let Some(summary) = get_cached_summary(&e.video_id) else {
                processed += 1;
                continue;
            };
            let inserted = upsert_history(
                &HistoryEntry {
                    video: summary,
                    watched_at: e.watched_at.clone(),
                    progress: e.progress,
                },
                "sync",
            );
            if inserted {
                new_count += 1;
            } else {
                seen_count += 1;
            }
            processed += 1;
        }

        {
            let db = db();
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE sync_log SET finished_at = ?1, status = 'ok', new_count = ?2, seen_count = ?3 WHERE id = ?4",
                params![
                    chrono::Utc::now().timestamp_millis(),
                    new_count as i64,
                    seen_count as i64,
                    log_id
                ],
            );
        }

        set_progress(SyncProgress {
            phase: "done".into(),
            processed,
            total,
            new_count,
            total_retained: remote.total_retained,
            message: None,
        });

        Ok(SyncResult {
            status: "ok".into(),
            new_count,
            seen_count,
            total_retained: remote.total_retained,
            message: None,
        })
    }
    .await;

    RUNNING.store(false, Ordering::SeqCst);

    match result {
        Ok(r) => r,
        Err(message) => {
            let db = db();
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE sync_log SET finished_at = ?1, status = 'error', new_count = ?2, seen_count = ?3, message = ?4 WHERE id = ?5",
                params![
                    chrono::Utc::now().timestamp_millis(),
                    new_count as i64,
                    seen_count as i64,
                    message,
                    log_id
                ],
            );
            set_progress(SyncProgress {
                phase: "done".into(),
                processed: 0,
                total: 0,
                new_count,
                total_retained: 0,
                message: Some(message.clone()),
            });
            SyncResult {
                status: "error".into(),
                new_count,
                seen_count,
                total_retained: 0,
                message: Some(message),
            }
        }
    }
}

/// Push local SQLite watch history up to PMVHaven (`PUT /users/watch-progress`).
pub async fn push_local_history() -> SyncResult {
    if !is_connected() {
        return SyncResult {
            status: "skipped".into(),
            new_count: 0,
            seen_count: 0,
            total_retained: 0,
            message: Some("Not connected".into()),
        };
    }
    if RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return SyncResult {
            status: "skipped".into(),
            new_count: 0,
            seen_count: 0,
            total_retained: 0,
            message: Some("Sync already running".into()),
        };
    }

    set_progress(SyncProgress {
        phase: "pushing".into(),
        processed: 0,
        total: 0,
        new_count: 0,
        total_retained: 0,
        message: Some("Pushing local history to PMVHaven…".into()),
    });

    let (entries, total) = crate::services::repo::get_history_page(1, 500);
    let total_u = total.max(entries.len() as u64);
    let client = shared_client();
    let mut pushed = 0u64;
    let mut failed = 0u64;

    for (i, entry) in entries.iter().enumerate() {
        let pct = (entry.progress * 100.0).round().clamp(1.0, 100.0) as u32;
        let dur = entry.video.duration_seconds.max(1);
        match client
            .push_watch_progress(&entry.video.id, pct, dur)
            .await
        {
            Ok(()) => {
                pushed += 1;
                // Also bump view once so it lands in remote watchHistory.
                client.record_view(&entry.video.id).await;
            }
            Err(_) => failed += 1,
        }
        if i % 5 == 0 {
            set_progress(SyncProgress {
                phase: "pushing".into(),
                processed: (i + 1) as u64,
                total: total_u,
                new_count: pushed,
                total_retained: total_u,
                message: Some(format!("Pushed {pushed}…")),
            });
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
    }

    RUNNING.store(false, Ordering::SeqCst);
    set_progress(SyncProgress {
        phase: "done".into(),
        processed: entries.len() as u64,
        total: total_u,
        new_count: pushed,
        total_retained: total_u,
        message: None,
    });

    SyncResult {
        status: if failed == 0 { "ok".into() } else { "partial".into() },
        new_count: pushed,
        seen_count: failed,
        total_retained: total_u,
        message: Some(format!("Pushed {pushed}, failed {failed}")),
    }
}
