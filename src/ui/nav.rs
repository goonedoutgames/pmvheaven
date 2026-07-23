use crate::models::AccountUser;
use crate::services::queue;
use crate::ui::chrome::LOGO;
use crate::ui::pages::components::QueuePanel;
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
    let mut queue_len = use_signal(|| queue::len());
    let mut search_q = use_signal(|| String::new());
    let navigator = use_navigator();

    use_effect(move || {
        let _ = queue_open();
        queue_len.set(queue::len());
    });

    rsx! {
        nav { class: "navbar",
            Link { to: Route::Home {}, class: "nav-brand",
                img { src: LOGO, alt: "PMVHeaven" }
                span { "PMVHeaven" }
            }
            div { class: "nav-links",
                Link { to: Route::Home {}, class: "nav-link", "Home" }
                Link { to: Route::Browse { sort: Some("-uploadDate".into()), tags: None, creator: None }, class: "nav-link", "Browse" }
                Link { to: Route::History {}, class: "nav-link", "History" }
                Link { to: Route::Favorites {}, class: "nav-link", "Favorites" }
                Link { to: Route::WatchLater {}, class: "nav-link", "Later" }
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
                    let open = !*queue_open.read();
                    queue_open.set(open);
                    queue_len.set(queue::len());
                },
                "Queue ({queue_len()})"
            }
            div { class: "nav-user",
                if let Some(u) = user() {
                    Link { to: Route::Settings {}, "{u.username}" }
                } else {
                    Link { to: Route::Login {}, class: "btn btn-primary", "Sign in" }
                }
            }
        }
        div { class: "content",
            Outlet::<Route> {}
        }
        QueuePanel {}
    }
}

pub use crate::ui::pages::*;
