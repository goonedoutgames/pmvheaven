use crate::models::{AccountUser, PlayableVideo, VideoSummary};
use crate::services::player::{self, OpenIntent};
use crate::services::queue;
use crate::services::repo::watched_progress_map;
use crate::services::stream_proxy::proxied_url;
use crate::ui::ctx::{
    HoverPreviewVolume, HoverPreviews, PausePreviewsWhilePlaying, ProxyBase, QueueTick,
};
use crate::ui::nav::{browse_link, Route};
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
    let mut queue_tick = use_context::<QueueTick>().0;
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let user = use_context::<Signal<Option<AccountUser>>>();
    let now_playing = use_context::<Signal<Option<PlayableVideo>>>();
    let mut play_choice = use_context::<Signal<Option<VideoSummary>>>();
    let hover_previews = use_context::<HoverPreviews>().0;
    let pause_previews_while_playing = use_context::<PausePreviewsWhilePlaying>().0;
    let hover_preview_volume = use_context::<HoverPreviewVolume>().0;
    let proxy_base = use_context::<ProxyBase>().0;
    let navigator = use_navigator();
    let mut hovering = use_signal(|| false);
    let id = video.id.clone();
    let queued = {
        let _ = queue_tick();
        queue::is_queued(&id)
    };
    let preview = video.preview_url.clone().map(|u| {
        let base = proxy_base();
        let proxied = proxied_url(&base, &u);
        if proxied.is_empty() {
            u
        } else {
            proxied
        }
    });
    let has_preview = preview.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let video_el_id = format!("preview-{}", video.id);
    let progress = watched_map().get(&id).copied();
    let signed_in = user().is_some();
    let is_watched = signed_in && progress.is_some();
    let rating = video.rating;
    let playing = now_playing().is_some();
    let allow_preview = hover_previews()
        && has_preview
        && !(pause_previews_while_playing() && playing);
    let preview_vol = hover_preview_volume().clamp(0.0, 1.0);
    let preview_muted = preview_vol <= 0.001;

    let open_from_thumb = {
        let video = video.clone();
        move |_| {
            match player::open_intent(&video, now_playing) {
                OpenIntent::Play | OpenIntent::AlreadyPlaying => {
                    navigator.push(Route::Watch { id: video.id.clone() });
                }
                OpenIntent::Choice(v) => {
                    play_choice.set(Some(v));
                }
            }
        }
    };
    let open_from_title = {
        let video = video.clone();
        move |_| {
            match player::open_intent(&video, now_playing) {
                OpenIntent::Play | OpenIntent::AlreadyPlaying => {
                    navigator.push(Route::Watch { id: video.id.clone() });
                }
                OpenIntent::Choice(v) => {
                    play_choice.set(Some(v));
                }
            }
        }
    };

    rsx! {
        div {
            class: if is_watched { "video-card is-watched" } else { "video-card" },
            onmouseenter: move |_| {
                if allow_preview {
                    hovering.set(true);
                }
            },
            onmouseleave: move |_| {
                hovering.set(false);
            },
            div {
                class: "thumb-wrap",
                onclick: open_from_thumb,
                if !video.thumbnail_url.is_empty() {
                    img {
                        class: if hovering() && allow_preview { "thumb dim" } else { "thumb" },
                        src: "{video.thumbnail_url}",
                        alt: "{video.title}",
                        loading: "lazy",
                        decoding: "async",
                    }
                }
                if hovering() && allow_preview {
                    if let Some(src) = preview.clone() {
                        if !src.is_empty() {
                            video {
                                id: "{video_el_id}",
                                class: "preview-video show",
                                src: "{src}",
                                muted: preview_muted,
                                autoplay: true,
                                r#loop: true,
                                playsinline: true,
                                preload: "metadata",
                                onmounted: {
                                    let el_id = video_el_id.clone();
                                    move |_| {
                                        let el_id = el_id.clone();
                                        let vol = preview_vol;
                                        let muted = preview_muted;
                                        spawn(async move {
                                            let _ = document::eval(&format!(
                                                r#"
                                                const el = document.getElementById({el_id:?});
                                                if (!el) return 'no-el';
                                                try {{
                                                  el.muted = {muted};
                                                  el.volume = {vol};
                                                }} catch (e) {{}}
                                                return 'ok';
                                                "#
                                            ))
                                            .await;
                                        });
                                    }
                                },
                            }
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
            div {
                class: "card-title",
                onclick: open_from_title,
                "{video.title}"
            }
            div { class: "card-meta",
                Link {
                    to: browse_link(
                        "-releaseDate",
                        None,
                        None,
                        Some(if video.uploader_username.is_empty() {
                            video.uploader.clone()
                        } else {
                            video.uploader_username.clone()
                        }),
                        None,
                        None,
                    ),
                    class: "uploader-link",
                    "{video.uploader_username}"
                }
                " · {format_views(video.views)}"
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

/// Compact count for chips / badges (no unit suffix).
pub fn format_compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{n}")
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
