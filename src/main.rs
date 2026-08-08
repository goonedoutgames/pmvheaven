mod app;
mod models;
mod paths;
mod services;
mod ui;

#[cfg(target_os = "linux")]
mod linux_gfx;

use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;

fn window_icon() -> Option<Icon> {
    // 256×256 RGBA baked from assets/app-icon-256.png (no runtime image crate).
    const RGBA: &[u8] = include_bytes!("../assets/app-icon-256.rgba");
    Icon::from_rgba(RGBA.to_vec(), 256, 256).ok()
}

fn main() {
    // Must run before WebKit/GTK init so child WebKitWebProcess inherits flags,
    // and so we can re-exec with LD_PRELOAD of the system Wayland client.
    #[cfg(target_os = "linux")]
    linux_gfx::prepare();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // DB must be ready before any component reads account/session state.
    let _ = paths::ensure_data_dir();
    if let Err(e) = services::db::init_db() {
        tracing::error!("db init failed: {e}");
    }
    // Warm crypto key so first encrypt during sign-in never races the DB lock.
    services::crypto::ensure_key();
    services::queue::load_queue();

    let mut window = WindowBuilder::new()
        .with_title("PMVHeaven")
        .with_decorations(false)
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(900.0, 600.0));
    if let Some(icon) = window_icon() {
        // Title-bar / Alt-Tab small icon (ICON_SMALL).
        window = window.with_window_icon(Some(icon));
    }
    // Taskbar / Win+Tab use ICON_BIG via with_taskbar_icon — Start Menu can look
    // correct from the installer .ico while the running window still shows the
    // default unless this is set.
    #[cfg(target_os = "windows")]
    {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
        if let Some(icon) = window_icon() {
            window = window.with_taskbar_icon(Some(icon));
        }
    }

    let mut cfg = Config::new()
        .with_window(window)
        .with_disable_context_menu(true)
        .with_custom_head(
            r#"<meta name="color-scheme" content="dark">
                    <style>
                      html, body { background: #0a0a0c; }
                      /* Keep media on the GPU compositor thread. */
                      video, .player-video {
                        transform: translate3d(0,0,0);
                        -webkit-transform: translate3d(0,0,0);
                        backface-visibility: hidden;
                      }
                    </style>"#
            .into(),
        );
    if let Some(icon) = window_icon() {
        cfg = cfg.with_icon(icon);
    }

    LaunchBuilder::desktop().with_cfg(cfg).launch(app::App);
}
