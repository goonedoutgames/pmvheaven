use crate::services::updates::{self, AvailableUpdate};
use dioxus::prelude::*;

#[component]
pub fn UpdatePrompt() -> Element {
    let mut update = use_signal(|| None::<AvailableUpdate>);

    use_future(move || async move {
        // Don't block startup — check a moment after launch.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        if let Some(u) = updates::check_for_update().await {
            tracing::info!("update available: {}", u.version);
            update.set(Some(u));
        }
    });

    let Some(info) = update() else {
        return rsx! {};
    };

    let version = info.version.clone();
    let version_dismiss = version.clone();
    let download_url = info.download_url.clone();
    let html_url = info.html_url.clone();
    let title = info
        .name
        .clone()
        .unwrap_or_else(|| format!("PMVHeaven {version}"));

    rsx! {
        div { class: "modal-backdrop update-prompt",
            div { class: "modal",
                h2 { "Update available" }
                p {
                    "You're on "
                    strong { "v{updates::APP_VERSION}" }
                    ". "
                    "{title} (v{version}) is ready to download."
                }
                div { class: "modal-actions",
                    button {
                        class: "btn btn-ghost",
                        onclick: move |_| {
                            updates::dismiss_update(&version_dismiss);
                            update.set(None);
                        },
                        "Not now"
                    }
                    button {
                        class: "btn btn-ghost",
                        title: "Open the release page",
                        onclick: move |_| {
                            updates::open_url(&html_url);
                        },
                        "Release notes"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            updates::open_url(&download_url);
                        },
                        "Download"
                    }
                }
            }
        }
    }
}

