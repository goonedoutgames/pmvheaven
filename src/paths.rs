//! App data paths and legacy v1 database detection.

use std::path::PathBuf;

/// Prefer the same identifier family as the Dioxus/Tauri app data folder.
pub fn app_data_dir() -> PathBuf {
    // Match common desktop app-data layout used by the v1 Tauri build:
    //   Linux:   ~/.local/share/com.pmvheaven.app
    //   Windows: %APPDATA%\com.pmvheaven.app
    if let Some(dir) = dirs::data_dir() {
        let candidate = dir.join("com.pmvheaven.desktop");
        if candidate.exists() || !v1_alt_paths().iter().any(|p| p.exists()) {
            return candidate;
        }
        for alt in v1_alt_paths() {
            if alt.join("pmvheaven.db").exists() {
                return alt;
            }
        }
        return candidate;
    }
    PathBuf::from(".").join("data")
}

fn v1_alt_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(d) = dirs::data_dir() {
        out.push(d.join("pmvheaven"));
        out.push(d.join("PMVHeaven"));
    }
    out
}

pub fn v1_db_path() -> PathBuf {
    app_data_dir().join("pmvheaven.db")
}

pub fn v2_db_path() -> PathBuf {
    app_data_dir().join("pmvheaven_v2.db")
}

pub fn queue_path() -> PathBuf {
    app_data_dir().join("queue.json")
}

pub fn now_playing_path() -> PathBuf {
    app_data_dir().join("now_playing.json")
}

/// True when a legacy v1 database is still present (prompt to remove; no migration).
pub fn has_legacy_db() -> bool {
    v1_db_path().exists()
}

/// Remove legacy v1 database files (including WAL/SHM sidecars).
pub fn remove_legacy_db() -> std::io::Result<()> {
    let base = v1_db_path();
    for suffix in ["", "-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{}", base.display(), suffix));
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }
    Ok(())
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir())
}
