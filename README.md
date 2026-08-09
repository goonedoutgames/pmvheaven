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

### End users (Linux)

- [Flatpak](https://flatpak.org/) + Flathub remote (for `org.gnome.Platform`)

### Developers

- Rust stable (1.85+)
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started): `cargo install dioxus-cli`
- **Linux:** WebKitGTK (`webkit2gtk`), GTK3, and `xdotool` (provides `libxdo` — required to link Dioxus desktop)
- **Windows:** WebView2 runtime

On Arch/CachyOS (dev native builds):

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 xdotool base-devel
```

## Develop

```bash
dx serve --platform desktop
# or
cargo run
```

## Linux Flatpak (supported distribution)

Released Linux builds are **Flatpak** (`com.pmvheaven.Desktop`), built inside `org.gnome.Sdk` so WebKitGTK and GStreamer match the runtime on every distro (Arch/CachyOS included).

### One-time: Flathub + GNOME Platform 50

The bundle needs `org.gnome.Platform//50` from Flathub (not shipped inside the `.flatpak` file):

```bash
# Arch/CachyOS: sudo pacman -S flatpak
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y flathub org.gnome.Platform//50
```

### Install the app bundle

```bash
flatpak install --user -y ./PMVHeaven-<version>-x86_64.flatpak
flatpak run com.pmvheaven.Desktop
```

If a GUI installer fails with “requires the runtime org.gnome.Platform/… which was not found”, run the Flathub/Platform commands above, then retry.

**Already installed an older build that crashes on launch** (`cannot open display` / `readPIDFromPeer`)? That needs a **new Flatpak build** — Dioxus was forcing `GDK_BACKEND=x11` while Flatpak often has no X11 display. Overrides alone cannot fix it. Install the latest CI/release artifact after it finishes building.

App data lives under the Flatpak sandbox, e.g.
`~/.var/app/com.pmvheaven.Desktop/data/com.pmvheaven.desktop/`.

### Local Flatpak build

```bash
./scripts/gen-cargo-sources.sh
flatpak-builder --user --install --force-clean build-dir \
  packaging/flatpak/com.pmvheaven.Desktop.yml
flatpak run com.pmvheaven.Desktop
```

### Packaging test CI

Push to branch `ci/linux-packaging` (or run **Linux packaging** via workflow_dispatch) to build:

- **Flatpak** (primary) — download and smoke-test on your machine
- **AppImage** (comparison only, `*-COMPARE.AppImage`) — not supported for release

## Bundle (developers)

```bash
dx bundle --platform desktop --release
```

**Windows** (CI publishes these): NSIS installer under `target/dx/…` (e.g. `*-setup.exe`).

**Linux AppImage** is optional/comparison-only after `dx bundle`; post-process with `./scripts/fix-appimage-wayland.sh`. Prefer Flatpak for anything you ship.

## Releases (CI)

Pushes to `main` run [`.github/workflows/release.yml`](.github/workflows/release.yml):

1. Read semver from `Cargo.toml` (`2.2.7` → tag `v2.2.7`)
2. Skip if that GitHub Release already exists (bump the Cargo version to cut a new one)
3. Build **Linux Flatpak** + **Windows NSIS `.exe`**
4. Publish a GitHub Release with both artifacts

Manual re-run: Actions → **Release** → **Run workflow** (optionally force recreate).

On launch the app checks `goonedoutgames/pmvheaven` for a newer release and offers a download link when one is available (Linux prefers `.flatpak`).

### Linux graphics A/B (`PMV_GFX`) — native / AppImage only

Inside Flatpak these workarounds are skipped. For `dx serve` or a local AppImage on rolling Wayland:

| Mode | Command | Behavior |
|------|---------|----------|
| `wayland` (default) | `PMV_GFX=wayland ./…AppImage` | System Wayland preload, DMABUF **on** |
| `dmabuf-off` | `PMV_GFX=dmabuf-off ./…AppImage` | Preload + `WEBKIT_DISABLE_DMABUF_RENDERER=1` |
| `soft` | `PMV_GFX=soft ./…AppImage` | Preload + DMABUF + compositing off (slowest) |
| `stock` | `PMV_GFX=stock ./…AppImage` | No fixes (baseline / protocol-error repro) |

## Data

Native/`dx serve` app data lives under the OS data dir for identifier `com.pmvheaven.desktop`
(e.g. `~/.local/share/com.pmvheaven.desktop` on Linux):

| File | Purpose |
|------|---------|
| `pmvheaven_v2.db` | v2 SQLite (account, videos, history, favorites, …) |
| `queue.json` | Ephemeral play queue |
| `pmvheaven.db` | **Legacy v1 only** — if present on first launch, the app prompts to delete it |

Flatpak uses the same relative layout under `~/.var/app/com.pmvheaven.Desktop/data/`.

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
packaging/flatpak/  # Flatpak manifest + desktop/metainfo
public/       # logo.png + sexy_close.svg (brand copies)
```

## Auth note

Sign-in uses PMVHaven Better Auth (`POST /auth/sign-in/email`). Session cookies are encrypted at rest. Watch history is pulled from `/auth/session` (`watchHistory` + `watchProgress`) because `/user/watch-history` is broken server-side.
