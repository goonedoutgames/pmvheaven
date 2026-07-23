use crate::paths;
use dioxus::prelude::*;

#[component]
pub fn LegacyDbGate(on_cleared: EventHandler<()>) -> Element {
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div { class: "modal-backdrop",
            div { class: "modal",
                h2 { "Legacy data found" }
                p {
                    "An older PMVHeaven (v1) database was found in the app data folder. "
                    "v2 cannot migrate it. Remove the old database to start fresh, or quit."
                }
                if let Some(err) = error() {
                    p { class: "error", "{err}" }
                }
                div { class: "modal-actions",
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            #[cfg(feature = "desktop")]
                            {
                                dioxus::desktop::window().close();
                            }
                        },
                        "Quit"
                    }
                    button {
                        class: "btn btn-danger",
                        onclick: move |_| {
                            match paths::remove_legacy_db() {
                                Ok(()) => on_cleared.call(()),
                                Err(e) => error.set(Some(e.to_string())),
                            }
                        },
                        "Remove old data"
                    }
                }
            }
        }
    }
}
