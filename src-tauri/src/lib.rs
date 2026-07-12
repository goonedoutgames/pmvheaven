use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Manager, RunEvent};

/// Holds the spawned Next.js server process so it can be killed on exit.
struct ServerProc(Mutex<Option<Child>>);

const SERVER_ADDR: &str = "127.0.0.1:3000";

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
