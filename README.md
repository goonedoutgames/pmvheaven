# PMVHeaven v2

Ad-free desktop client for [PMVHaven](https://pmvhaven.com), rewritten in **Rust + [Dioxus](https://dioxuslabs.com/) 0.7**. One native binary — no Node/Next.js sidecar.

## Features

- **Browse & discover** — site-parity filters (sort, tags, models, music, creator, duration/rating/views, content chips), trending home, infinite scroll
- **Search** — title/text search via PMVHaven `/api/videos/search` (works signed out)
- **Uploader profiles** — click a username to browse that uploader’s videos (`uploader=` filter)
- **Diegetic discovery** — tags, models, creators, and music chips on the watch page jump into browse filters
- **Playback** — HTML5 video in the system WebView, HLS via vendored `hls.js`, Rust localhost media proxy (SSRF-guarded)
- **Permanent watch history** — SQLite archive; pull from PMVHaven session + push local watches back via `/users/watch-progress`
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

**Windows** (CI publishes these): NSIS installer under `target/dx/…` (e.g. `*-setup.exe`).

**Linux:** no CI AppImage — build from source on your machine:

```bash
dx bundle --platform desktop --release
# optional Wayland/AppImage post-process if you produced an AppImage locally:
./scripts/fix-appimage-wayland.sh
```

No Node runtime is bundled.

## Releases (CI)

Pushes to `main` run [`.github/workflows/release.yml`](.github/workflows/release.yml):

1. Read semver from `Cargo.toml` (`2.2.7` → tag `v2.2.7`)
2. Skip if that GitHub Release already exists (bump the Cargo version to cut a new one)
3. Build **Windows NSIS `.exe` installer** only
4. Publish a GitHub Release with that artifact

Linux users: install deps above, then `dx serve` / `dx bundle` from source.

Manual re-run: Actions → **Release** → **Run workflow** (optionally force recreate).

On launch the app checks `goonedoutgames/pmvheaven` for a newer release and offers a download link when one is available.

### Linux graphics A/B (`PMV_GFX`)

When you build a Linux AppImage locally, after `dx bundle` run:

```bash
./scripts/fix-appimage-wayland.sh
```

That script bundles WebKit helpers + GStreamer plugins and relocates hardcoded Ubuntu paths so the AppImage can launch on Arch/CachyOS and other non-Debian hosts.

| Mode | Command | Behavior |
|------|---------|----------|
| `wayland` (default) | `PMV_GFX=wayland ./…AppImage` | System Wayland preload, DMABUF **on**, AppDir GST plugins |
| `dmabuf-off` | `PMV_GFX=dmabuf-off ./…AppImage` | Preload + `WEBKIT_DISABLE_DMABUF_RENDERER=1` |
| `soft` | `PMV_GFX=soft ./…AppImage` | Preload + DMABUF + compositing off (slowest) |
| `stock` | `PMV_GFX=stock ./…AppImage` | No fixes (baseline / protocol-error repro) |

Startup prints which mode is active on stderr. Native (`dx serve`) installs still prefer host GST plugins for VAAPI/NVDEC when available.

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
