use crate::app::{
    save_hover_preview_volume, save_hover_previews, save_pause_previews_while_playing,
};
use crate::models::AccountUser;
use crate::paths;
use crate::services::pmv::shared_client;
use crate::services::repo::history_count;
use crate::services::sync::{last_sync, sync_watch_history};
use crate::ui::ctx::{HoverPreviewVolume, HoverPreviews, PausePreviewsWhilePlaying};
use crate::ui::nav::Route;
use crate::ui::pages::components::refresh_watched_map;
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn Settings() -> Element {
    let mut user = use_context::<Signal<Option<AccountUser>>>();
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let mut hover_previews = use_context::<HoverPreviews>().0;
    let mut pause_previews_while_playing = use_context::<PausePreviewsWhilePlaying>().0;
    let mut hover_preview_volume = use_context::<HoverPreviewVolume>().0;
    let count = history_count();
    let last = last_sync();
    let mut msg = use_signal(|| None::<String>);
    let data_dir = paths::app_data_dir().display().to_string();
    let vol_pct = (hover_preview_volume() * 100.0).round() as u32;

    rsx! {
        div { class: "page-header",
            h1 { "Settings" }
            p { "Account, sync, and local data." }
        }
        section { class: "section",
            h2 { "Account" }
            if let Some(u) = user() {
                div { class: "settings-profile",
                    if let Some(url) = u.avatar_url.clone() {
                        img {
                            class: "settings-avatar",
                            src: "{url}",
                            alt: "",
                            referrerpolicy: "no-referrer",
                        }
                    } else {
                        {
                            let initial = u
                                .username
                                .chars()
                                .next()
                                .map(|c| c.to_uppercase().to_string())
                                .unwrap_or_else(|| "?".into());
                            rsx! {
                                span { class: "settings-avatar settings-avatar-fallback", "{initial}" }
                            }
                        }
                    }
                    div { class: "settings-profile-meta",
                        p { "Signed in as " strong { "{u.username}" } }
                        if let Some(e) = &u.email {
                            p { class: "muted", "{e}" }
                        }
                    }
                }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        spawn(async move {
                            shared_client().sign_out().await;
                            user.set(None);
                            msg.set(Some("Signed out".into()));
                        });
                    },
                    "Sign out"
                }
            } else {
                p { class: "muted", "Not signed in." }
                Link { to: Route::Login {}, class: "btn btn-primary", "Sign in" }
            }
        }
        section { class: "section",
            h2 { "Playback" }
            label { class: "filters-check",
                input {
                    r#type: "checkbox",
                    checked: hover_previews(),
                    onchange: move |_| {
                        let next = !hover_previews();
                        hover_previews.set(next);
                        save_hover_previews(next);
                    },
                }
                "Hover video previews on cards"
            }
            label { class: "filters-check", style: "margin-top:0.65rem;",
                input {
                    r#type: "checkbox",
                    checked: pause_previews_while_playing(),
                    disabled: !hover_previews(),
                    onchange: move |_| {
                        let next = !pause_previews_while_playing();
                        pause_previews_while_playing.set(next);
                        save_pause_previews_while_playing(next);
                    },
                }
                "Disable hover previews while a video is playing"
            }
            label {
                class: "filters-field",
                style: "margin-top:0.85rem; display:block;",
                span { style: "display:flex; justify-content:space-between; gap:0.75rem; margin-bottom:0.35rem; font-weight:600;",
                    span { "Hover preview volume" }
                    span { class: "muted", style: "font-weight:550;", "{vol_pct}%" }
                }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    step: "1",
                    value: "{vol_pct}",
                    disabled: !hover_previews(),
                    style: "width:min(320px, 100%);",
                    oninput: move |e| {
                        let pct: u32 = e.value().parse().unwrap_or(0).min(100);
                        let vol = pct as f32 / 100.0;
                        hover_preview_volume.set(vol);
                        save_hover_preview_volume(vol);
                    },
                }
            }
            p { class: "muted", style: "margin-top:0.5rem;",
                "0% keeps previews silent (default). Stream quality is on the player, per video."
            }
        }
        section { class: "section",
            h2 { "Local library" }
            p { "Permanent history entries: " strong { "{count}" } }
            if let Some((finished, new_count, status)) = last {
                p { class: "muted",
                    "Last sync: {status}, +{new_count} new"
                    if let Some(t) = finished {
                        {
                            let s = chrono::DateTime::from_timestamp_millis(t)
                                .map(|d| d.to_rfc3339())
                                .unwrap_or_default();
                            rsx! { " · {s}" }
                        }
                    }
                }
            }
            button {
                class: "btn btn-primary",
                onclick: move |_| {
                    spawn(async move {
                        msg.set(Some("Syncing…".into()));
                        let r = sync_watch_history().await;
                        refresh_watched_map(watched_map);
                        msg.set(Some(format!("{} (+{} new)", r.status, r.new_count)));
                    });
                },
                "Sync history now"
            }
            p { class: "muted", style: "margin-top:1rem;", "Data directory: {data_dir}" }
        }
        if let Some(m) = msg() {
            p { class: "muted", "{m}" }
        }
    }
}
