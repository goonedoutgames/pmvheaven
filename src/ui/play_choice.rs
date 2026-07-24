use crate::models::{PlayableVideo, VideoSummary};
use crate::services::player;
use crate::services::queue;
use dioxus::prelude::*;

/// Modal when the user opens a video while another is already playing.
#[component]
pub fn PlayChoiceModal() -> Element {
    let mut pending = use_context::<Signal<Option<VideoSummary>>>();
    let now_playing = use_context::<Signal<Option<PlayableVideo>>>();
    let start_at = use_context::<Signal<f64>>();
    let mut queue_tick = use_context::<Signal<u32>>();
    let navigator = use_navigator();

    let Some(video) = pending() else {
        return rsx! {};
    };

    let current_title = now_playing()
        .map(|p| p.summary.title)
        .unwrap_or_else(|| "current video".into());

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal",
                h3 { style: "margin:0 0 0.5rem;", "Something is already playing" }
                p { class: "muted", style: "margin:0 0 1rem;",
                    "\"{current_title}\" is in the player. What should we do with \"{video.title}\"?"
                }
                div { class: "modal-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: {
                            let v = video.clone();
                            move |_| {
                                pending.set(None);
                                let id = v.id.clone();
                                spawn(async move {
                                    let _ = player::play_id(&id, now_playing, start_at).await;
                                });
                                navigator.push(crate::ui::nav::Route::Watch { id: v.id.clone() });
                            }
                        },
                        "Play now"
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: {
                            let v = video.clone();
                            move |_| {
                                queue::add(v.clone());
                                queue_tick.set(queue_tick() + 1);
                                pending.set(None);
                            }
                        },
                        "Add to queue"
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: {
                            let v = video.clone();
                            move |_| {
                                queue::play_next(v.clone());
                                queue_tick.set(queue_tick() + 1);
                                pending.set(None);
                            }
                        },
                        "Play next"
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| pending.set(None),
                        "Cancel"
                    }
                }
            }
        }
    }
}
