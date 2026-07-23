use crate::models::{AccountUser, VideoSummary};
use crate::services::queue;
use crate::services::repo::watched_progress_map;
use crate::ui::nav::Route;
use dioxus::prelude::*;
use std::collections::HashMap;

/// Refresh the shared watched-progress map from SQLite (call after sync / local watch).
pub fn refresh_watched_map(mut map: Signal<HashMap<String, f64>>) {
    map.set(watched_progress_map());
}

fn rating_class(rating: f64) -> &'static str {
    if rating >= 80.0 {
        "rating high"
    } else if rating >= 65.0 {
        "rating mid"
    } else if rating > 0.0 {
        "rating low"
    } else {
        "rating"
    }
}

#[component]
pub fn VideoCard(video: VideoSummary) -> Element {
    let mut queue_tick = use_context::<Signal<u32>>();
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let user = use_context::<Signal<Option<AccountUser>>>();
    let mut hovering = use_signal(|| false);
    let id = video.id.clone();
    let queued = queue::is_queued(&id);
    let preview = video.preview_url.clone();
    let has_preview = preview.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let video_el_id = format!("preview-{}", video.id);
    let progress = watched_map().get(&id).copied();
    let signed_in = user().is_some();
    let is_watched = signed_in && progress.is_some();
    let rating = video.rating;

    rsx! {
        div {
            class: if is_watched { "video-card is-watched" } else { "video-card" },
            onmouseenter: {
                let el_id = video_el_id.clone();
                move |_| {
                    hovering.set(true);
                    if has_preview {
                        let id = el_id.clone();
                        spawn(async move {
                            let _ = document::eval(&format!(
                                r#"(function(){{
                                  const el = document.getElementById({id:?});
                                  if (!el) return;
                                  el.currentTime = 0;
                                  el.play().catch(()=>{{}});
                                }})()"#
                            )).await;
                        });
                    }
                }
            },
            onmouseleave: {
                let el_id = video_el_id.clone();
                move |_| {
                    hovering.set(false);
                    if has_preview {
                        let id = el_id.clone();
                        spawn(async move {
                            let _ = document::eval(&format!(
                                r#"(function(){{
                                  const el = document.getElementById({id:?});
                                  if (!el) return;
                                  el.pause();
                                }})()"#
                            )).await;
                        });
                    }
                }
            },
            Link { to: Route::Watch { id: video.id.clone() },
                div { class: "thumb-wrap",
                    if !video.thumbnail_url.is_empty() {
                        img {
                            class: if hovering() && has_preview { "thumb dim" } else { "thumb" },
                            src: "{video.thumbnail_url}",
                            alt: "{video.title}",
                            loading: "lazy",
                        }
                    }
                    if let Some(src) = preview.clone() {
                        if !src.is_empty() {
                            video {
                                id: "{video_el_id}",
                                class: if hovering() { "preview-video show" } else { "preview-video" },
                                src: "{src}",
                                muted: true,
                                r#loop: true,
                                playsinline: true,
                                preload: "none",
                            }
                        }
                    }
                    div { class: "thumb-badges",
                        if is_watched {
                            span { class: "badge watched",
                                span { class: "badge-check", "✓" }
                                "Watched"
                            }
                        }
                        if rating > 0.0 {
                            span { class: "{rating_class(rating)}",
                                "★ {rating.round() as i32}%"
                            }
                        }
                    }
                    if !video.duration.is_empty() {
                        span { class: "duration", "{video.duration}" }
                    }
                    if let Some(p) = progress {
                        if p > 0.0 {
                            span { class: "progress-track",
                                span {
                                    class: "progress-fill",
                                    style: "width: {((p * 100.0).clamp(0.0, 100.0))}%;",
                                }
                            }
                        }
                    }
                }
            }
            Link { to: Route::Watch { id: video.id.clone() },
                div { class: "card-title", "{video.title}" }
            }
            div { class: "card-meta",
                "{video.uploader_username} · {format_views(video.views)}"
            }
            div { class: "card-actions",
                button {
                    class: if queued { "icon-btn active" } else { "icon-btn" },
                    onclick: {
                        let v = video.clone();
                        move |_| {
                            queue::add(v.clone());
                            queue_tick.set(queue_tick() + 1);
                        }
                    },
                    if queued { "Queued" } else { "+ Queue" }
                }
                button {
                    class: "icon-btn",
                    onclick: {
                        let v = video.clone();
                        move |_| {
                            queue::play_next(v.clone());
                            queue_tick.set(queue_tick() + 1);
                        }
                    },
                    "Play next"
                }
            }
        }
    }
}

pub fn format_views(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M views", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K views", n as f64 / 1_000.0)
    } else {
        format!("{n} views")
    }
}

#[component]
pub fn VideoGrid(items: Vec<VideoSummary>) -> Element {
    rsx! {
        div { class: "grid",
            for v in items {
                VideoCard { video: v }
            }
        }
    }
}

#[component]
pub fn QueuePanel() -> Element {
    let open = use_context::<Signal<bool>>();
    let mut tick = use_context::<Signal<u32>>();
    let items = use_memo(move || {
        let _ = tick();
        queue::snapshot().items
    });
    let mut now_playing = use_context::<Signal<Option<VideoSummary>>>();
    let navigator = use_navigator();

    rsx! {
        aside { class: if open() { "queue-panel open" } else { "queue-panel" },
            div { class: "queue-header",
                h3 { style: "margin:0;", "Queue ({items().len()})" }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        queue::clear();
                        tick.set(tick() + 1);
                    },
                    "Clear"
                }
            }
            div { class: "queue-list",
                if items().is_empty() {
                    p { class: "muted", style: "padding:1rem;", "Queue is empty" }
                } else {
                    for (i, v) in items().into_iter().enumerate() {
                        div { class: "queue-item", key: "{v.id}-{i}",
                            img { src: "{v.thumbnail_url}", alt: "" }
                            div { class: "meta",
                                div { class: "t", "{v.title}" }
                                div { class: "card-meta",
                                    "{v.uploader_username}"
                                    if v.rating > 0.0 {
                                        " · ★ {v.rating.round() as i32}%"
                                    }
                                }
                            }
                            button {
                                class: "icon-btn",
                                onclick: {
                                    let id = v.id.clone();
                                    let vid = v.clone();
                                    move |_| {
                                        queue::remove(&id);
                                        now_playing.set(Some(vid.clone()));
                                        tick.set(tick() + 1);
                                        navigator.push(Route::Watch { id: id.clone() });
                                    }
                                },
                                "Play"
                            }
                            button {
                                class: "icon-btn",
                                onclick: {
                                    let id = v.id.clone();
                                    move |_| {
                                        queue::remove(&id);
                                        tick.set(tick() + 1);
                                    }
                                },
                                "✕"
                            }
                            if i > 0 {
                                button {
                                    class: "icon-btn",
                                    onclick: move |_| {
                                        queue::move_item(i, i - 1);
                                        tick.set(tick() + 1);
                                    },
                                    "↑"
                                }
                            }
                            button {
                                class: "icon-btn",
                                onclick: move |_| {
                                    queue::move_item(i, i + 1);
                                    tick.set(tick() + 1);
                                },
                                "↓"
                            }
                        }
                    }
                }
            }
        }
    }
}
