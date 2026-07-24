use crate::models::{HistoryEntry, PlayableVideo, VideoDetail, VideoSummary};
use crate::paths;
use crate::services::pmv::shared_client;
use crate::services::queue;
use crate::services::repo::{cache_video, upsert_history};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// Persisted playback session (mirrors v1 `ph_now_playing_v1`).
/// `at` is playback position in **seconds**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NowPlayingSession {
    pub video: PlayableVideo,
    pub at: f64,
}

pub fn load_now_playing() -> Option<NowPlayingSession> {
    let data = std::fs::read_to_string(paths::now_playing_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_now_playing(video: &PlayableVideo, at_secs: f64) {
    let _ = paths::ensure_data_dir();
    let session = NowPlayingSession {
        video: video.clone(),
        at: at_secs.max(0.0),
    };
    if let Ok(json) = serde_json::to_string_pretty(&session) {
        let _ = std::fs::write(paths::now_playing_path(), json);
    }
}

pub fn clear_now_playing() {
    let _ = std::fs::remove_file(paths::now_playing_path());
}

/// Restore last session into UI signals (call once after bootstrap).
pub fn restore_into(
    mut now_playing: Signal<Option<PlayableVideo>>,
    mut start_at: Signal<f64>,
) -> bool {
    let Some(session) = load_now_playing() else {
        return false;
    };
    if session.video.summary.id.is_empty() || session.video.video_url.is_empty() {
        clear_now_playing();
        return false;
    }
    cache_video(&session.video.summary);
    start_at.set(session.at);
    now_playing.set(Some(session.video));
    true
}

/// Apply a fully-fetched detail into the persistent player. Same id = no-op (keeps progress).
/// `start_at` is seconds.
pub fn play_detail(
    mut now_playing: Signal<Option<PlayableVideo>>,
    mut start_at: Signal<f64>,
    detail: &VideoDetail,
) {
    let id = detail.summary.id.as_str();
    if now_playing
        .read()
        .as_ref()
        .is_some_and(|p| p.summary.id == id)
    {
        return;
    }
    cache_video(&detail.summary);
    let resume_secs = if detail.watch_progress > 0.01 && detail.watch_progress < 0.95 {
        detail.watch_progress * detail.summary.duration_seconds.max(1) as f64
    } else {
        0.0
    };
    let playable = PlayableVideo::from(detail);
    start_at.set(resume_secs);
    now_playing.set(Some(playable.clone()));
    save_now_playing(&playable, resume_secs);
    upsert_history(
        &HistoryEntry {
            video: detail.summary.clone(),
            watched_at: chrono::Utc::now().to_rfc3339(),
            progress: detail.watch_progress.max(0.01),
        },
        "local",
    );
}

/// Fetch detail then play. Returns false on failure.
pub async fn play_id(
    id: &str,
    now_playing: Signal<Option<PlayableVideo>>,
    start_at: Signal<f64>,
) -> bool {
    match shared_client().get_video(id).await {
        Ok(detail) => {
            play_detail(now_playing, start_at, &detail);
            let client = shared_client();
            let vid = id.to_string();
            spawn(async move {
                client.record_view(&vid).await;
            });
            true
        }
        Err(e) => {
            tracing::warn!("play_id({id}) failed: {e}");
            false
        }
    }
}

pub fn stop(mut now_playing: Signal<Option<PlayableVideo>>, mut start_at: Signal<f64>) {
    clear_now_playing();
    start_at.set(0.0);
    now_playing.set(None);
    spawn(async move {
        let _ = document::eval(
            r#"
            const v = document.getElementById('pmv-player');
            if (v) {
              try {
                v.pause();
                v.removeAttribute('src');
                delete v.dataset.boundSrc;
                v.load();
              } catch (e) {}
            }
            if (window.__hls) { try { window.__hls.destroy(); } catch (e) {} window.__hls = null; }
            window.__pmvEnded = false;
            window.__pmvProgress = null;
            return 'ok';
            "#,
        )
        .await;
    });
}

/// Advance to the next queued item after the current video ends.
pub async fn advance_queue(
    now_playing: Signal<Option<PlayableVideo>>,
    start_at: Signal<f64>,
    mut queue_tick: Signal<u32>,
) {
    let Some(next) = queue::shift() else {
        // Keep last frame; clear session so relaunch doesn't restart a finished clip.
        clear_now_playing();
        return;
    };
    queue_tick.set(queue_tick() + 1);
    let _ = play_id(&next.id, now_playing, start_at).await;
}

/// Play a queued item: stop current, play clicked, remove only that item from the queue.
pub async fn play_from_queue(
    id: &str,
    now_playing: Signal<Option<PlayableVideo>>,
    start_at: Signal<f64>,
    mut queue_tick: Signal<u32>,
) {
    // Slice this video out of the queue; leave every other entry alone.
    let _ = queue::take(id);
    queue_tick.set(queue_tick() + 1);
    let _ = play_id(id, now_playing, start_at).await;
}

/// Card / list open intent: idle → play; same → noop; busy → caller shows choice.
pub enum OpenIntent {
    Play,
    AlreadyPlaying,
    Choice(VideoSummary),
}

pub fn open_intent(
    video: &VideoSummary,
    now_playing: Signal<Option<PlayableVideo>>,
) -> OpenIntent {
    match now_playing.read().as_ref() {
        None => OpenIntent::Play,
        Some(p) if p.summary.id == video.id => OpenIntent::AlreadyPlaying,
        Some(_) => OpenIntent::Choice(video.clone()),
    }
}

/// Keep only the newest window of browse/search results in memory.
pub fn trim_front<T>(items: &mut Vec<T>, max: usize) {
    if items.len() > max {
        let excess = items.len() - max;
        items.drain(0..excess);
    }
}
