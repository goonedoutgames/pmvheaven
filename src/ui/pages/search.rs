use crate::models::{FeedParams, VideoSummary};
use crate::services::pmv::{is_connected, shared_client};
use crate::ui::pages::components::VideoGrid;
use dioxus::prelude::*;

#[component]
pub fn Search(q: Option<String>) -> Element {
    let query = q.clone().unwrap_or_default();
    let mut items = use_signal(|| Vec::<VideoSummary>::new());
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let q_clone = query.clone();

    use_future(move || {
        let q = q_clone.clone();
        async move {
            if q.trim().is_empty() {
                loading.set(false);
                return;
            }
            loading.set(true);
            let client = shared_client();
            let result = if is_connected() {
                client.search(&q, 1, 32).await
            } else {
                client
                    .get_videos(FeedParams {
                        page: Some(1),
                        limit: Some(32),
                        tags: Some(q.clone()),
                        ..Default::default()
                    })
                    .await
            };
            match result {
                Ok(paged) => {
                    items.set(paged.items);
                    err.set(None);
                }
                Err(e) => match client
                    .get_videos(FeedParams {
                        page: Some(1),
                        limit: Some(32),
                        tags: Some(q),
                        ..Default::default()
                    })
                    .await
                {
                    Ok(paged) => {
                        items.set(paged.items);
                        err.set(None);
                    }
                    Err(_) => err.set(Some(e.to_string())),
                },
            }
            loading.set(false);
        }
    });

    rsx! {
        div { class: "page-header",
            h1 { "Search" }
            p { "Results for \"{query}\"" }
        }
        if loading() {
            div { class: "loading", "Searching…" }
        } else if let Some(e) = err() {
            p { class: "error", "{e}" }
        } else if items().is_empty() {
            p { class: "muted", "No results" }
        } else {
            VideoGrid { items: items() }
        }
    }
}
