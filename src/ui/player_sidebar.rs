use crate::models::{HistoryEntry, PlayableVideo};
use crate::services::player::{self, advance_queue, play_from_queue};
use crate::services::queue;
use crate::services::repo::{get_cached_summary, upsert_history};
use crate::services::stream_proxy::proxied_url;
use crate::ui::nav::Route;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_HIST_MS: AtomicU64 = AtomicU64::new(0);
static LAST_SAVE_MS: AtomicU64 = AtomicU64::new(0);
static LAST_PUSH_MS: AtomicU64 = AtomicU64::new(0);
/// Last known playback position (ms) — updated without reactive signal churn.
static LAST_POS_MS: AtomicU64 = AtomicU64::new(0);

/// Right-side persistent player + queue. Survives route changes.
#[component]
pub fn PlayerSidebar() -> Element {
    let now_playing = use_context::<Signal<Option<PlayableVideo>>>();
    let mut start_at = use_context::<Signal<f64>>();
    let proxy_base = use_context::<Signal<String>>();
    let mut queue_tick = use_context::<Signal<u32>>();
    let mut queue_open = use_context::<Signal<bool>>();
    let mut watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let player_fs = use_context::<Signal<bool>>();
    let navigator = use_navigator();

    let queue_items = use_memo(move || {
        let _ = queue_tick();
        queue::snapshot().items
    });

    let playing_id = use_memo(move || {
        now_playing()
            .map(|p| p.summary.id)
            .unwrap_or_default()
    });

    // One long-lived JS↔Rust channel. No polling evals — those fight WebKit
    // decode/scrub and are the main source of playback jank.
    use_future(move || async move {
        loop {
            let mut eval = document::eval(
                r#"
                window.__pmvSend = (msg) => {
                  try { dioxus.send(String(msg)); } catch (e) {}
                };
                window.__pmvReorderSend = (payload) => {
                  window.__pmvSend('reorder|' + payload);
                };
                await new Promise(() => {});
                "#,
            );
            loop {
                match eval.recv::<String>().await {
                    Ok(raw) => {
                        handle_player_msg(
                            &raw,
                            now_playing,
                            start_at,
                            queue_tick,
                            player_fs,
                            &mut watched_map,
                        )
                        .await;
                    }
                    Err(e) => {
                        tracing::warn!("player channel closed: {e}");
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
    });

    // Attach stream in JS only when the video *id* changes — same as v1 VideoStage.
    // Never put `src` in RSX: Dioxus re-applying it on fullscreen re-renders
    // restarts/stalls GStreamer decode (v1 avoided this by using a ref + attachStream).
    use_effect(move || {
        let id = playing_id();
        if id.is_empty() {
            return;
        }
        let resume = *start_at.peek();
        let src = now_playing
            .peek()
            .as_ref()
            .map(|p| {
                if !p.video_url.is_empty() {
                    p.video_url.clone()
                } else {
                    proxied_url(
                        &proxy_base.peek(),
                        p.hls_master_playlist_url.as_deref().unwrap_or(""),
                    )
                }
            })
            .unwrap_or_default();
        LAST_POS_MS.store((resume * 1000.0) as u64, Ordering::Relaxed);
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            let _ = document::eval(&format!(
                r#"
                const el = document.getElementById('pmv-player');
                if (!el) return 'no-el';
                el.dataset.vid = {id:?};
                el.setAttribute('controlsList', 'nofullscreen nodownload noremoteplayback');
                try {{ el.disablePictureInPicture = true; }} catch (e) {{}}

                const send = (msg) => {{
                  if (typeof window.__pmvSend === 'function') window.__pmvSend(msg);
                }};

                // Native WebKit video-fullscreen is black — bounce to our FS.
                if (!window.__pmvBailFsBound) {{
                  window.__pmvBailFsBound = true;
                  const bailFs = () => {{
                    const doc = document;
                    const fsEl = doc.fullscreenElement || doc.webkitFullscreenElement || null;
                    if (fsEl === el) {{
                      try {{ doc.exitFullscreen && doc.exitFullscreen(); }} catch (e) {{}}
                      try {{ doc.webkitExitFullscreen && doc.webkitExitFullscreen(); }} catch (e) {{}}
                      send('toggle-fs');
                    }}
                  }};
                  document.addEventListener('fullscreenchange', bailFs);
                  document.addEventListener('webkitfullscreenchange', bailFs);
                }}

                if (!window.__pmvFsEsc) {{
                  window.__pmvFsEsc = (e) => {{
                    if (e.key === 'Escape' && window.__pmvFs) send('toggle-fs');
                  }};
                  window.addEventListener('keydown', window.__pmvFsEsc);
                }}

                const resume = {resume};
                const src = {src:?};
                window.__pmvResumePending = resume > 1;
                window.__pmvSeeking = false;
                let lastSend = 0;

                // Only (re)assign src when the bound video changes — never on FS toggle.
                if (src && el.dataset.boundSrc !== src) {{
                  el.dataset.boundSrc = src;
                  el.src = resume > 1
                    ? (src + '#t=' + Math.floor(resume))
                    : src;
                }}

                const snap = () => {{
                  if (!el.duration || el.duration < 1) return null;
                  return {{
                    id: el.dataset.vid || '',
                    t: el.currentTime || 0,
                    progress: (el.currentTime || 0) / el.duration,
                    paused: !!el.paused
                  }};
                }};

                const maybeSendProg = (force) => {{
                  if (window.__pmvResumePending || window.__pmvSeeking) return;
                  const s = snap();
                  if (!s || s.t < 1) return;
                  const now = Date.now();
                  if (!force && now - lastSend < 2500) return;
                  lastSend = now;
                  send('prog|' + s.id + '|' + s.t.toFixed(2) + '|' + s.progress.toFixed(4));
                }};

                el.onended = () => send('ended');
                el.onseeking = () => {{ window.__pmvSeeking = true; }};
                el.onseeked = () => {{
                  window.__pmvSeeking = false;
                  maybeSendProg(true);
                }};
                el.onpause = () => maybeSendProg(true);
                el.ontimeupdate = () => maybeSendProg(false);

                const applyResume = () => {{
                  if (!(resume > 1)) {{
                    window.__pmvResumePending = false;
                    return;
                  }}
                  try {{
                    if (el.duration && resume < el.duration) el.currentTime = resume;
                  }} catch (e) {{}}
                  setTimeout(() => {{
                    window.__pmvResumePending = false;
                    maybeSendProg(true);
                  }}, 400);
                }};
                if (el.readyState >= 1) applyResume();
                else el.addEventListener('loadedmetadata', applyResume, {{ once: true }});

                const p = el.play();
                if (p && p.catch) p.catch(() => {{}});
                return 'ok';
                "#
            ))
            .await;
        });
    });

    let visible = now_playing().is_some() || !queue_items().is_empty() || queue_open();
    if !visible {
        return rsx! {};
    }

    let fs = player_fs();

    rsx! {
        aside { class: if fs { "player-sidebar fullscreen" } else { "player-sidebar" },
            if let Some(playable) = now_playing() {
                {
                    let id = playable.summary.id.clone();
                    let title = playable.summary.title.clone();
                    let thumb = playable.summary.thumbnail_url.clone();
                    let has_url = !playable.video_url.is_empty()
                        || playable
                            .hls_master_playlist_url
                            .as_ref()
                            .is_some_and(|u| !u.is_empty());

                    rsx! {
                        div { class: "player-stage",
                            if !has_url {
                                div { class: "player-error", "No stream URL for this video." }
                            } else {
                                // No `src` / no title attr — src is JS-bound; title would
                                // show a browser tooltip over the player (v1 avoided this).
                                video {
                                    key: "{id}",
                                    id: "pmv-player",
                                    class: "player-video",
                                    poster: "{thumb}",
                                    controls: true,
                                    playsinline: true,
                                    preload: "auto",
                                }
                            }
                            button {
                                class: "player-fs-btn",
                                title: if fs { "Exit fullscreen (Esc)" } else { "Fullscreen" },
                                onclick: move |_| {
                                    set_fullscreen_mode(!player_fs(), player_fs);
                                },
                                if fs { "⤡" } else { "⤢" }
                            }
                        }
                        div { class: if fs { "player-meta is-hidden" } else { "player-meta" },
                            div { class: "player-title", title: "{title}", "{title}" }
                            button {
                                class: "icon-btn",
                                title: "Open details",
                                onclick: {
                                    let id = id.clone();
                                    move |_| {
                                        if player_fs() {
                                            set_fullscreen_mode(false, player_fs);
                                        }
                                        navigator.push(Route::Watch { id: id.clone() });
                                    }
                                },
                                "↗"
                            }
                            button {
                                class: "icon-btn",
                                title: "Close player",
                                onclick: move |_| {
                                    flush_position(now_playing, start_at);
                                    if player_fs() {
                                        set_fullscreen_mode(false, player_fs);
                                    }
                                    player::stop(now_playing, start_at);
                                },
                                "✕"
                            }
                        }
                    }
                }
            } else {
                div { class: "player-meta",
                    div { class: "player-title", "Queue" }
                    button {
                        class: "icon-btn",
                        onclick: move |_| queue_open.set(false),
                        "✕"
                    }
                }
            }

            // Keep queue mounted (hidden in FS) so toggling FS doesn't churn the DOM
            // around the <video> element.
            div {
                class: if fs { "sidebar-queue is-hidden" } else { "sidebar-queue" },
                div { class: "sidebar-queue-header",
                    span { "Up next ({queue_items().len()})" }
                    if !queue_items().is_empty() {
                        button {
                            class: "icon-btn",
                            onclick: move |_| {
                                queue::clear();
                                queue_tick.set(queue_tick() + 1);
                            },
                            "Clear"
                        }
                    }
                }
                if !queue_items().is_empty() {
                    p { class: "queue-hint", "Drag ⠿ to reorder · click to play · ✕ removes" }
                }
                div { id: "pmv-queue-list", class: "sidebar-queue-list",
                    if queue_items().is_empty() {
                        p { class: "muted", style: "padding:0.75rem;", "Queue is empty — add videos while you browse." }
                    } else {
                        for (i, v) in queue_items().into_iter().enumerate() {
                            {
                                let vid = v.id.clone();
                                rsx! {
                                    div {
                                        key: "{v.id}",
                                        id: "qi-{i}",
                                        class: "queue-card",
                                        "data-qi": "{i}",
                                        button {
                                            class: "queue-grip",
                                            title: "Drag to reorder",
                                            onpointerdown: move |evt| {
                                                evt.prevent_default();
                                                spawn(async move {
                                                    let _ = document::eval(&start_drag_js(i)).await;
                                                });
                                            },
                                            "⠿"
                                        }
                                        img {
                                            src: "{v.thumbnail_url}",
                                            alt: "",
                                            draggable: false,
                                            onclick: {
                                                let vid = vid.clone();
                                                move |_| {
                                                    let id = vid.clone();
                                                    spawn(async move {
                                                        play_from_queue(
                                                            &id,
                                                            now_playing,
                                                            start_at,
                                                            queue_tick,
                                                        )
                                                        .await;
                                                    });
                                                }
                                            },
                                        }
                                        div {
                                            class: "queue-card-meta",
                                            onclick: {
                                                let vid = vid.clone();
                                                move |_| {
                                                    let id = vid.clone();
                                                    spawn(async move {
                                                        play_from_queue(
                                                            &id,
                                                            now_playing,
                                                            start_at,
                                                            queue_tick,
                                                        )
                                                        .await;
                                                    });
                                                }
                                            },
                                            div { class: "queue-card-title", "{v.title}" }
                                            div { class: "queue-card-sub",
                                                "{v.uploader_username}"
                                                if !v.duration.is_empty() {
                                                    " · {v.duration}"
                                                }
                                                if v.rating > 0.0 {
                                                    " · ★ {v.rating.round() as i32}%"
                                                }
                                            }
                                        }
                                        button {
                                            class: "icon-btn queue-remove",
                                            title: "Remove from queue",
                                            onclick: {
                                                let id = v.id.clone();
                                                move |_| {
                                                    queue::remove(&id);
                                                    queue_tick.set(queue_tick() + 1);
                                                }
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn handle_player_msg(
    raw: &str,
    now_playing: Signal<Option<PlayableVideo>>,
    mut start_at: Signal<f64>,
    mut queue_tick: Signal<u32>,
    player_fs: Signal<bool>,
    watched_map: &mut Signal<HashMap<String, f64>>,
) {
    if raw == "ended" {
        // v1 keeps fullscreen across queue advances — do not exit FS here.
        advance_queue(now_playing, start_at, queue_tick).await;
        return;
    }
    if raw == "toggle-fs" {
        set_fullscreen_mode(!player_fs(), player_fs);
        return;
    }
    if let Some(rest) = raw.strip_prefix("reorder|") {
        let mut parts = rest.split(',');
        if let (Some(a), Some(b)) = (parts.next(), parts.next()) {
            if let (Ok(from), Ok(to)) = (a.parse::<usize>(), b.parse::<usize>()) {
                if from != to {
                    queue::move_item(from, to);
                    queue_tick.set(queue_tick() + 1);
                }
            }
        }
        return;
    }
    if let Some(rest) = raw.strip_prefix("prog|") {
        let mut parts = rest.split('|');
        let vid = parts.next().unwrap_or("").to_string();
        let t: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let progress: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        if vid.is_empty() || t < 1.0 {
            return;
        }
        LAST_POS_MS.store((t * 1000.0) as u64, Ordering::Relaxed);

        let Some(playable) = now_playing.peek().clone() else {
            return;
        };
        if playable.summary.id != vid {
            return;
        }

        // Update reactive start_at rarely — close-flush / resume helpers only.
        let prev = *start_at.peek();
        if (t - prev).abs() >= 2.0 {
            start_at.set(t);
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let last_save = LAST_SAVE_MS.load(Ordering::Relaxed);
        if now_ms.saturating_sub(last_save) >= 2_500 {
            LAST_SAVE_MS.store(now_ms, Ordering::Relaxed);
            player::save_now_playing(&playable, t);
        }

        let last = LAST_HIST_MS.load(Ordering::Relaxed);
        if now_ms.saturating_sub(last) >= 20_000 {
            LAST_HIST_MS.store(now_ms, Ordering::Relaxed);
            if let Some(summary) = get_cached_summary(&vid) {
                upsert_history(
                    &HistoryEntry {
                        video: summary.clone(),
                        watched_at: chrono::Utc::now().to_rfc3339(),
                        progress,
                    },
                    "local",
                );
            }
            // Avoid rewriting the whole watched map if unchanged enough.
            let cur = watched_map.peek().get(&vid).copied().unwrap_or(-1.0);
            if (cur - progress).abs() >= 0.02 {
                watched_map.with_mut(|m| {
                    m.insert(vid.clone(), progress);
                });
            }

            // Backfill progress to PMVHaven when signed in.
            let last_push = LAST_PUSH_MS.load(Ordering::Relaxed);
            if now_ms.saturating_sub(last_push) >= 20_000 {
                LAST_PUSH_MS.store(now_ms, Ordering::Relaxed);
                let id = vid.clone();
                let pct = (progress * 100.0).round().clamp(1.0, 100.0) as u32;
                let dur = playable.summary.duration_seconds.max(1);
                spawn(async move {
                    let client = crate::services::pmv::shared_client();
                    let _ = client.push_watch_progress(&id, pct, dur).await;
                });
            }
        }
    }
}

fn start_drag_js(from: usize) -> String {
    format!(
        r#"
        const from = {from};
        const list = document.getElementById('pmv-queue-list');
        if (!list) return 'no-list';
        const cards = () => Array.from(list.querySelectorAll(':scope > .queue-card'));
        const srcCard = cards()[from];
        if (!srcCard) return 'no-card';
        let to = from;

        const ghost = srcCard.cloneNode(true);
        ghost.id = 'pmv-drag-ghost';
        ghost.classList.add('queue-card-ghost');
        ghost.querySelectorAll('button').forEach((b) => b.remove());
        const rect = srcCard.getBoundingClientRect();
        ghost.style.width = rect.width + 'px';
        ghost.style.left = rect.left + 'px';
        ghost.style.top = rect.top + 'px';
        document.body.appendChild(ghost);

        const paint = () => {{
          cards().forEach((el, idx) => {{
            el.classList.toggle('dragging', idx === from);
            el.classList.toggle('drag-over', idx === to && idx !== from);
          }});
        }};
        paint();

        const onMove = (ev) => {{
          ghost.style.left = (ev.clientX - 24) + 'px';
          ghost.style.top = (ev.clientY - 20) + 'px';
          const hit = document.elementFromPoint(ev.clientX, ev.clientY);
          const card = hit && hit.closest('#pmv-queue-list > .queue-card');
          if (!card || card === ghost) return;
          const idx = cards().indexOf(card);
          if (idx >= 0 && idx !== to) {{
            to = idx;
            paint();
          }}
        }};
        const finish = () => {{
          cards().forEach((el) => el.classList.remove('dragging', 'drag-over'));
          if (ghost && ghost.parentNode) ghost.parentNode.removeChild(ghost);
          window.removeEventListener('pointermove', onMove, true);
          window.removeEventListener('pointerup', finish, true);
          window.removeEventListener('pointercancel', finish, true);
          if (from !== to && typeof window.__pmvReorderSend === 'function') {{
            window.__pmvReorderSend(from + ',' + to);
          }}
        }};
        window.addEventListener('pointermove', onMove, true);
        window.addEventListener('pointerup', finish, true);
        window.addEventListener('pointercancel', finish, true);
        return 'dragging';
        "#
    )
}

fn flush_position(
    now_playing: Signal<Option<PlayableVideo>>,
    mut start_at: Signal<f64>,
) {
    if let Some(playable) = now_playing.peek().clone() {
        let t = (LAST_POS_MS.load(Ordering::Relaxed) as f64) / 1000.0;
        let t = if t >= 1.0 { t } else { *start_at.peek() };
        if t >= 1.0 {
            start_at.set(t);
            player::save_now_playing(&playable, t);
        }
    }
}

fn set_fullscreen_mode(on: bool, mut player_fs: Signal<bool>) {
    player_fs.set(on);
    // v1: in-flow fill + Tauri setFullscreen. Same here via wry/tao.
    // Native <video> fullscreen stays blocked (black on WebKitGTK).
    dioxus::desktop::window().set_fullscreen(on);
    spawn(async move {
        let _ = document::eval(&format!(
            r#"
            window.__pmvFs = {on};
            return 'ok';
            "#
        ))
        .await;
    });
}
