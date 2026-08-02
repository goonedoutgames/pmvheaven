use crate::models::HistoryEntry;
use crate::services::player::trim_front;
use crate::services::repo::get_history_page;
use crate::services::sync::{is_syncing, push_local_history, sync_progress, sync_watch_history};
use crate::ui::pages::components::{refresh_watched_map, VideoCard};
use dioxus::prelude::*;
use std::collections::HashMap;

const MAX_HISTORY_ITEMS: usize = 120;

#[component]
pub fn History() -> Element {
    let mut items = use_signal(|| Vec::<HistoryEntry>::new());
    let mut total = use_signal(|| 0u64);
    let mut page = use_signal(|| 1u32);
    let mut syncing = use_signal(|| false);
    let mut sync_msg = use_signal(|| None::<String>);
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();

    use_effect(move || {
        let (list, t) = get_history_page(1, 60);
        items.set(list);
        total.set(t);
        page.set(1);
    });

    rsx! {
        div { class: "page-header",
            h1 { "Watch history" }
            p { "{total()} videos saved permanently in local SQLite" }
        }
        div { class: "history-actions",
            button {
                class: "btn btn-primary",
                disabled: syncing(),
                onclick: move |_| {
                    syncing.set(true);
                    sync_msg.set(Some("Starting sync…".into()));
                    spawn(async move {
                        let result = sync_watch_history().await;
                        sync_msg.set(Some(format!(
                            "Pull {} — +{} new, {} seen",
                            result.status, result.new_count, result.seen_count
                        )));
                        syncing.set(false);
                        let (list, t) = get_history_page(1, 60);
                        items.set(list);
                        total.set(t);
                        page.set(1);
                        refresh_watched_map(watched_map);
                    });
                    spawn(async move {
                        while is_syncing() {
                            if let Some(p) = sync_progress() {
                                sync_msg.set(Some(format!(
                                    "{} ({}/{}) {}",
                                    p.phase,
                                    p.processed,
                                    p.total,
                                    p.message.unwrap_or_default()
                                )));
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        }
                    });
                },
                if syncing() { "Syncing…" } else { "Sync from PMVHaven" }
            }
            button {
                class: "btn btn-ghost",
                disabled: syncing(),
                title: "Upload local watch history to your PMVHaven account",
                onclick: move |_| {
                    syncing.set(true);
                    sync_msg.set(Some("Pushing local history…".into()));
                    spawn(async move {
                        let result = push_local_history().await;
                        sync_msg.set(result.message.or_else(|| {
                            Some(format!(
                                "Push {} — {} uploaded",
                                result.status, result.new_count
                            ))
                        }));
                        syncing.set(false);
                    });
                    spawn(async move {
                        while is_syncing() {
                            if let Some(p) = sync_progress() {
                                sync_msg.set(Some(format!(
                                    "{} ({}/{}) {}",
                                    p.phase,
                                    p.processed,
                                    p.total,
                                    p.message.unwrap_or_default()
                                )));
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        }
                    });
                },
                "Push to PMVHaven"
            }
            if let Some(m) = sync_msg() {
                span { class: "muted", "{m}" }
            }
        }
        div { class: "grid",
            for e in items() {
                VideoCard { video: e.video }
            }
        }
        if items().len() < total() as usize {
            div { style: "text-align:center; margin-top:1.5rem;",
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        let p = page() + 1;
                        let (more, t) = get_history_page(p, 60);
                        let mut cur = items();
                        cur.extend(more);
                        trim_front(&mut cur, MAX_HISTORY_ITEMS);
                        items.set(cur);
                        total.set(t);
                        page.set(p);
                    },
                    "Load more"
                }
            }
        }
        if total() == 0 {
            p { class: "muted", "No history yet. Sign in and sync, or watch something." }
        }
    }
}
