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
    #[route("/browse?:sort&:tags&:creator&:uploader&:stars&:music")]
    Browse {
        sort: Option<String>,
        tags: Option<String>,
        creator: Option<String>,
        uploader: Option<String>,
        stars: Option<String>,
        music: Option<String>,
    },
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

/// Convenience for deep-links into browse filters.
pub fn browse_link(
    sort: &str,
    tags: Option<String>,
    creator: Option<String>,
    uploader: Option<String>,
    stars: Option<String>,
    music: Option<String>,
) -> Route {
    Route::Browse {
        sort: Some(sort.into()),
        tags,
        creator,
        uploader,
        stars,
        music,
    }
}

#[component]
fn AppLayout() -> Element {
    let user = use_context::<Signal<Option<AccountUser>>>();
    let mut queue_open = use_context::<Signal<bool>>();
    let queue_tick = use_context::<Signal<u32>>();
    let now_playing = use_context::<Signal<Option<PlayableVideo>>>();
    let player_fs = use_context::<Signal<bool>>();
    let mut search_q = use_signal(|| String::new());
    let mut menu_open = use_signal(|| false);
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

    let close_menu = move |_| menu_open.set(false);

    rsx! {
        div { class: "{shell_class}",
            div { class: "app-main",
                nav { class: "navbar",
                    div { class: "nav-left",
                        Link {
                            to: Route::Home {},
                            class: "nav-brand",
                            onclick: close_menu,
                            img { src: LOGO, alt: "PMVHeaven" }
                            span { class: "nav-brand-text", "PMVHeaven" }
                        }
                        div { class: "nav-links nav-links-primary",
                            Link {
                                to: Route::Home {},
                                class: "nav-link",
                                onclick: close_menu,
                                "Home"
                            }
                            Link {
                                to: browse_link("-releaseDate", None, None, None, None, None),
                                class: "nav-link",
                                onclick: close_menu,
                                "Browse"
                            }
                        }
                        div { class: "nav-links nav-links-secondary",
                            Link {
                                to: Route::History {},
                                class: "nav-link",
                                onclick: close_menu,
                                "History"
                            }
                            Link {
                                to: Route::Favorites {},
                                class: "nav-link",
                                onclick: close_menu,
                                "Favorites"
                            }
                            Link {
                                to: Route::WatchLater {},
                                class: "nav-link",
                                onclick: close_menu,
                                "Later"
                            }
                        }
                        div { class: "nav-overflow",
                            button {
                                class: if menu_open() { "nav-icon-btn on" } else { "nav-icon-btn" },
                                title: "Menu",
                                r#type: "button",
                                onclick: move |_| menu_open.set(!menu_open()),
                                "☰"
                            }
                            if menu_open() {
                                div { class: "nav-menu",
                                    Link {
                                        to: Route::Home {},
                                        class: "nav-menu-item",
                                        onclick: close_menu,
                                        "Home"
                                    }
                                    Link {
                                        to: browse_link("-releaseDate", None, None, None, None, None),
                                        class: "nav-menu-item",
                                        onclick: close_menu,
                                        "Browse"
                                    }
                                    Link {
                                        to: Route::History {},
                                        class: "nav-menu-item",
                                        onclick: close_menu,
                                        "History"
                                    }
                                    Link {
                                        to: Route::Favorites {},
                                        class: "nav-menu-item",
                                        onclick: close_menu,
                                        "Favorites"
                                    }
                                    Link {
                                        to: Route::WatchLater {},
                                        class: "nav-menu-item",
                                        onclick: close_menu,
                                        "Watch later"
                                    }
                                    Link {
                                        to: Route::Settings {},
                                        class: "nav-menu-item",
                                        onclick: close_menu,
                                        "Settings"
                                    }
                                }
                            }
                        }
                    }

                    form {
                        class: "nav-search",
                        onsubmit: move |e| {
                            e.prevent_default();
                            menu_open.set(false);
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
                        }
                    }

                    div { class: "nav-right",
                        button {
                            class: "nav-queue-btn",
                            r#type: "button",
                            title: "Queue",
                            onclick: move |_| {
                                menu_open.set(false);
                                queue_open.set(true);
                            },
                            span { class: "nav-queue-label", "Queue" }
                            span { class: "nav-queue-badge", "{queue_len()}" }
                        }
                        div { class: "nav-user",
                            if let Some(u) = user() {
                                Link {
                                    to: Route::Settings {},
                                    class: "nav-user-name",
                                    onclick: close_menu,
                                    "{u.username}"
                                }
                            } else {
                                Link {
                                    to: Route::Login {},
                                    class: "btn btn-primary nav-signin",
                                    onclick: close_menu,
                                    "Sign in"
                                }
                            }
                        }
                    }
                }
                if menu_open() {
                    button {
                        class: "nav-menu-backdrop",
                        r#type: "button",
                        aria_label: "Close menu",
                        onclick: move |_| menu_open.set(false),
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
