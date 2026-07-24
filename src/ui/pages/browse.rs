use crate::models::{FeedParams, VideoSort, VideoSummary};
use crate::services::player::trim_front;
use crate::services::pmv::shared_client;
use crate::ui::pages::components::VideoGrid;
use dioxus::prelude::*;

/// Keep at most ~3 pages of cards mounted while browsing.
const MAX_BROWSE_ITEMS: usize = 96;

fn parse_sort(s: &str) -> VideoSort {
    match s {
        "uploadDate" => VideoSort::Oldest,
        "-views" => VideoSort::MostViews,
        "views" => VideoSort::LeastViews,
        "-likes" => VideoSort::MostLiked,
        "-bayesianRating" => VideoSort::TopRated,
        _ => VideoSort::Newest,
    }
}

#[component]
pub fn Browse(sort: Option<String>, tags: Option<String>, creator: Option<String>) -> Element {
    let current_sort = parse_sort(sort.as_deref().unwrap_or("-uploadDate"));
    let mut items = use_signal(|| Vec::<VideoSummary>::new());
    let mut page = use_signal(|| 1u32);
    let mut has_next = use_signal(|| false);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let tags_sig = use_signal(|| tags.clone());
    let creator_sig = use_signal(|| creator.clone());
    let sort_sig = use_signal(|| current_sort);

    use_future(move || async move {
        loading.set(true);
        let client = shared_client();
        match client
            .get_videos(FeedParams {
                page: Some(1),
                limit: Some(32),
                sort: Some(sort_sig()),
                tags: tags_sig(),
                creator: creator_sig(),
                ..Default::default()
            })
            .await
        {
            Ok(paged) => {
                items.set(paged.items);
                has_next.set(paged.pagination.has_next);
                page.set(1);
                err.set(None);
            }
            Err(e) => err.set(Some(e.to_string())),
        }
        loading.set(false);
    });

    let sorts = [
        VideoSort::Newest,
        VideoSort::MostViews,
        VideoSort::TopRated,
        VideoSort::MostLiked,
    ];

    rsx! {
        div { class: "page-header",
            h1 { "Browse" }
            p {
                if let Some(t) = &tags { "Tag: {t}" }
                else if let Some(c) = &creator { "Creator: {c}" }
                else { "Explore the catalog" }
            }
        }
        div { class: "tabs",
            for s in sorts {
                Link {
                    to: crate::ui::nav::Route::Browse {
                        sort: Some(s.as_api().to_string()),
                        tags: tags.clone(),
                        creator: creator.clone(),
                    },
                    class: if current_sort == s { "tab active" } else { "tab" },
                    "{s.label()}"
                }
            }
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
                        let tags = tags_sig();
                        let creator = creator_sig();
                        let sort = sort_sig();
                        spawn(async move {
                            loading.set(true);
                            let client = shared_client();
                            match client
                                .get_videos(FeedParams {
                                    page: Some(next),
                                    limit: Some(32),
                                    sort: Some(sort),
                                    tags,
                                    creator,
                                    ..Default::default()
                                })
                                .await
                            {
                                Ok(paged) => {
                                    let mut cur = items();
                                    cur.extend(paged.items);
                                    trim_front(&mut cur, MAX_BROWSE_ITEMS);
                                    items.set(cur);
                                    has_next.set(paged.pagination.has_next);
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
