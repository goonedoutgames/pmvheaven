use crate::models::PlayableVideo;
use crate::services::player;
use dioxus::prelude::*;

pub const LOGO: Asset = asset!("/assets/logo.png");
pub const SEXY_CLOSE: Asset = asset!("/assets/sexy_close.svg");
pub const MAIN_CSS: Asset = asset!("/assets/main.css");
pub const HLS_JS: Asset = asset!("/assets/hls.min.js");

#[component]
pub fn WindowChrome() -> Element {
    // Optional: flush now-playing position before quit.
    let now_playing = try_use_context::<Signal<Option<PlayableVideo>>>();
    let start_at = try_use_context::<Signal<f64>>();

    rsx! {
        header { class: "titlebar",
            div { class: "titlebar-brand",
                img { src: LOGO, alt: "PMVHeaven" }
                span {
                    "PMV"
                    span { class: "accent", "Heaven" }
                }
            }
            div { class: "titlebar-controls",
                button {
                    class: "titlebar-btn",
                    title: "Minimize",
                    onclick: move |_| {
                        dioxus::desktop::window().set_minimized(true);
                    },
                    "—"
                }
                button {
                    class: "titlebar-btn",
                    title: "Maximize",
                    onclick: move |_| {
                        let win = dioxus::desktop::window();
                        win.set_maximized(!win.is_maximized());
                    },
                    "□"
                }
                button {
                    class: "titlebar-btn close",
                    title: "Close",
                    onclick: move |_| {
                        if let (Some(np), Some(at)) = (now_playing, start_at) {
                            if let Some(playable) = np() {
                                let t = at();
                                if t >= 1.0 {
                                    player::save_now_playing(&playable, t);
                                }
                            }
                        }
                        // Best-effort: grab live currentTime once more before exit.
                        spawn(async move {
                            if let Ok(v) = document::eval(
                                r#"
                                const el = document.getElementById('pmv-player');
                                if (!el || !el.dataset.vid) return null;
                                return (el.dataset.vid || '') + '|' + (el.currentTime || 0);
                                "#,
                            )
                            .await
                            {
                                if let Some(raw) = v.as_str() {
                                    let mut parts = raw.split('|');
                                    let vid = parts.next().unwrap_or("");
                                    let t: f64 =
                                        parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                                    if t >= 1.0 {
                                        if let Some(np) = now_playing {
                                            if let Some(playable) = np() {
                                                if playable.summary.id == vid {
                                                    player::save_now_playing(&playable, t);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            dioxus::desktop::window().close();
                        });
                    },
                    img { src: SEXY_CLOSE, alt: "Close" }
                }
            }
        }
    }
}
