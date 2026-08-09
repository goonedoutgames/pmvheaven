use crate::models::PlayableVideo;
use crate::services::player;
use crate::ui::ctx::StartAt;
use dioxus::prelude::*;

// Windows (and dx bundle/NSIS): Dioxus `asset!()` requires the dx linker.
#[cfg(not(target_os = "linux"))]
pub const LOGO: Asset = asset!("/assets/logo.png");
#[cfg(not(target_os = "linux"))]
pub const SEXY_CLOSE: Asset = asset!("/assets/sexy_close.svg");
#[cfg(not(target_os = "linux"))]
pub const MAIN_CSS: Asset = asset!("/assets/main.css");
#[cfg(not(target_os = "linux"))]
pub const HLS_JS: Asset = asset!("/assets/hls.min.js");

// Linux Flatpak uses plain `cargo` (no dx linker), so `asset!()` 404s at runtime.
// Embed CSS/images instead. Playback uses native HLS / proxy — not these assets.
#[cfg(target_os = "linux")]
mod linux_assets {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use std::sync::LazyLock;

    fn data_url(mime: &str, bytes: &[u8]) -> String {
        format!("data:{mime};base64,{}", STANDARD.encode(bytes))
    }

    pub static LOGO: LazyLock<String> =
        LazyLock::new(|| data_url("image/png", include_bytes!("../../assets/logo.png")));
    pub static SEXY_CLOSE: LazyLock<String> = LazyLock::new(|| {
        data_url("image/svg+xml", include_bytes!("../../assets/sexy_close.svg"))
    });
    pub const MAIN_CSS_INLINE: &str = include_str!("../../assets/main.css");
}
#[cfg(target_os = "linux")]
pub use linux_assets::{LOGO, MAIN_CSS_INLINE, SEXY_CLOSE};

/// Stylesheet (+ hls.js on Windows). Linux injects CSS as a `<style>` tag so
/// Flatpak cargo builds don't need the dx asset bundle.
pub fn head_assets() -> Element {
    #[cfg(not(target_os = "linux"))]
    {
        rsx! {
            document::Link { rel: "stylesheet", href: MAIN_CSS }
            document::Script { src: HLS_JS }
        }
    }
    #[cfg(target_os = "linux")]
    {
        rsx! {
            document::Style { "{MAIN_CSS_INLINE}" }
        }
    }
}

#[component]
pub fn BrandLogo(#[props(default)] class: String, #[props(default)] style: String) -> Element {
    #[cfg(not(target_os = "linux"))]
    {
        if style.is_empty() {
            rsx! { img { class: "{class}", src: LOGO, alt: "PMVHeaven" } }
        } else {
            rsx! { img { class: "{class}", src: LOGO, alt: "PMVHeaven", style: "{style}" } }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let src = LOGO.as_str();
        if style.is_empty() {
            rsx! { img { class: "{class}", src: "{src}", alt: "PMVHeaven" } }
        } else {
            rsx! { img { class: "{class}", src: "{src}", alt: "PMVHeaven", style: "{style}" } }
        }
    }
}

#[component]
fn SexyCloseIcon() -> Element {
    #[cfg(not(target_os = "linux"))]
    {
        rsx! { img { src: SEXY_CLOSE, alt: "Close" } }
    }
    #[cfg(target_os = "linux")]
    {
        let src = SEXY_CLOSE.as_str();
        rsx! { img { src: "{src}", alt: "Close" } }
    }
}

#[component]
pub fn WindowChrome() -> Element {
    // Optional: flush now-playing position before quit.
    let now_playing = try_use_context::<Signal<Option<PlayableVideo>>>();
    let start_at = try_use_context::<StartAt>().map(|s| s.0);

    rsx! {
        header { class: "titlebar",
            div { class: "titlebar-brand",
                BrandLogo {}
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
                    SexyCloseIcon {}
                }
            }
        }
    }
}
