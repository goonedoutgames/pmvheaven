use crate::models::{HistoryEntry, PlayableVideo};
use crate::services::player::{self, advance_queue, play_from_queue};
use crate::services::queue;
use crate::services::repo::{get_cached_summary, upsert_history};
use crate::services::stream_proxy::proxied_url;
use crate::ui::ctx::{
    PlayerFs, PlayerQueueH, PlayerRailW, ProxyBase, QueueOpen, QueueTick, StartAt,
};
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
    let start_at = use_context::<StartAt>().0;
    let proxy_base = use_context::<ProxyBase>().0;
    let mut queue_tick = use_context::<QueueTick>().0;
    let mut queue_open = use_context::<QueueOpen>().0;
    let mut watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let player_fs = use_context::<PlayerFs>().0;
    let player_rail_w = use_context::<PlayerRailW>().0;
    let player_queue_h = use_context::<PlayerQueueH>().0;
    let navigator = use_navigator();
    // Per-session quality preference (mirrors PMVHaven): auto | height | original
    let mut player_quality = use_signal(|| "auto".to_string());

    let queue_items = use_memo(move || {
        let _ = queue_tick();
        queue::snapshot().items
    });
    let queue_total = use_memo(move || queue::format_items_duration(&queue_items()));
    let queue_count = use_memo(move || queue_items().len());

    let playing_id = use_memo(move || {
        now_playing()
            .map(|p| p.summary.id)
            .unwrap_or_default()
    });

    // Reset quality to Auto when the clip changes.
    use_effect(move || {
        let _id = playing_id();
        player_quality.set("auto".into());
        spawn(async move {
            let _ = document::eval(
                r#"
                window.__pmvQuality = 'auto';
                return 'ok';
                "#,
            )
            .await;
        });
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
                            player_rail_w,
                            player_queue_h,
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

    // Attach stream when the video id changes.
    // Match PMVHaven: HLS master when available, progressive Original as fallback.
    // Quality changes go through window.__pmvApplyQuality (no full rebind).
    use_effect(move || {
        let id = playing_id();
        if id.is_empty() {
            return;
        }
        let resume = *start_at.peek();
        let (file_src, hls_src, hint_ar, thumb) = now_playing
            .peek()
            .as_ref()
            .map(|p| {
                // Always proxy media through the local Referer-injecting proxy.
                // Direct CDN fetches from WebKit often fail (hotlink / auth).
                let raw_file = p.video_url.clone();
                let file_is_playlist = raw_file.contains(".m3u8");
                let file = if raw_file.is_empty() || file_is_playlist {
                    String::new()
                } else {
                    proxied_url(&proxy_base.peek(), &raw_file)
                };
                let hls = p
                    .hls_master_playlist_url
                    .as_deref()
                    .filter(|u| !u.is_empty())
                    .or_else(|| {
                        // Some API payloads put the master playlist in videoUrl.
                        file_is_playlist.then_some(raw_file.as_str())
                    })
                    .map(|u| proxied_url(&proxy_base.peek(), u))
                    .filter(|u| !u.is_empty())
                    .unwrap_or_default();
                let ar = if p.summary.aspect_ratio > 0.0 {
                    p.summary.aspect_ratio
                } else {
                    16.0 / 9.0
                };
                let thumb = p.summary.thumbnail_url.clone();
                (file, hls, ar, thumb)
            })
            .unwrap_or((String::new(), String::new(), 16.0 / 9.0, String::new()));
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

                if (!el.dataset.dblFs) {{
                  el.dataset.dblFs = '1';
                  el.addEventListener('dblclick', (e) => {{
                    e.preventDefault();
                    send('toggle-fs');
                  }});
                }}

                // Stage box tracks the clip AR (refined from decoded frames).
                // Fitting is always CSS object-fit:contain — never cover/crop.
                const applyStageAr = (ar) => {{
                  const stage = el.closest('.player-stage');
                  if (!stage || !(ar > 0) || !Number.isFinite(ar)) return;
                  stage.style.setProperty('--player-ar', String(ar));
                  stage.classList.toggle('is-portrait', ar < 1);
                  stage.classList.remove('is-cover');
                  el.style.objectFit = 'contain';
                  el.style.objectPosition = 'center';
                }};
                applyStageAr({hint_ar});
                if (!el.dataset.arListen) {{
                  el.dataset.arListen = '1';
                  const refineAr = () => {{
                    const w = el.videoWidth | 0;
                    const h = el.videoHeight | 0;
                    if (w > 0 && h > 0) applyStageAr(w / h);
                  }};
                  el.addEventListener('loadedmetadata', refineAr);
                  el.addEventListener('loadeddata', refineAr);
                }} else if (el.videoWidth > 0 && el.videoHeight > 0) {{
                  applyStageAr(el.videoWidth / el.videoHeight);
                }}

                // Avoid WebView poster+controls composite clipping on queue switches:
                // show thumb on the stage until the first decoded frame, then enable controls.
                const stageEl = el.closest('.player-stage');
                const thumb = {thumb:?};
                const revealFrame = () => {{
                  el.controls = true;
                  if (stageEl) {{
                    stageEl.classList.remove('is-awaiting-frame');
                    stageEl.style.removeProperty('--player-poster');
                  }}
                  try {{ el.removeAttribute('poster'); }} catch (e) {{}}
                }};
                el.controls = false;
                try {{ el.removeAttribute('poster'); }} catch (e) {{}}
                if (stageEl) {{
                  stageEl.classList.add('is-awaiting-frame');
                  if (thumb) stageEl.style.setProperty('--player-poster', 'url(' + JSON.stringify(thumb) + ')');
                }}
                if (el.readyState >= 2) revealFrame();
                else {{
                  el.addEventListener('loadeddata', revealFrame, {{ once: true }});
                  el.addEventListener('playing', revealFrame, {{ once: true }});
                  el.addEventListener('error', revealFrame, {{ once: true }});
                }}

                const resume = {resume};
                const fileSrc = {file_src:?};
                const hlsSrc = {hls_src:?};
                window.__pmvResumePending = resume > 1;
                window.__pmvSeeking = false;
                window.__pmvFileSrc = fileSrc;
                window.__pmvHlsSrc = hlsSrc;
                if (!window.__pmvQuality) window.__pmvQuality = 'auto';
                let lastSend = 0;

                const destroyHls = () => {{
                  if (window.__hls) {{
                    try {{ window.__hls.destroy(); }} catch (e) {{}}
                    window.__hls = null;
                  }}
                }};

                const heightPref = (q) => {{
                  if (!q || q === 'auto' || q === 'original') return -1;
                  const n = parseInt(q, 10);
                  return Number.isFinite(n) ? n : -1;
                }};

                const applyQuality = (hls, q) => {{
                  if (!hls || !hls.levels || !hls.levels.length) return;
                  const want = heightPref(q);
                  if (want < 0) {{
                    hls.capLevelToPlayerSize = true;
                    hls.autoLevelCapping = -1;
                    hls.currentLevel = -1;
                    return;
                  }}
                  hls.capLevelToPlayerSize = false;
                  let best = 0;
                  let bestDist = Infinity;
                  for (let i = 0; i < hls.levels.length; i++) {{
                    const ht = hls.levels[i].height || 0;
                    const dist = Math.abs(ht - want);
                    if (dist < bestDist) {{ bestDist = dist; best = i; }}
                  }}
                  hls.currentLevel = best;
                  hls.loadLevel = best;
                }};

                const playFile = () => {{
                  destroyHls();
                  if (!fileSrc) return 'no-file';
                  el.dataset.boundSrc = 'file:' + fileSrc;
                  el.src = resume > 1 ? (fileSrc + '#t=' + Math.floor(resume)) : fileSrc;
                  const onErr = () => {{
                    el.removeEventListener('error', onErr);
                    if (hlsSrc) playHls();
                    else revealFrame();
                  }};
                  el.addEventListener('error', onErr, {{ once: true }});
                  const p = el.play();
                  if (p && p.catch) p.catch(() => {{}});
                  return 'file';
                }};

                const playHls = () => {{
                  if (!hlsSrc) return playFile();
                  const start = () => {{
                    if (!(window.Hls && window.Hls.isSupported())) {{
                      return playFile();
                    }}
                    destroyHls();
                    el.removeAttribute('src');
                    try {{ el.load(); }} catch (e) {{}}
                    el.dataset.boundSrc = 'hls:' + hlsSrc;
                    const q = window.__pmvQuality || 'auto';
                    const hls = new window.Hls({{
                      enableWorker: true,
                      lowLatencyMode: false,
                      backBufferLength: 30,
                      maxBufferLength: 45,
                      maxMaxBufferLength: 60,
                      capLevelToPlayerSize: heightPref(q) < 0,
                      startLevel: -1,
                    }});
                    let fellBack = false;
                    const fallback = () => {{
                      if (fellBack) return;
                      fellBack = true;
                      destroyHls();
                      playFile();
                    }};
                    hls.loadSource(hlsSrc);
                    hls.attachMedia(el);
                    hls.on(window.Hls.Events.MANIFEST_PARSED, () => {{
                      applyQuality(hls, window.__pmvQuality || 'auto');
                      if (resume > 1) {{
                        try {{ el.currentTime = resume; }} catch (e) {{}}
                      }}
                      const p = el.play();
                      if (p && p.catch) p.catch(() => {{}});
                    }});
                    hls.on(window.Hls.Events.ERROR, (_e, data) => {{
                      if (data && data.fatal) fallback();
                    }});
                    setTimeout(() => {{
                      if (!fellBack && el.readyState < 2 && (!el.duration || isNaN(el.duration))) {{
                        fallback();
                      }}
                    }}, 8000);
                    window.__hls = hls;
                    return 'hls';
                  }};
                  if (window.Hls) return start();
                  let n = 0;
                  const t = setInterval(() => {{
                    n += 1;
                    if (window.Hls || n > 40) {{
                      clearInterval(t);
                      start();
                    }}
                  }}, 50);
                  return 'hls-wait';
                }};

                window.__pmvApplyQuality = (q) => {{
                  window.__pmvQuality = q || 'auto';
                  if (q === 'original') return playFile();
                  if (window.__hls && window.__hls.levels && window.__hls.levels.length) {{
                    applyQuality(window.__hls, q);
                    return 'level';
                  }}
                  return playHls();
                }};

                const qNow = window.__pmvQuality || 'auto';
                // Prefer progressive when present: WebKitGTK MSE/hls.js is often flaky,
                // and a stuck poster with controls=false looks like "no video element".
                const preferFile = (qNow === 'original') || !hlsSrc
                  || (!!fileSrc && (qNow === 'auto'));
                const bindKey = (preferFile ? 'file:' : 'hls:') + (preferFile ? fileSrc : hlsSrc);
                if (el.dataset.boundSrc === bindKey) {{
                  if (window.__hls) applyQuality(window.__hls, window.__pmvQuality || 'auto');
                  return 'same';
                }}

                if (preferFile) playFile();
                else playHls();

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

                return 'ok';
                "#
            ))
            .await;
        });
    });

    // Fullscreen chrome: show on mouse move, hide after 60s idle.
    use_effect(move || {
        let fs = player_fs();
        spawn(async move {
            let _ = document::eval(&format!(
                r#"
                window.__pmvFs = {fs};
                if (typeof window.__pmvFsIdleCleanup === 'function') {{
                  try {{ window.__pmvFsIdleCleanup(); }} catch (e) {{}}
                  window.__pmvFsIdleCleanup = null;
                }}
                document.querySelectorAll('.player-sidebar').forEach((el) => {{
                  el.classList.remove('fs-ui-visible');
                }});
                if (!{fs}) return 'fs-off';

                // Match typical video-control auto-hide (not a long idle).
                const IDLE_MS = 3500;
                let timer = null;
                const sidebar = () => document.querySelector('.player-sidebar.fullscreen');
                const show = () => {{
                  const el = sidebar();
                  if (!el) return;
                  el.classList.add('fs-ui-visible');
                  if (timer) clearTimeout(timer);
                  timer = setTimeout(() => {{
                    const cur = sidebar();
                    if (cur) cur.classList.remove('fs-ui-visible');
                  }}, IDLE_MS);
                }};
                const onMove = () => {{
                  if (window.__pmvFs) show();
                }};
                // Wait a frame so the .fullscreen class is on the DOM.
                requestAnimationFrame(() => {{
                  requestAnimationFrame(show);
                }});
                window.addEventListener('mousemove', onMove, true);
                window.addEventListener('pointermove', onMove, true);
                window.__pmvFsIdleCleanup = () => {{
                  window.removeEventListener('mousemove', onMove, true);
                  window.removeEventListener('pointermove', onMove, true);
                  if (timer) clearTimeout(timer);
                  timer = null;
                }};
                return 'fs-idle-on';
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
    let queue_count = queue_count();
    let queue_total = queue_total();

    rsx! {
        aside { class: if fs { "player-sidebar fullscreen" } else { "player-sidebar" },
            if !fs {
                div {
                    class: "player-rail-resize",
                    title: "Drag to resize player",
                    onpointerdown: move |evt| {
                        evt.prevent_default();
                        spawn(async move {
                            let _ = document::eval(RAIL_RESIZE_JS).await;
                        });
                    },
                }
            }
            if let Some(playable) = now_playing() {
                {
                    let id = playable.summary.id.clone();
                    let title = playable.summary.title.clone();
                    let has_file = !playable.video_url.is_empty();
                    let has_hls = playable
                        .hls_master_playlist_url
                        .as_ref()
                        .is_some_and(|u| !u.is_empty());
                    let has_url = has_file || has_hls;
                    let mut variants = playable.hls_variants.clone();
                    variants.sort_by(|a, b| b.height.cmp(&a.height));
                    let q_cur = player_quality();
                    let ar = if playable.summary.aspect_ratio > 0.0 {
                        playable.summary.aspect_ratio
                    } else {
                        16.0 / 9.0
                    };
                    let stage_class = if ar < 1.0 {
                        "player-stage is-portrait"
                    } else {
                        "player-stage"
                    };
                    let stage_style = format!("--player-ar: {ar}");

                    rsx! {
                        div { class: "player-upper",
                            div { class: "player-stage-host",
                                div { class: "{stage_class}", style: "{stage_style}",
                                    if !has_url {
                                        div { class: "player-error", "No stream URL for this video." }
                                    } else {
                                        video {
                                            key: "{id}",
                                            id: "pmv-player",
                                            class: "player-video",
                                            controls: false,
                                            playsinline: true,
                                            preload: "auto",
                                        }
                                    }
                                    if !fs {
                                        button {
                                            class: "player-fs-btn",
                                            title: "Fullscreen (double-click video)",
                                            onclick: move |_| {
                                                set_fullscreen_mode(!player_fs(), player_fs);
                                            },
                                            "⤢"
                                        }
                                    }
                                }
                            }
                            div { class: if fs { "player-meta player-fs-bar" } else { "player-meta" },
                                div { class: "player-title", "{title}" }
                                if has_hls || has_file {
                                    select {
                                        class: "player-quality",
                                        title: "Stream quality",
                                        value: "{q_cur}",
                                        onchange: move |e| {
                                            let q = e.value();
                                            player_quality.set(q.clone());
                                            spawn(async move {
                                                let _ = document::eval(&format!(
                                                    r#"
                                                    if (typeof window.__pmvApplyQuality === 'function') {{
                                                      window.__pmvApplyQuality({q:?});
                                                    }} else {{
                                                      window.__pmvQuality = {q:?};
                                                    }}
                                                    return 'ok';
                                                    "#
                                                ))
                                                .await;
                                            });
                                        },
                                        option { value: "auto", selected: q_cur == "auto", "Auto" }
                                        for v in variants {
                                            {
                                                let val = v.height.to_string();
                                                let label = if v.resolution.is_empty() {
                                                    format!("{}p", v.height)
                                                } else {
                                                    v.resolution.clone()
                                                };
                                                let selected = q_cur == val;
                                                rsx! {
                                                    option { value: "{val}", selected: selected, "{label}" }
                                                }
                                            }
                                        }
                                        if has_file {
                                            option {
                                                value: "original",
                                                selected: q_cur == "original",
                                                "Original"
                                            }
                                        }
                                    }
                                }
                                if fs {
                                    button {
                                        class: "icon-btn player-fs-details",
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
                                        "Details"
                                    }
                                    button {
                                        class: "icon-btn player-fs-exit",
                                        title: "Exit fullscreen (Esc)",
                                        onclick: move |_| {
                                            set_fullscreen_mode(false, player_fs);
                                        },
                                        "Exit"
                                    }
                                } else {
                                    button {
                                        class: "icon-btn",
                                        title: "Open details",
                                        onclick: {
                                            let id = id.clone();
                                            move |_| {
                                                navigator.push(Route::Watch { id: id.clone() });
                                            }
                                        },
                                        "↗"
                                    }
                                }
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
            if !fs {
                div {
                    class: "queue-resize",
                    title: "Drag to resize queue",
                    onpointerdown: move |evt| {
                        evt.prevent_default();
                        spawn(async move {
                            let _ = document::eval(QUEUE_RESIZE_JS).await;
                        });
                    },
                }
            }
            div {
                class: if fs { "sidebar-queue is-hidden" } else { "sidebar-queue" },
                div { class: "sidebar-queue-header",
                    div { class: "sidebar-queue-heading",
                        span { "Up next ({queue_count})" }
                        if queue_count > 0 {
                            span { class: "queue-total-time", title: "Total length of queued videos",
                                "{queue_total}"
                            }
                        }
                    }
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

const RAIL_RESIZE_JS: &str = r#"
(() => {
  const aside = document.querySelector('.player-sidebar:not(.fullscreen)');
  if (!aside) return 'no-aside';
  const send = (msg) => {
    if (typeof window.__pmvSend === 'function') window.__pmvSend(msg);
  };
  const minW = 280;
  const maxW = () => Math.max(minW + 40, Math.min(1400, Math.floor(window.innerWidth * 0.9)));
  const ensureUi = () => {
    let badge = document.getElementById('pmv-resize-badge');
    if (!badge) {
      badge = document.createElement('div');
      badge.id = 'pmv-resize-badge';
      badge.className = 'resize-badge';
      document.body.appendChild(badge);
    }
    return badge;
  };
  const applyW = (w, clientX, clientY) => {
    const px = w + 'px';
    document.documentElement.style.setProperty('--player-rail-w', px);
    // Inline during drag for immediate paint; cleared on pointerup / fullscreen.
    if (!aside.classList.contains('fullscreen')) {
      aside.style.width = px;
      aside.style.maxWidth = 'none';
    }
    window.__pmvRailLastW = w;
    const badge = ensureUi();
    badge.textContent = w + 'px wide';
    badge.style.display = 'block';
    badge.style.left = Math.min(window.innerWidth - 90, Math.max(8, clientX + 14)) + 'px';
    badge.style.top = Math.min(window.innerHeight - 36, Math.max(8, clientY - 12)) + 'px';
  };
  const onMove = (e) => {
    if (!window.__pmvRailBaseW) {
      window.__pmvRailBaseW = aside.getBoundingClientRect().width;
      window.__pmvRailBaseX = e.clientX;
    }
    const w = Math.round(
      Math.min(maxW(), Math.max(minW, window.__pmvRailBaseW + (window.__pmvRailBaseX - e.clientX)))
    );
    applyW(w, e.clientX, e.clientY);
  };
  const onUp = () => {
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('pointercancel', onUp);
    document.body.classList.remove('is-resizing-rail');
    const badge = document.getElementById('pmv-resize-badge');
    if (badge) badge.style.display = 'none';
    const w = window.__pmvRailLastW;
    // Drop inline width — persisted size lives in --player-rail-w (+ signal).
    aside.style.removeProperty('width');
    aside.style.removeProperty('max-width');
    window.__pmvRailBaseW = null;
    window.__pmvRailBaseX = null;
    if (w) send('rail|' + w);
  };
  document.body.classList.add('is-resizing-rail');
  window.__pmvRailBaseW = null;
  window.__pmvRailBaseX = null;
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
  window.addEventListener('pointercancel', onUp);
  return 'ok';
})()
"#;

const QUEUE_RESIZE_JS: &str = r#"
(() => {
  const aside = document.querySelector('.player-sidebar:not(.fullscreen)');
  const queue = aside && aside.querySelector('.sidebar-queue');
  if (!aside || !queue) return 'no-queue';
  const send = (msg) => {
    if (typeof window.__pmvSend === 'function') window.__pmvSend(msg);
  };
  const minH = 88;
  const maxH = () => {
    const asideH = aside.getBoundingClientRect().height;
    // Leave room for a usable video pane + meta (~160px).
    return Math.max(minH + 40, Math.floor(asideH - 160));
  };
  const ensureUi = () => {
    let badge = document.getElementById('pmv-resize-badge');
    if (!badge) {
      badge = document.createElement('div');
      badge.id = 'pmv-resize-badge';
      badge.className = 'resize-badge';
      document.body.appendChild(badge);
    }
    return badge;
  };
  const applyH = (h, clientX, clientY) => {
    const px = h + 'px';
    queue.style.flex = '0 0 ' + px;
    queue.style.height = px;
    queue.style.maxHeight = px;
    document.documentElement.style.setProperty('--player-queue-h', px);
    const main = document.getElementById('main');
    if (main) main.style.setProperty('--player-queue-h', px);
    if (aside) aside.style.setProperty('--player-queue-h', px);
    window.__pmvQueueLastH = h;
    const badge = ensureUi();
    badge.textContent = h + 'px queue';
    badge.style.display = 'block';
    badge.style.left = Math.min(window.innerWidth - 100, Math.max(8, clientX + 14)) + 'px';
    badge.style.top = Math.min(window.innerHeight - 36, Math.max(8, clientY - 12)) + 'px';
  };
  const onMove = (e) => {
    if (!window.__pmvQueueBaseH) {
      window.__pmvQueueBaseH = queue.getBoundingClientRect().height;
      window.__pmvQueueBaseY = e.clientY;
    }
    // Handle is the TOP edge of the queue: drag up → taller queue (divider follows mouse).
    const h = Math.round(
      Math.min(maxH(), Math.max(minH, window.__pmvQueueBaseH - (e.clientY - window.__pmvQueueBaseY)))
    );
    applyH(h, e.clientX, e.clientY);
  };
  const onUp = () => {
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('pointercancel', onUp);
    document.body.classList.remove('is-resizing-queue');
    const badge = document.getElementById('pmv-resize-badge');
    if (badge) badge.style.display = 'none';
    const h = window.__pmvQueueLastH;
    queue.style.removeProperty('flex');
    queue.style.removeProperty('height');
    queue.style.removeProperty('max-height');
    window.__pmvQueueBaseH = null;
    window.__pmvQueueBaseY = null;
    if (h) send('queueh|' + h);
  };
  document.body.classList.add('is-resizing-queue');
  window.__pmvQueueBaseH = null;
  window.__pmvQueueBaseY = null;
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
  window.addEventListener('pointercancel', onUp);
  return 'ok';
})()
"#;

async fn handle_player_msg(
    raw: &str,
    now_playing: Signal<Option<PlayableVideo>>,
    mut start_at: Signal<f64>,
    mut queue_tick: Signal<u32>,
    player_fs: Signal<bool>,
    mut player_rail_w: Signal<Option<u32>>,
    mut player_queue_h: Signal<Option<u32>>,
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
    if let Some(w) = raw.strip_prefix("rail|") {
        if let Ok(px) = w.parse::<u32>() {
            if (280..=1400).contains(&px) {
                player_rail_w.set(Some(px));
                crate::app::save_player_rail_width(px);
            }
        }
        return;
    }
    if let Some(h) = raw.strip_prefix("queueh|") {
        if let Ok(px) = h.parse::<u32>() {
            if (88..=1200).contains(&px) {
                player_queue_h.set(Some(px));
                crate::app::save_player_queue_height(px);
            }
        }
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

fn set_fullscreen_mode(on: bool, mut player_fs: Signal<bool>) {
    player_fs.set(on);
    // v1: in-flow fill + Tauri setFullscreen. Same here via wry/tao.
    // Native <video> fullscreen stays blocked (black on WebKitGTK).
    dioxus::desktop::window().set_fullscreen(on);
    // Strip any leftover inline geometry from rail/queue drag so FS can fill.
    spawn(async move {
        let _ = document::eval(&format!(
            r#"
            const aside = document.querySelector('.player-sidebar');
            if (aside) {{
              aside.style.removeProperty('width');
              aside.style.removeProperty('max-width');
              aside.style.removeProperty('min-width');
              aside.style.removeProperty('height');
              aside.style.removeProperty('max-height');
            }}
            const queue = document.querySelector('.sidebar-queue');
            if (queue && {on}) {{
              /* FS hides queue; leave saved size in CSS vars for exit. */
            }}
            if ({on}) {{
              document.documentElement.classList.add('pmv-player-fs');
            }} else {{
              document.documentElement.classList.remove('pmv-player-fs');
            }}
            return 'fs-layout';
            "#
        ))
        .await;
    });
}
