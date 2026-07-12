#[cfg(not(debug_assertions))]
use std::net::TcpStream;
use std::process::Child;
#[cfg(not(debug_assertions))]
use std::process::Command;
use std::sync::Mutex;
#[cfg(not(debug_assertions))]
use std::time::{Duration, Instant};

use tauri::{Manager, RunEvent};

/// Holds the spawned Next.js server process so it can be killed on exit.
struct ServerProc(Mutex<Option<Child>>);

#[cfg(not(debug_assertions))]
const SERVER_ADDR: &str = "127.0.0.1:3000";

#[cfg(not(debug_assertions))]
fn wait_for_port(addr: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(addr).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's native-Wayland path is broken on many GPU/driver combos. Two
    // independent things go wrong, so both need addressing (this keeps HARDWARE
    // acceleration — we never enable the software `WEBKIT_DISABLE_COMPOSITING_MODE`):
    //
    //  1. GDK_BACKEND=x11 — run the window under XWayland. Native-Wayland
    //     accelerated compositing renders blank/white here, and client-side
    //     titlebar buttons don't respond; XWayland gives working accelerated
    //     compositing and server-side (WM-drawn) decorations.
    //  2. WEBKIT_DISABLE_DMABUF_RENDERER=1 — WebKit's GPU process opens its own
    //     Wayland connection for zero-copy DMABUF buffers and crashes with
    //     "Error 71 (Protocol error) dispatching to Wayland display". Disabling
    //     DMABUF uses the (still GPU-accelerated) texture-upload path instead.
    //
    // Both respect an explicit value, so users on working setups can opt out
    // (e.g. `GDK_BACKEND=wayland`).
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("GDK_BACKEND").is_none() {
            std::env::set_var("GDK_BACKEND", "x11");
        }
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    tauri::Builder::default()
        .manage(ServerProc(Mutex::new(None)))
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window not found");

            // In release builds we ship and launch the standalone Next.js
            // server ourselves. In debug builds the `beforeDevCommand`
            // (`pnpm dev`) already provides the server on port 3000.
            #[cfg(not(debug_assertions))]
            {
                let resource_dir =
                    app.path().resource_dir().expect("no resource dir");
                let server_dir = resource_dir.join("server");
                let server_js = server_dir.join("server.js");

                // The resource dir is read-only in packaged installs, so keep
                // the SQLite database (and secrets) in the writable app-data dir.
                let data_dir = app
                    .path()
                    .app_data_dir()
                    .expect("no app data dir")
                    .join("data");
                let _ = std::fs::create_dir_all(&data_dir);

                match Command::new("node")
                    .arg(&server_js)
                    .current_dir(&server_dir)
                    .env("PORT", "3000")
                    .env("HOSTNAME", "127.0.0.1")
                    .env("NODE_ENV", "production")
                    .env("PH_DATA_DIR", &data_dir)
                    .spawn()
                {
                    Ok(child) => {
                        app.state::<ServerProc>().0.lock().unwrap().replace(child);
                        wait_for_port(SERVER_ADDR, Duration::from_secs(30));
                    }
                    Err(err) => {
                        eprintln!("Failed to start Next.js server: {err}");
                    }
                }
            }

            // WebKitGTK ships with MediaSource (MSE) disabled by default, which
            // breaks hls.js. Enable it (plus the web inspector) directly on the
            // underlying WebKitWebView.
            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{SettingsExt, WebViewExt};
                let _ = window.with_webview(|webview| {
                    let wv = webview.inner();
                    if let Some(settings) = WebViewExt::settings(&wv) {
                        settings.set_enable_mediasource(true);
                        settings.set_enable_developer_extras(true);
                        settings.set_media_playback_requires_user_gesture(false);
                    }
                });
            }

            let _ = window.show();
            let _ = window.set_focus();
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app_handle.try_state::<ServerProc>() {
                    if let Some(mut child) = state.0.lock().unwrap().take() {
                        let _ = child.kill();
                    }
                }
            }
        });
}
