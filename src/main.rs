mod app;
mod models;
mod paths;
mod services;
mod ui;

use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;

fn main() {
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

    let window = WindowBuilder::new()
        .with_title("PMVHeaven")
        .with_decorations(false)
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(900.0, 600.0));

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_disable_context_menu(true),
        )
        .launch(app::App);
}
