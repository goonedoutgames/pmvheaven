//! Linux WebKitGTK / Wayland graphics helpers for AppImage + rolling distros.
//!
//! AppImages often ship an older `libwayland-client` that triggers protocol errors
//! against bleeding-edge compositors. Prefer preloading the **system** Wayland
//! client so DMA-BUF / GPU compositing can stay enabled (much better for video).
//!
//! Compare modes with `PMV_GFX`:
//! - `wayland` (default) — system libwayland preload, DMABUF left on
//! - `dmabuf-off` — preload + `WEBKIT_DISABLE_DMABUF_RENDERER=1`
//! - `soft` — preload + DMABUF off + compositing off (slowest, most compatible)
//! - `stock` — no fixes (baseline for A/B)

#![cfg(target_os = "linux")]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const PRELOADED_FLAG: &str = "PMV_WAYLAND_PRELOADED";

/// Call before any WebKit / GTK init.
pub fn prepare() {
    let mode = env::var("PMV_GFX").unwrap_or_else(|_| "wayland".into());
    apply_webkit_flags(&mode);

    if mode == "stock" {
        eprintln!("[pmvheaven] PMV_GFX=stock (no Wayland/WebKit graphics fixes)");
        return;
    }

    let on_wayland = env::var_os("WAYLAND_DISPLAY").is_some()
        || env::var("XDG_SESSION_TYPE")
            .map(|s| s.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);

    if !on_wayland {
        eprintln!("[pmvheaven] PMV_GFX={mode} (not a Wayland session; skip libwayland preload)");
        return;
    }

    if env::var_os(PRELOADED_FLAG).is_some() {
        eprintln!(
            "[pmvheaven] PMV_GFX={mode} (system libwayland preloaded{})",
            webkit_flag_note(&mode)
        );
        return;
    }

    let Some(lib) = find_system_wayland_client() else {
        eprintln!(
            "[pmvheaven] PMV_GFX={mode} (no system libwayland-client found; skip preload{})",
            webkit_flag_note(&mode)
        );
        return;
    };

    reexec_with_preload(&lib, &mode);
}

fn webkit_flag_note(mode: &str) -> &'static str {
    match mode {
        "dmabuf-off" => "; DMABUF renderer disabled",
        "soft" => "; DMABUF+compositing disabled",
        _ => "; DMABUF left enabled",
    }
}

fn apply_webkit_flags(mode: &str) {
    // Only set when unset so users can still override explicitly.
    match mode {
        "dmabuf-off" => {
            set_if_unset("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        "soft" => {
            set_if_unset("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            set_if_unset("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
        _ => {}
    }
}

fn set_if_unset(key: &str, value: &str) {
    if env::var_os(key).is_none() {
        env::set_var(key, value);
    }
}

fn find_system_wayland_client() -> Option<String> {
    const CANDIDATES: &[&str] = &[
        "/usr/lib/libwayland-client.so.0",
        "/usr/lib/libwayland-client.so",
        "/usr/lib64/libwayland-client.so.0",
        "/usr/lib64/libwayland-client.so",
        "/usr/lib/x86_64-linux-gnu/libwayland-client.so.0",
        "/usr/lib/x86_64-linux-gnu/libwayland-client.so",
        "/usr/lib/aarch64-linux-gnu/libwayland-client.so.0",
        "/usr/lib/aarch64-linux-gnu/libwayland-client.so",
        "/lib/x86_64-linux-gnu/libwayland-client.so.0",
        "/lib64/libwayland-client.so.0",
    ];

    for path in CANDIDATES {
        if Path::new(path).is_file() {
            return Some((*path).to_string());
        }
    }

    // Fallback: ask ldconfig (works on most distros).
    let out = Command::new("ldconfig").args(["-p"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if !line.contains("libwayland-client.so") {
            continue;
        }
        if let Some(path) = line.split(" => ").nth(1).map(str::trim) {
            if Path::new(path).is_file() {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn reexec_with_preload(lib: &str, mode: &str) -> ! {
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("/proc/self/exe"));
    let args: Vec<_> = env::args_os().skip(1).collect();

    let preload = match env::var("LD_PRELOAD") {
        Ok(existing) if !existing.is_empty() => {
            if existing.split(':').any(|p| p == lib) {
                existing
            } else {
                format!("{lib}:{existing}")
            }
        }
        _ => lib.to_string(),
    };

    eprintln!(
        "[pmvheaven] PMV_GFX={mode} — re-exec with LD_PRELOAD={preload}{}",
        webkit_flag_note(mode)
    );

    let mut cmd = Command::new(&exe);
    cmd.args(&args)
        .env(PRELOADED_FLAG, "1")
        .env("PMV_GFX", mode)
        .env("LD_PRELOAD", preload);

    // Preserve AppImage / desktop integration env automatically via inherit.
    use std::os::unix::process::CommandExt;
    let err = cmd.exec();
    panic!("failed to re-exec with Wayland preload: {err}");
}
