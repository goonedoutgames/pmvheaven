use crate::models::AccountUser;
use crate::paths;
use crate::services::pmv::shared_client;
use crate::services::repo::history_count;
use crate::services::sync::{last_sync, sync_watch_history};
use crate::ui::nav::Route;
use crate::ui::pages::components::refresh_watched_map;
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn Settings() -> Element {
    let mut user = use_context::<Signal<Option<AccountUser>>>();
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let count = history_count();
    let last = last_sync();
    let mut msg = use_signal(|| None::<String>);
    let data_dir = paths::app_data_dir().display().to_string();

    rsx! {
        div { class: "page-header",
            h1 { "Settings" }
            p { "Account, sync, and local data." }
        }
        section { class: "section",
            h2 { "Account" }
            if let Some(u) = user() {
                p { "Signed in as " strong { "{u.username}" } }
                if let Some(e) = &u.email {
                    p { class: "muted", "{e}" }
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
