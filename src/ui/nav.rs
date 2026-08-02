use crate::models::{AccountUser, PlayableVideo};
use crate::services::queue;
use crate::ui::chrome::LOGO;
use crate::ui::play_choice::PlayChoiceModal;
use crate::ui::player_sidebar::PlayerSidebar;
use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
    #[route("/")]
    Home {},
    #[route("/browse?:sort&:tags&:creator")]
    Browse { sort: Option<String>, tags: Option<String>, creator: Option<String> },
    #[route("/search?:q")]
    Search { q: Option<String> },
    #[route("/watch/:id")]
    Watch { id: String },
    #[route("/history")]
    History {},
    #[route("/favorites")]
    Favorites {},
    #[route("/watch-later")]
    WatchLater {},
    #[route("/settings")]
    Settings {},
    #[route("/login")]
    Login {},
}

#[component]
fn AppLayout() -> Element {
    let user = use_context::<Signal<Option<AccountUser>>>();
    let mut queue_open = use_context::<Signal<bool>>();
    let queue_tick = use_context::<Signal<u32>>();
    let now_playing = use_context::<Signal<Option<PlayableVideo>>>();
    let player_fs = use_context::<Signal<bool>>();
    let mut search_q = use_signal(|| String::new());
    let navigator = use_navigator();

    let queue_len = use_memo(move || {
        let _ = queue_tick();
        queue::len()
    });

    let show_sidebar = now_playing().is_some() || queue_len() > 0 || queue_open();
    let shell_class = if player_fs() {
        "app-shell has-sidebar player-fs"
    } else if show_sidebar {
        "app-shell has-sidebar"
    } else {
        "app-shell"
    };

    rsx! {
        div { class: "{shell_class}",
            div { class: "app-main",
                nav { class: "navbar",
                    Link { to: Route::Home {}, class: "nav-brand",
                        img { src: LOGO, alt: "PMVHeaven" }
                        span { class: "nav-brand-text", "PMVHeaven" }
                    }
                    div { class: "nav-links",
                        Link { to: Route::Home {}, class: "nav-link", "Home" }
                        Link { to: Route::Browse { sort: Some("-releaseDate".into()), tags: None, creator: None }, class: "nav-link", "Browse" }
                        Link { to: Route::History {}, class: "nav-link nav-hide-sm", "History" }
                        Link { to: Route::Favorites {}, class: "nav-link nav-hide-sm", "Favorites" }
                        Link { to: Route::WatchLater {}, class: "nav-link nav-hide-md", "Later" }
                    }
                    form {
                        class: "nav-search",
                        onsubmit: move |e| {
                            e.prevent_default();
                            let q = search_q();
                            if !q.trim().is_empty() {
                                navigator.push(Route::Search { q: Some(q) });
                            }
                        },
                        input {
                            r#type: "search",
                            placeholder: "Search…",
                            value: "{search_q}",
                            oninput: move |e| search_q.set(e.value()),
                            style: "width: 100%;",
                        }
                    }
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            queue_open.set(true);
                        },
                        "Queue ({queue_len()})"
                    }
                    div { class: "nav-user",
                        if let Some(u) = user() {
                            Link { to: Route::Settings {}, class: "nav-user-name", "{u.username}" }
                        } else {
                            Link { to: Route::Login {}, class: "btn btn-primary", "Sign in" }
                        }
                    }
                }
                div { class: "content",
                    Outlet::<Route> {}
                }
            }
            PlayerSidebar {}
        }
        PlayChoiceModal {}
    }
}

pub use crate::ui::pages::*;
