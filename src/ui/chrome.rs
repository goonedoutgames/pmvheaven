use dioxus::prelude::*;

pub const LOGO: Asset = asset!("/assets/logo.png");
pub const SEXY_CLOSE: Asset = asset!("/assets/sexy_close.svg");
pub const MAIN_CSS: Asset = asset!("/assets/main.css");
pub const HLS_JS: Asset = asset!("/assets/hls.min.js");

#[component]
pub fn WindowChrome() -> Element {
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
                        dioxus::desktop::window().close();
                    },
                    img { src: SEXY_CLOSE, alt: "Close" }
                }
            }
        }
    }
}
