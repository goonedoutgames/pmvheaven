use crate::models::VideoSummary;
use crate::services::player::trim_front;
use crate::services::pmv::shared_client;
use crate::ui::pages::browse_filters::{BrowseFilterPanel, BrowseFilterState};
use crate::ui::pages::components::VideoGrid;
use dioxus::prelude::*;

/// Keep at most ~3 pages of cards mounted while browsing.
const MAX_BROWSE_ITEMS: usize = 96;

#[component]
pub fn Browse(sort: Option<String>, tags: Option<String>, creator: Option<String>) -> Element {
    let mut filters = use_signal(|| {
        BrowseFilterState::from_route(sort.as_deref(), tags.as_deref(), creator.as_deref())
    });
    let mut items = use_signal(|| Vec::<VideoSummary>::new());
    let mut page = use_signal(|| 1u32);
    let mut has_next = use_signal(|| false);
    let mut total = use_signal(|| 0u64);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload_tick = use_signal(|| 0u32);

    // Seed from route when navigating via tag/creator links.
    use_effect(move || {
        let next =
            BrowseFilterState::from_route(sort.as_deref(), tags.as_deref(), creator.as_deref());
        let cur = filters.peek().clone();
        if cur.tags != next.tags || cur.creator != next.creator || cur.sort != next.sort {
            filters.set(next);
            reload_tick.set(reload_tick() + 1);
        }
    });

    use_effect(move || {
        let _ = reload_tick();
        let feed = filters.peek().to_feed(1);
        spawn(async move {
            loading.set(true);
            let client = shared_client();
            match client.get_videos(feed).await {
                Ok(paged) => {
                    total.set(paged.pagination.total);
                    items.set(paged.items);
                    has_next.set(paged.pagination.has_next);
                    page.set(1);
                    err.set(None);
                }
                Err(e) => err.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    });

    let f = filters();
    let summary = {
        let mut parts = Vec::new();
        if !f.tags.trim().is_empty() {
            parts.push(format!("tags: {}", f.tags.trim()));
        }
        if !f.stars.trim().is_empty() {
            parts.push(format!("models: {}", f.stars.trim()));
        }
        if !f.creator.trim().is_empty() {
            parts.push(format!("creator: {}", f.creator.trim()));
        }
        if parts.is_empty() {
            "Explore the catalog".to_string()
        } else {
            parts.join(" · ")
        }
    };

    rsx! {
        div { class: "page-header",
            h1 { "Browse" }
            p { "{summary}"
                if total() > 0 {
                    span { class: "muted", " · {total()} results" }
                }
            }
        }

        BrowseFilterPanel {
            filters,
            on_apply: move |_| {
                reload_tick.set(reload_tick() + 1);
            },
        }

        if let Some(e) = err() {
            p { class: "error", "{e}" }
        }
        VideoGrid { items: items() }
        if loading() {
            div { class: "loading", "Loading…" }
        } else if has_next() {
            div { style: "text-align:center; margin-top:1.5rem;",
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let next = page() + 1;
                        let feed = filters.peek().to_feed(next);
                        spawn(async move {
                            loading.set(true);
                            let client = shared_client();
                            match client.get_videos(feed).await {
                                Ok(paged) => {
                                    let mut cur = items();
                                    cur.extend(paged.items);
                                    trim_front(&mut cur, MAX_BROWSE_ITEMS);
                                    items.set(cur);
                                    has_next.set(paged.pagination.has_next);
                                    total.set(paged.pagination.total);
                                    page.set(next);
                                }
                                Err(e) => err.set(Some(e.to_string())),
                            }
                            loading.set(false);
                        });
                    },
                    "Load more"
                }
            }
        }
    }
}
