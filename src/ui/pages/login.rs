use crate::models::AccountUser;
use crate::services::pmv::shared_client;
use crate::services::sync::sync_watch_history;
use crate::ui::chrome::LOGO;
use crate::ui::nav::Route;
use crate::ui::pages::components::refresh_watched_map;
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn Login() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut user = use_context::<Signal<Option<AccountUser>>>();
    let watched_map = use_context::<Signal<HashMap<String, f64>>>();
    let navigator = use_navigator();

    rsx! {
        div { class: "login-page",
            div { style: "text-align:center; margin-bottom:1.5rem;",
                img { src: LOGO, alt: "PMVHeaven", style: "width:72px; height:72px; border-radius:16px;" }
                h1 { "Sign in" }
                p { class: "muted", "Connect your PMVHaven account. Credentials stay encrypted on this device." }
            }
            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }
            form {
                onsubmit: move |evt| {
                    evt.prevent_default();
                    if busy() {
                        return;
                    }
                    let em = email().trim().to_string();
                    let pw = password();
                    if em.is_empty() || pw.is_empty() {
                        error.set(Some("Email and password are required".into()));
                        return;
                    }
                    busy.set(true);
                    error.set(None);
                    spawn(async move {
                        let result = shared_client().sign_in(&em, &pw).await;
                        match result {
                            Ok(u) => {
                                user.set(Some(u));
                                busy.set(false);
                                navigator.push(Route::Home {});
                                spawn(async move {
                                    let _ = sync_watch_history().await;
                                    refresh_watched_map(watched_map);
                                });
                            }
                            Err(e) => {
                                tracing::warn!("sign-in failed: {e}");
                                error.set(Some(e.to_string()));
                                busy.set(false);
                            }
                        }
                    });
                },
                div { class: "form-field",
                    label { "Email" }
                    input {
                        r#type: "email",
                        required: true,
                        autocomplete: "username",
                        value: "{email}",
                        oninput: move |e| email.set(e.value()),
                    }
                }
                div { class: "form-field",
                    label { "Password" }
                    input {
                        r#type: "password",
                        required: true,
                        autocomplete: "current-password",
                        value: "{password}",
                        oninput: move |e| password.set(e.value()),
                    }
                }
                button {
                    class: "btn btn-primary",
                    r#type: "submit",
                    disabled: busy(),
                    style: "width:100%;",
                    if busy() { "Signing in…" } else { "Sign in" }
                }
            }
        }
    }
}
