use crate::services::pmv::shared_client;
use crate::ui::pages::components::VideoGrid;
use dioxus::prelude::*;

const PAGE_SIZE: u32 = 32;

#[component]
pub fn Search(q: Option<String>) -> Element {
    let query = q.clone().unwrap_or_default();
    let mut items = use_signal(|| Vec::new());
    let mut page = use_signal(|| 1u32);
    let mut has_next = use_signal(|| false);
    let mut total = use_signal(|| 0u64);
    let mut loading = use_signal(|| true);
    let mut loading_more = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    // Re-run whenever the route `q` changes (nav search → same page).
    use_effect(move || {
        let q = q.clone().unwrap_or_default();
        spawn(async move {
            if q.trim().is_empty() {
                items.set(Vec::new());
                total.set(0);
                has_next.set(false);
                page.set(1);
                loading.set(false);
                err.set(None);
                return;
            }
            loading.set(true);
            err.set(None);
            let client = shared_client();
            match client.search(&q, 1, PAGE_SIZE).await {
                Ok(paged) => {
                    items.set(paged.items);
                    total.set(paged.pagination.total);
                    has_next.set(paged.pagination.has_next);
                    page.set(1);
                }
                Err(e) => {
                    items.set(Vec::new());
                    err.set(Some(e.to_string()));
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "page-header",
            h1 { "Search" }
            if query.trim().is_empty() {
                p { "Type a title, tag, or phrase in the search box." }
            } else {
                p {
                    "Results for \"{query}\""
                    if total() > 0 {
                        span { class: "muted", " · {total()} matches" }
                    }
                }
            }
        }
        if loading() {
            div { class: "loading", "Searching…" }
        } else if let Some(e) = err() {
            p { class: "error", "{e}" }
        } else if query.trim().is_empty() {
            p { class: "muted", "Enter a search above to find videos by title." }
        } else if items().is_empty() {
            p { class: "muted", "No results" }
        } else {
            VideoGrid { items: items() }
            if has_next() {
                div { style: "text-align:center; margin-top:1.5rem;",
                    button {
                        class: "btn btn-primary",
                        disabled: loading_more(),
                        onclick: {
                            let q = query.clone();
                            move |_| {
                                let q = q.clone();
                                let next = page() + 1;
                                spawn(async move {
                                    loading_more.set(true);
                                    let client = shared_client();
                                    if let Ok(paged) = client.search(&q, next, PAGE_SIZE).await {
                                        items.with_mut(|v| v.extend(paged.items));
                                        has_next.set(paged.pagination.has_next);
                                        page.set(next);
                                    }
                                    loading_more.set(false);
                                });
                            }
                        },
                        if loading_more() { "Loading…" } else { "Load more" }
                    }
                }
            }
        }
    }
}
