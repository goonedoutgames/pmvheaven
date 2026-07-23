use crate::models::{HistoryEntry, VideoDetail, VideoSummary};
use crate::services::pmv::shared_client;
use crate::services::queue;
use crate::services::repo::{
    Bucket, cache_video, set_local_bucket, upsert_history,
};
use crate::services::stream_proxy::proxied_url;
use crate::ui::nav::Route;
use crate::ui::pages::components::{refresh_watched_map, VideoGrid};
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn Watch(id: String) -> Element {
    let mut detail = use_signal(|| None::<VideoDetail>);
    let mut related = use_signal(|| Vec::<VideoSummary>::new());
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let proxy_base = use_context::<Signal<String>>();
    let mut queue_tick = use_context::<Signal<u32>>();
    let mut now_playing = use_context::<Signal<Option<VideoSummary>>>();
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let id_c = id.clone();

    use_future(move || {
        let id = id_c.clone();
        async move {
            loading.set(true);
            let client = shared_client();
            match client.get_video(&id).await {
                Ok(d) => {
                    cache_video(&d.summary);
                    now_playing.set(Some(d.summary.clone()));
                    // Record local watch start
                    upsert_history(
                        &HistoryEntry {
                            video: d.summary.clone(),
                            watched_at: chrono::Utc::now().to_rfc3339(),
                            progress: d.watch_progress.max(0.01),
                        },
                        "local",
                    );
                    refresh_watched_map(watched_map);
                    let client2 = client.clone();
                    let vid = id.clone();
                    spawn(async move {
                        client2.record_view(&vid).await;
                    });
                    let rel = client.get_related(&id).await.unwrap_or_default();
                    related.set(rel);
                    detail.set(Some(d));
                    err.set(None);
                }
                Err(e) => err.set(Some(e.to_string())),
            }
            loading.set(false);
        }
    });

    rsx! {
        if loading() {
            div { class: "loading", "Loading video…" }
        } else if let Some(e) = err() {
            p { class: "error", "{e}" }
        } else if let Some(d) = detail() {
            {
                let stream = if d.hls_enabled {
                    d.hls_master_playlist_url
                        .clone()
                        .filter(|u| !u.is_empty())
                        .unwrap_or_else(|| d.video_url.clone())
                } else {
                    d.video_url.clone()
                };
                let proxied = proxied_url(&proxy_base(), &stream);
                let mp4 = proxied_url(&proxy_base(), &d.video_url);
                let resume = d.watch_progress;
                let title = d.summary.title.clone();
                let summary = d.summary.clone();
                let is_fav = d.is_favorited;
                let is_later = d.is_watch_later;

                rsx! {
                    div { class: "watch-layout",
                        div { class: "watch-main",
                            VideoPlayer {
                                src: proxied,
                                mp4_fallback: mp4,
                                resume: resume,
                                title: title.clone(),
                                on_ended: move |_| {
                                    if let Some(next) = queue::shift() {
                                        queue_tick.set(queue_tick() + 1);
                                        now_playing.set(Some(next.clone()));
                                        // Navigation handled by parent via signal + effect ideally;
                                        // use navigator in handler:
                                    }
                                },
                                video_id: d.summary.id.clone(),
                            }
                            h1 { style: "margin: 1rem 0 0.35rem; font-size: 1.35rem;", "{d.summary.title}" }
                            p { class: "muted",
                                Link {
                                    to: Route::Browse {
                                        sort: Some("-uploadDate".into()),
                                        tags: None,
                                        creator: Some(d.summary.uploader_username.clone()),
                                    },
                                    "{d.summary.uploader_username}"
                                }
                                " · {crate::ui::pages::components::format_views(d.summary.views)}"
                            }
                            div { class: "card-actions", style: "margin-top: 0.75rem;",
                                button {
                                    class: "btn btn-ghost",
                                    onclick: {
                                        let v = summary.clone();
                                        move |_| {
                                            queue::add(v.clone());
                                            queue_tick.set(queue_tick() + 1);
                                        }
                                    },
                                    "Add to queue"
                                }
                                button {
                                    class: if is_fav { "btn btn-primary" } else { "btn btn-ghost" },
                                    onclick: {
                                        let v = summary.clone();
                                        let on = !is_fav;
                                        move |_| {
                                            set_local_bucket(Bucket::Favorites, &v, on);
                                            let client = shared_client();
                                            let id = v.id.clone();
                                            spawn(async move {
                                                let _ = client.set_favorite(&id, on).await;
                                            });
                                            detail.with_mut(|d| {
                                                if let Some(d) = d.as_mut() {
                                                    d.is_favorited = on;
                                                }
                                            });
                                        }
                                    },
                                    if is_fav { "Favorited" } else { "Favorite" }
                                }
                                button {
                                    class: if is_later { "btn btn-primary" } else { "btn btn-ghost" },
                                    onclick: {
                                        let v = summary.clone();
                                        let on = !is_later;
                                        move |_| {
                                            set_local_bucket(Bucket::WatchLater, &v, on);
                                            let client = shared_client();
                                            let id = v.id.clone();
                                            spawn(async move {
                                                let _ = client.set_watch_later(&id, on).await;
                                            });
                                            detail.with_mut(|d| {
                                                if let Some(d) = d.as_mut() {
                                                    d.is_watch_later = on;
                                                }
                                            });
                                        }
                                    },
                                    if is_later { "Saved" } else { "Watch later" }
                                }
                            }
                            if !d.description.is_empty() {
                                p { style: "margin-top: 1rem; line-height: 1.5; color: var(--muted);", "{d.description}" }
                            }
                            div { class: "tag-list",
                                for t in d.summary.tags.clone() {
                                    Link {
                                        to: Route::Browse {
                                            sort: Some("-views".into()),
                                            tags: Some(t.clone()),
                                            creator: None,
                                        },
                                        class: "chip",
                                        "{t}"
                                    }
                                }
                            }
                        }
                        aside {
                            h2 { style: "margin-top:0;", "Related" }
                            VideoGrid { items: related() }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn VideoPlayer(
    src: String,
    mp4_fallback: String,
    resume: f64,
    title: String,
    video_id: String,
    on_ended: EventHandler<()>,
) -> Element {
    let navigator = use_navigator();
    let mut queue_tick = use_context::<Signal<u32>>();

    use_effect(move || {
        let src = src.clone();
        let mp4 = mp4_fallback.clone();
        let resume = resume;
        let video_id = video_id.clone();
        spawn(async move {
            // Attach stream via eval — prefer progressive MP4 on WebKit, else hls.js
            let script = format!(
                r#"
                (function() {{
                  const video = document.getElementById('pmv-player');
                  if (!video) return 'no-video';
                  const src = {src:?};
                  const mp4 = {mp4:?};
                  const resume = {resume};
                  const isHls = src.includes('.m3u8');
                  const preferMp4 = true; // WebKitGTK: progressive is more reliable
                  function setProgressHandlers() {{
                    let last = 0;
                    video.ontimeupdate = () => {{
                      if (!video.duration || video.duration < 1) return;
                      const now = Date.now();
                      if (now - last < 15000) return;
                      last = now;
                      const p = video.currentTime / video.duration;
                      window.__pmvProgress = {{ id: {video_id:?}, progress: p }};
                    }};
                    video.onended = () => {{ window.__pmvEnded = true; }};
                  }}
                  function startAt() {{
                    if (resume > 0.01 && resume < 0.95) {{
                      const t = () => {{ video.currentTime = resume * (video.duration || 0); video.removeEventListener('loadedmetadata', t); }};
                      video.addEventListener('loadedmetadata', t);
                    }}
                    video.play().catch(() => {{}});
                  }}
                  if (preferMp4 && mp4) {{
                    if (window.__hls) {{ try {{ window.__hls.destroy(); }} catch(e) {{}} window.__hls = null; }}
                    video.src = mp4;
                    setProgressHandlers();
                    startAt();
                    return 'mp4';
                  }}
                  if (isHls && window.Hls && window.Hls.isSupported()) {{
                    if (window.__hls) {{ try {{ window.__hls.destroy(); }} catch(e) {{}} }}
                    const hls = new window.Hls();
                    window.__hls = hls;
                    hls.loadSource(src);
                    hls.attachMedia(video);
                    hls.on(window.Hls.Events.MANIFEST_PARSED, startAt);
                    hls.on(window.Hls.Events.ERROR, (_, data) => {{
                      if (data.fatal && mp4) {{
                        try {{ hls.destroy(); }} catch(e) {{}}
                        video.src = mp4;
                        startAt();
                      }}
                    }});
                    setProgressHandlers();
                    return 'hls';
                  }}
                  video.src = isHls ? mp4 || src : src;
                  setProgressHandlers();
                  startAt();
                  return 'native';
                }})()
                "#
            );
            let _ = document::eval(&script).await;
        });
    });

    // Poll for ended + progress
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Ok(v) = document::eval("return !!window.__pmvEnded").await {
                if v.as_bool() == Some(true) {
                    let _ = document::eval("window.__pmvEnded = false").await;
                    if let Some(next) = queue::shift() {
                        queue_tick.set(queue_tick() + 1);
                        navigator.push(Route::Watch { id: next.id });
                    }
                    on_ended.call(());
                }
            }
            if let Ok(v) = document::eval("return window.__pmvProgress || null").await {
                if !v.is_null() {
                    let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let progress = v.get("progress").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let _ = document::eval("window.__pmvProgress = null").await;
                    if let Some(summary) = crate::services::repo::get_cached_summary(&id) {
                        upsert_history(
                            &HistoryEntry {
                                video: summary,
                                watched_at: chrono::Utc::now().to_rfc3339(),
                                progress,
                            },
                            "local",
                        );
                    }
                }
            }
        }
    });

    rsx! {
        video {
            id: "pmv-player",
            controls: true,
            playsinline: true,
            style: "width:100%; max-height:70vh; background:#000; border-radius:12px;",
            "title": "{title}",
        }
    }
}
