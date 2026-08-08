use crate::models::{PlayableVideo, VideoDetail, VideoSummary};
use crate::services::player;
use crate::services::pmv::shared_client;
use crate::services::queue;
use crate::services::repo::{Bucket, set_local_bucket};
use crate::ui::ctx::{QueueTick, StartAt};
use crate::ui::nav::browse_link;
use crate::ui::pages::components::{format_views, VideoGrid};
use dioxus::prelude::*;

#[component]
pub fn Watch(id: String) -> Element {
    let mut detail = use_signal(|| None::<VideoDetail>);
    let mut related = use_signal(|| Vec::<VideoSummary>::new());
    let mut related_done = use_signal(|| false);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let now_playing = use_context::<Signal<Option<PlayableVideo>>>();
    let start_at = use_context::<StartAt>().0;
    let mut queue_tick = use_context::<QueueTick>().0;
    let id_c = id.clone();

    use_future(move || {
        let id = id_c.clone();
        async move {
            loading.set(true);
            related_done.set(false);
            let client = shared_client();
            match client.get_video(&id).await {
                Ok(d) => {
                    // Kick the rail immediately — don't wait on related.
                    player::play_detail(now_playing, start_at, queue_tick, &d);
                    let client2 = client.clone();
                    let vid = id.clone();
                    spawn(async move {
                        client2.record_view(&vid).await;
                    });
                    detail.set(Some(d));
                    err.set(None);
                    loading.set(false);
                    spawn(async move {
                        let rel = client.get_related(&id).await.unwrap_or_default();
                        related.set(rel);
                        related_done.set(true);
                    });
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        }
    });

    let is_current = now_playing()
        .as_ref()
        .is_some_and(|p| p.summary.id == id);

    rsx! {
        if loading() {
            div { class: "loading", "Loading video…" }
        } else if let Some(e) = err() {
            p { class: "error", "{e}" }
        } else if let Some(d) = detail() {
            {
                let summary = d.summary.clone();
                let is_fav = d.is_favorited;
                let is_later = d.is_watch_later;
                let uploader_name = d.summary.uploader_username.clone();
                let uploader_key = if !d.summary.uploader_username.is_empty() {
                    d.summary.uploader_username.clone()
                } else {
                    d.summary.uploader.clone()
                };
                rsx! {
                    div { class: "watch-layout",
                        div { class: "watch-main",
                            div { class: "watch-poster",
                                if !d.summary.thumbnail_url.is_empty() {
                                    img {
                                        src: "{d.summary.thumbnail_url}",
                                        alt: "{d.summary.title}",
                                    }
                                }
                                div { class: "watch-poster-overlay",
                                    span { class: "watch-poster-badge",
                                        if is_current { "Playing in the player panel" }
                                        else { "Starting…" }
                                    }
                                }
                            }
                            h1 { style: "margin: 1rem 0 0.35rem; font-size: 1.35rem;", "{d.summary.title}" }
                            p { class: "muted",
                                Link {
                                    to: browse_link(
                                        "-releaseDate",
                                        None,
                                        None,
                                        Some(uploader_key),
                                        None,
                                        None,
                                    ),
                                    class: "uploader-link",
                                    "{uploader_name}"
                                }
                                " · {format_views(d.summary.views)}"
                                if d.summary.rating > 0.0 {
                                    " · ★ {d.summary.rating.round() as i32}%"
                                }
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
                            if !d.summary.tags.is_empty() {
                                div { class: "discover-block",
                                    span { class: "discover-label", "Tags" }
                                    div { class: "tag-list",
                                        for t in d.summary.tags.clone() {
                                            Link {
                                                to: browse_link(
                                                    "-views",
                                                    Some(t.clone()),
                                                    None,
                                                    None,
                                                    None,
                                                    None,
                                                ),
                                                class: "chip",
                                                "{t}"
                                            }
                                        }
                                    }
                                }
                            }
                            if !d.stars.is_empty() {
                                div { class: "discover-block",
                                    span { class: "discover-label", "Models" }
                                    div { class: "tag-list",
                                        for s in d.stars.clone() {
                                            Link {
                                                to: browse_link(
                                                    "-views",
                                                    None,
                                                    None,
                                                    None,
                                                    Some(s.clone()),
                                                    None,
                                                ),
                                                class: "chip",
                                                "{s}"
                                            }
                                        }
                                    }
                                }
                            }
                            if !d.creator.is_empty() {
                                div { class: "discover-block",
                                    span { class: "discover-label", "Creators" }
                                    div { class: "tag-list",
                                        for c in d.creator.clone() {
                                            Link {
                                                to: browse_link(
                                                    "-views",
                                                    None,
                                                    Some(c.clone()),
                                                    None,
                                                    None,
                                                    None,
                                                ),
                                                class: "chip",
                                                "{c}"
                                            }
                                        }
                                    }
                                }
                            }
                            if !d.music.is_empty() {
                                div { class: "discover-block",
                                    span { class: "discover-label", "Music" }
                                    div { class: "tag-list",
                                        for m in d.music.clone() {
                                            {
                                                let artist = m.artist.clone();
                                                let song = m.song.clone();
                                                let label = if artist.is_empty() {
                                                    song.clone()
                                                } else if song.is_empty() {
                                                    artist.clone()
                                                } else {
                                                    format!("{artist} — {song}")
                                                };
                                                let music_q = if !artist.is_empty() && !song.is_empty() {
                                                    format!("{artist} - {song}")
                                                } else if !artist.is_empty() {
                                                    artist.clone()
                                                } else {
                                                    song.clone()
                                                };
                                                rsx! {
                                                    Link {
                                                        to: browse_link(
                                                            "-views",
                                                            None,
                                                            None,
                                                            None,
                                                            None,
                                                            Some(music_q),
                                                        ),
                                                        class: "chip",
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        aside {
                            h2 { style: "margin-top:0;", "Related" }
                            if !related_done() {
                                p { class: "muted", "Loading related…" }
                            } else if related().is_empty() {
                                p { class: "muted", "No related videos" }
                            } else {
                                VideoGrid { items: related() }
                            }
                        }
                    }
                }
            }
        }
    }
}
