use crate::models::{AccountUser, PlayableVideo, VideoSummary};
use crate::paths;
use crate::services::db::{get_setting, set_setting};
use crate::services::pmv::{get_account_user, shared_client};
use crate::services::repo::watched_progress_map;
use crate::services::stream_proxy;
use crate::ui::chrome::{MAIN_CSS, HLS_JS, WindowChrome};
use crate::ui::ctx::{
    HoverPreviewVolume, HoverPreviews, PausePreviewsWhilePlaying, PlayerFs, PlayerQueueH,
    PlayerRailW, ProxyBase, QueueOpen, QueueTick, StartAt,
};
use crate::ui::legacy_gate::LegacyDbGate;
use crate::ui::nav::Route;
use crate::ui::update_prompt::UpdatePrompt;
use dioxus::prelude::*;

const PLAYER_RAIL_WIDTH_KEY: &str = "player_rail_width";
const PLAYER_QUEUE_HEIGHT_KEY: &str = "player_queue_height";
const HOVER_PREVIEWS_KEY: &str = "hover_previews";
const PAUSE_PREVIEWS_WHILE_PLAYING_KEY: &str = "pause_previews_while_playing";
const HOVER_PREVIEW_VOLUME_KEY: &str = "hover_preview_volume";

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
    let player_queue_h = use_signal(load_player_queue_height);
    let hover_previews = use_signal(load_hover_previews);
    let pause_previews_while_playing = use_signal(load_pause_previews_while_playing);
    let hover_preview_volume = use_signal(load_hover_preview_volume);

    // Typed wrappers — bare Signal<bool> contexts collide with each other.
    use_context_provider(|| user);
    use_context_provider(|| QueueOpen(queue_open));
    use_context_provider(|| QueueTick(queue_tick));
    use_context_provider(|| now_playing);
    use_context_provider(|| StartAt(start_at));
    use_context_provider(|| play_choice);
    use_context_provider(|| ProxyBase(proxy_base));
    use_context_provider(|| watched_map);
    use_context_provider(|| PlayerFs(player_fs));
    use_context_provider(|| PlayerRailW(player_rail_w));
    use_context_provider(|| PlayerQueueH(player_queue_h));
    use_context_provider(|| HoverPreviews(hover_previews));
    use_context_provider(|| PausePreviewsWhilePlaying(pause_previews_while_playing));
    use_context_provider(|| HoverPreviewVolume(hover_preview_volume));

    use_future(move || async move {
        if show_legacy() {
            return;
        }
        bootstrap(proxy_base, ready, now_playing, start_at, user).await;
    });

    let main_class = if player_fs() { "is-player-fs" } else { "" };

    // Keep layout vars on <html> so #main style diffs don't clobber live resize.
    use_effect(move || {
        let w = player_rail_w();
        let h = player_queue_h();
        spawn(async move {
            let w_js = w
                .map(|n| format!("'{n}px'"))
                .unwrap_or_else(|| "null".into());
            let h_js = h
                .map(|n| format!("'{n}px'"))
                .unwrap_or_else(|| "null".into());
            let _ = document::eval(&format!(
                r#"
                const s = document.documentElement.style;
                const w = {w_js};
                const h = {h_js};
                if (w) s.setProperty('--player-rail-w', w);
                else s.removeProperty('--player-rail-w');
                if (h) s.setProperty('--player-queue-h', h);
                else s.removeProperty('--player-queue-h');
                return 'layout-vars';
                "#
            ))
            .await;
        });
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Script { src: HLS_JS }
        div { id: "main", class: "{main_class}",
            WindowChrome {}
            if show_legacy() {
                LegacyDbGate {
                    on_cleared: move |_| {
                        show_legacy.set(false);
                        spawn(async move {
                            bootstrap(proxy_base, ready, now_playing, start_at, user).await;
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
        .filter(|w| (280..=1400).contains(w))
}

pub fn save_player_rail_width(w: u32) {
    if (280..=1400).contains(&w) {
        set_setting(PLAYER_RAIL_WIDTH_KEY, &w.to_string());
    }
}

fn load_player_queue_height() -> Option<u32> {
    get_setting(PLAYER_QUEUE_HEIGHT_KEY)
        .and_then(|s| s.parse().ok())
        .filter(|h| (88..=1200).contains(h))
}

pub fn save_player_queue_height(h: u32) {
    if (88..=1200).contains(&h) {
        set_setting(PLAYER_QUEUE_HEIGHT_KEY, &h.to_string());
    }
}

fn load_hover_previews() -> bool {
    matches!(get_setting(HOVER_PREVIEWS_KEY).as_deref(), Some("1") | Some("true"))
}

pub fn save_hover_previews(on: bool) {
    set_setting(HOVER_PREVIEWS_KEY, if on { "1" } else { "0" });
}

/// Default on: skip card hover previews while the rail is playing (less decode hitching).
fn load_pause_previews_while_playing() -> bool {
    match get_setting(PAUSE_PREVIEWS_WHILE_PLAYING_KEY).as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        // Missing key → enabled (opt-out).
        _ => true,
    }
}

pub fn save_pause_previews_while_playing(on: bool) {
    set_setting(PAUSE_PREVIEWS_WHILE_PLAYING_KEY, if on { "1" } else { "0" });
}

/// 0.0 = muted (default), 1.0 = full. Stored as 0–100 percent.
fn load_hover_preview_volume() -> f32 {
    get_setting(HOVER_PREVIEW_VOLUME_KEY)
        .and_then(|s| s.parse::<f32>().ok())
        .map(|n| {
            if n > 1.0 {
                (n / 100.0).clamp(0.0, 1.0)
            } else {
                n.clamp(0.0, 1.0)
            }
        })
        .unwrap_or(0.0)
}

pub fn save_hover_preview_volume(vol: f32) {
    let pct = (vol.clamp(0.0, 1.0) * 100.0).round() as u32;
    set_setting(HOVER_PREVIEW_VOLUME_KEY, &pct.to_string());
}

async fn bootstrap(
    mut proxy_base: Signal<String>,
    mut ready: Signal<bool>,
    now_playing: Signal<Option<PlayableVideo>>,
    start_at: Signal<f64>,
    mut user: Signal<Option<AccountUser>>,
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
    // Refresh session so avatar / username match PMVHaven (local row can be stale).
    if user.peek().is_some() {
        match shared_client().refresh_profile().await {
            Some(u) => user.set(Some(u)),
            None => tracing::warn!("profile refresh failed; keeping cached account"),
        }
    }
    ready.set(true);
}
