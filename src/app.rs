use crate::models::{PlayableVideo, VideoSummary};
use crate::paths;
use crate::services::db::{get_setting, set_setting};
use crate::services::pmv::get_account_user;
use crate::services::repo::watched_progress_map;
use crate::services::stream_proxy;
use crate::ui::chrome::{MAIN_CSS, HLS_JS, WindowChrome};
use crate::ui::legacy_gate::LegacyDbGate;
use crate::ui::nav::Route;
use crate::ui::update_prompt::UpdatePrompt;
use dioxus::prelude::*;

const PLAYER_RAIL_WIDTH_KEY: &str = "player_rail_width";
const HOVER_PREVIEWS_KEY: &str = "hover_previews";

#[component]
pub fn App() -> Element {
    let ready = use_signal(|| false);
    let mut show_legacy = use_signal(|| paths::has_legacy_db());
    let user = use_signal(get_account_user);
    let queue_open = use_signal(|| false);
    let queue_tick = use_signal(|| 0u32);
    let now_playing = use_signal(|| None::<PlayableVideo>);
    let start_at = use_signal(|| 0.0f64);
    let play_choice = use_signal(|| None::<VideoSummary>);
    let proxy_base = use_signal(|| String::new());
    let watched_map = use_signal(watched_progress_map);
    let player_fs = use_signal(|| false);
    let player_rail_w = use_signal(load_player_rail_width);
    let hover_previews = use_signal(load_hover_previews);

    use_context_provider(|| user);
    use_context_provider(|| queue_open);
    use_context_provider(|| queue_tick);
    use_context_provider(|| now_playing);
    use_context_provider(|| start_at);
    use_context_provider(|| play_choice);
    use_context_provider(|| proxy_base);
    use_context_provider(|| watched_map);
    use_context_provider(|| player_fs);
    use_context_provider(|| player_rail_w);
    use_context_provider(|| hover_previews);

    use_future(move || async move {
        if show_legacy() {
            return;
        }
        bootstrap(proxy_base, ready, now_playing, start_at).await;
    });

    let main_class = if player_fs() { "is-player-fs" } else { "" };
    let rail_style = player_rail_w()
        .map(|w| format!("--player-rail-w:{w}px"))
        .unwrap_or_default();

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Script { src: HLS_JS }
        div { id: "main", class: "{main_class}", style: "{rail_style}",
            WindowChrome {}
            if show_legacy() {
                LegacyDbGate {
                    on_cleared: move |_| {
                        show_legacy.set(false);
                        spawn(async move {
                            bootstrap(proxy_base, ready, now_playing, start_at).await;
                        });
                    }
                }
            } else if !ready() {
                div { class: "loading", style: "padding-top: 20vh;", "Starting PMVHeaven…" }
            } else {
                div { class: "app-body",
                    Router::<Route> {}
                }
                UpdatePrompt {}
            }
        }
    }
}

fn load_player_rail_width() -> Option<u32> {
    get_setting(PLAYER_RAIL_WIDTH_KEY)
        .and_then(|s| s.parse().ok())
        .filter(|w| (320..=1000).contains(w))
}

pub fn save_player_rail_width(w: u32) {
    if (320..=1000).contains(&w) {
        set_setting(PLAYER_RAIL_WIDTH_KEY, &w.to_string());
    }
}

fn load_hover_previews() -> bool {
    matches!(get_setting(HOVER_PREVIEWS_KEY).as_deref(), Some("1") | Some("true"))
}

pub fn save_hover_previews(on: bool) {
    set_setting(HOVER_PREVIEWS_KEY, if on { "1" } else { "0" });
}

async fn bootstrap(
    mut proxy_base: Signal<String>,
    mut ready: Signal<bool>,
    now_playing: Signal<Option<PlayableVideo>>,
    start_at: Signal<f64>,
) {
    match stream_proxy::start_proxy().await {
        Ok((base, _shutdown)) => {
            tracing::info!("stream proxy at {base}");
            proxy_base.set(base);
        }
        Err(e) => tracing::error!("proxy failed: {e}"),
    }
    if crate::services::player::restore_into(now_playing, start_at) {
        tracing::info!("restored previous playback session");
    }
    ready.set(true);
}
