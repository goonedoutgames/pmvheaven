# PMVHeaven v2

Ad-free desktop client for [PMVHaven](https://pmvhaven.com), rewritten in **Rust + [Dioxus](https://dioxuslabs.com/) 0.7**. One native binary — no Node/Next.js sidecar.

## Features

- **Browse & discover** — trending, top-rated, newest, popular tags, infinite scroll
- **Search** — authenticated search with tag fallback when signed out
- **Playback** — HTML5 video in the system WebView, HLS via vendored `hls.js`, Rust localhost media proxy (SSRF-guarded)
- **Permanent watch history** — SQLite archive that never prunes; pull-sync from PMVHaven session history
- **Play queue** — add / play next / reorder / clear; persisted to `queue.json`
- **Favorites & Watch Later** — local mirror with remote write-back
- **Custom chrome** — undecorated window with brand logo and `sexy_close.svg`

## Requirements

- Rust stable (1.85+)
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started): `cargo install dioxus-cli`
- **Linux:** WebKitGTK (`webkit2gtk`), GTK3, and `xdotool` (provides `libxdo` — required to link Dioxus desktop)
- **Windows:** WebView2 runtime

On Arch/CachyOS:

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 xdotool base-devel
```

## Develop

```bash
dx serve --platform desktop
# or
cargo run
```

## Bundle

```bash
dx bundle --platform desktop --release
```

On Linux this produces an AppImage under:

`target/dx/pmvheaven/bundle/linux/appimage/pmvheaven_2.0.0_x86_64.AppImage`

No Node runtime is bundled. Windows builds use the same command on a Windows host (MSI/NSIS via the Dioxus bundler).

## Data

App data lives under the OS data dir for identifier `com.pmvheaven.desktop`
(e.g. `~/.local/share/com.pmvheaven.desktop` on Linux):

| File | Purpose |
|------|---------|
| `pmvheaven_v2.db` | v2 SQLite (account, videos, history, favorites, …) |
| `queue.json` | Ephemeral play queue |
| `pmvheaven.db` | **Legacy v1 only** — if present on first launch, the app prompts to delete it |

This is a **breaking** rewrite: v1 databases are not migrated.

Optional: set `PH_SECRET` for a stable AES-256-GCM key; otherwise a random key is stored in settings.

## Layout

```
src/
  main.rs / app.rs
  models.rs / paths.rs
  services/   # db, crypto, pmv client, sync, queue, stream proxy
  ui/         # chrome, router, pages
assets/       # logo.png, sexy_close.svg, hls.min.js, main.css
public/       # logo.png + sexy_close.svg (brand copies)
```

## Auth note

Sign-in uses PMVHaven Better Auth (`POST /auth/sign-in/email`). Session cookies are encrypted at rest. Watch history is pulled from `/auth/session` (`watchHistory` + `watchProgress`) because `/user/watch-history` is broken server-side.
