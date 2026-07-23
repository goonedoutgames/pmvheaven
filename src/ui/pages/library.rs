use crate::services::repo::{Bucket, get_bucket};
use crate::ui::pages::components::VideoGrid;
use dioxus::prelude::*;

#[component]
pub fn Favorites() -> Element {
    let items = use_signal(|| get_bucket(Bucket::Favorites));
    rsx! {
        div { class: "page-header",
            h1 { "Favorites" }
            p { "Mirrored locally and synced to PMVHaven when signed in." }
        }
        if items().is_empty() {
            p { class: "muted", "No favorites yet." }
        } else {
            VideoGrid { items: items() }
        }
    }
}

#[component]
pub fn WatchLater() -> Element {
    let items = use_signal(|| get_bucket(Bucket::WatchLater));
    rsx! {
        div { class: "page-header",
            h1 { "Watch later" }
            p { "Your saved list, kept locally." }
        }
        if items().is_empty() {
            p { class: "muted", "Nothing saved for later." }
        } else {
            VideoGrid { items: items() }
        }
    }
}
