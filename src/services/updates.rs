//! Check GitHub Releases for a newer semver than this binary.

use crate::services::db::{get_setting, set_setting};
use serde::Deserialize;
use semver::Version;

const REPO: &str = "goonedoutgames/pmvheaven";
const DISMISS_KEY: &str = "dismissed_update_version";

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub version: String,
    pub html_url: String,
    /// Best-effort direct asset URL for this OS; falls back to release page.
    pub download_url: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

pub fn dismissed_version() -> Option<String> {
    get_setting(DISMISS_KEY)
}

pub fn dismiss_update(version: &str) {
    set_setting(DISMISS_KEY, version);
}

pub async fn check_for_update() -> Option<AvailableUpdate> {
    let current = match Version::parse(APP_VERSION) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("invalid CARGO_PKG_VERSION {APP_VERSION}: {e}");
            return None;
        }
    };

    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent(format!("PMVHeaven/{APP_VERSION}"))
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .ok()?;

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("update check network error: {e}");
            return None;
        }
    };

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        // No releases yet.
        return None;
    }
    if !resp.status().is_success() {
        tracing::debug!("update check HTTP {}", resp.status());
        return None;
    }

    let release: GhRelease = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("update check parse error: {e}");
            return None;
        }
    };

    if release.draft || release.prerelease {
        return None;
    }

    let remote = parse_tag_version(&release.tag_name)?;
    if remote <= current {
        return None;
    }

    let version = remote.to_string();
    if dismissed_version().as_deref() == Some(version.as_str()) {
        return None;
    }

    let download_url = prefer_asset(&release.assets)
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_else(|| release.html_url.clone());

    Some(AvailableUpdate {
        version,
        html_url: release.html_url,
        download_url,
        name: release.name,
    })
}

fn parse_tag_version(tag: &str) -> Option<Version> {
    let t = tag.trim().trim_start_matches('v');
    Version::parse(t).ok()
}

fn prefer_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    #[cfg(target_os = "linux")]
    {
        assets
            .iter()
            .find(|a| a.name.ends_with(".flatpak"))
            .or_else(|| assets.iter().find(|a| a.name.ends_with(".AppImage")))
            .or_else(|| assets.iter().find(|a| a.name.contains("linux")))
    }
    #[cfg(target_os = "windows")]
    {
        assets
            .iter()
            .find(|a| {
                let n = a.name.to_ascii_lowercase();
                n.ends_with(".exe") && (n.contains("setup") || n.contains("installer"))
            })
            .or_else(|| {
                assets
                    .iter()
                    .find(|a| a.name.to_ascii_lowercase().ends_with(".exe"))
            })
            .or_else(|| {
                assets
                    .iter()
                    .find(|a| a.name.to_ascii_lowercase().ends_with(".msi"))
            })
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = assets;
        None
    }
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
}
