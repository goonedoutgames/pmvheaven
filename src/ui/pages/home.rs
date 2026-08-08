use crate::models::{FeedParams, PopularTag, VideoSort, VideoSummary};
use crate::services::pmv::shared_client;
use crate::ui::nav::browse_link;
use crate::ui::pages::components::{format_compact, VideoGrid};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let mut trending = use_signal(|| Vec::<VideoSummary>::new());
    let mut top_rated = use_signal(|| Vec::<VideoSummary>::new());
    let mut newest = use_signal(|| Vec::<VideoSummary>::new());
    let mut tags = use_signal(|| Vec::<PopularTag>::new());
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);

    use_future(move || async move {
        loading.set(true);
        let client = shared_client();
        let t = client.get_trending().await;
        let rated = client
            .get_videos(FeedParams {
                page: Some(1),
                limit: Some(16),
                sort: Some(VideoSort::TopRated),
                ..Default::default()
            })
            .await;
        let fresh = client
            .get_videos(FeedParams {
                page: Some(1),
                limit: Some(16),
                sort: Some(VideoSort::Newest),
                ..Default::default()
            })
            .await;
        let tg = client.get_popular_tags().await;

        match (t, rated, fresh, tg) {
            (Ok(tr), Ok(r), Ok(n), Ok(tg)) => {
                trending.set(tr);
                top_rated.set(r.items);
                newest.set(n.items);
                tags.set(tg.into_iter().take(24).collect());
                err.set(None);
            }
            (e1, e2, e3, e4) => {
                let msg = [e1.err(), e2.err(), e3.err(), e4.err()]
                    .into_iter()
                    .flatten()
                    .map(|e| e.to_string())
                    .next()
                    .unwrap_or_else(|| "Failed to load".into());
                err.set(Some(msg));
            }
        }
        loading.set(false);
    });

    rsx! {
        div { class: "page-header",
            h1 { "Discover" }
            p { "Trending, top-rated, and newest from PMVHaven — ad-free." }
        }
        if loading() {
            div { class: "loading", "Loading…" }
        } else if let Some(e) = err() {
            p { class: "error", "{e}" }
        } else {
            if !tags().is_empty() {
                section { class: "section tags-section",
                    h2 { "Popular tags" }
                    div { class: "chip-row",
                        for t in tags() {
                            Link {
                                to: browse_link(
                                    "-releaseDate",
                                    Some(t.name.clone()),
                                    None,
                                    None,
                                    None,
                                    None,
                                ),
                                class: "chip",
                                span { class: "chip-label", "{t.name}" }
                                if t.usage_count > 0 {
                                    span { class: "chip-count", "{format_compact(t.usage_count)}" }
                                }
                            }
                        }
                    }
                }
            }
            section { class: "section",
                h2 { "Trending" }
                VideoGrid { items: trending() }
            }
            section { class: "section",
                h2 { "Top rated" }
                VideoGrid { items: top_rated() }
            }
            section { class: "section",
                h2 { "Newest" }
                VideoGrid { items: newest() }
            }
        }
    }
}
