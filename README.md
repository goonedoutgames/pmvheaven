# PMVHeaven

A sleek, ad-free alternative frontend for [PMVHaven](https://pmvhaven.com), built
on Next.js 16 (App Router, React 19, Tailwind 4). It talks to PMVHaven's own
backend API for browsing, playback, auth, favorites and watch-later, and mirrors
your **watch history into a local SQLite database** so it is kept **permanently**
— never subject to PMVHaven's rolling retention window.

## Features

- **Browse & discover** — trending, top-rated, newest, popular tags, infinite scroll.
- **Search** — full-text search when signed in (falls back to tag filtering when signed out).
- **Adaptive playback** — HLS via `hls.js` streamed through a same-origin proxy
  (PMVHaven's CDN sends no CORS headers), with resume-from-position and quality
  switching. Progressive MP4 fallback.
- **Permanent watch history** — on connect, the app snapshots your retained
  history into SQLite and keeps syncing, with live per-phase progress. Nothing
  is ever pruned locally, so it grows past PMVHaven's rolling limit over time.
- **Play queue** — a Spotify-style ephemeral queue: *Add to queue* / *Play next*
  from any card or the watch page, reorder in a slide-over panel, and each video
  autoplays the next one when it ends.
- **Favorites & Watch Later** — mirrored locally and written back to PMVHaven.
- **Desktop app** — ships as a native window via Tauri (Rust + system WebView).
- **No ads.**

## How it works

- **`src/lib/pmvhaven.ts`** — typed client for the PMVHaven REST API
  (`https://pmvhaven.com/api`). Auth uses Better Auth (`POST /auth/sign-in/email`);
  the resulting `better-auth.session_token` cookies are captured, encrypted, and
  reused for authenticated calls, with silent re-login on expiry.
- **`src/lib/db.ts` + `repo.ts`** — SQLite (`better-sqlite3`) storage for the
  account, cached video metadata, and the permanent history/favorites/watch-later
  tables.
- **`src/lib/sync.ts`** — imports watch history from the user object returned by
  `/auth/session` (`watchHistory` + `watchProgress`), hydrates the video IDs via
  the public `/videos/bulk` endpoint, and upserts everything locally. PMVHaven's
  documented `/user/watch-history` endpoint is broken server-side (500:
  "$slice path collision"), and free accounts only expose the most recent ~500
  entries of `watchHistoryCount`, so each sync captures that window; re-running
  over time accumulates a permanent archive that outgrows the rolling limit.
- **`src/components/QueueProvider.tsx`** — client-side ephemeral play queue,
  persisted to `localStorage` (never to the account) so it survives navigation.
- **`src/app/api/stream`** — HLS/media proxy that rewrites playlists and streams
  segments same-origin (also SSRF-guarded to PMVHaven's own media hosts).

Your PMVHaven credentials never reach the browser: they are sent once to
PMVHaven to obtain a session, then encrypted (AES-256-GCM) and stored locally.

## Getting started

```bash
pnpm install
pnpm dev        # http://localhost:3000
```

Open the app, click **Sign in**, and enter your PMVHaven email + password.
An initial history sync runs automatically; you can re-run **Sync history**
anytime from Settings or the History page (with live progress).

### Desktop app (Tauri)

The app is server-backed (SQLite, auth, stream proxy), so the desktop build ships
the Next.js **standalone** server and launches it as a child process, pointing a
native WebView window at it. The SQLite DB lives in the OS app-data dir.

```bash
# Dev: opens a native window wrapping `pnpm dev`
pnpm desktop:dev

# Release: builds the standalone server, then bundles (AppImage/.deb/etc.)
pnpm desktop:build
```

`pnpm desktop:dev` starts its own Next dev server on port 3000 — stop any
separate `pnpm dev` first to avoid a port clash. The `src-tauri/server/` dir
stays empty (just a `.keep`) except during a release build, when
`desktop:prepare` fills it with a flattened, self-contained copy of the
standalone server (symlinks dereferenced, dangling links skipped).

Requires the Rust toolchain and Tauri's system deps (`webkit2gtk-4.1`), plus
`node` available on the target machine to run the bundled server. Icons live in
`src-tauri/icons/` (regenerate with `pnpm tauri icon <path-to-1024.png>`).

### Environment

- `PH_SECRET` *(recommended in production)* — stable secret used to derive the
  encryption key for stored credentials/cookies. If unset, a random key is
  generated once and persisted in the DB.
- `PH_DATA_DIR` *(optional)* — directory for the SQLite database (default `./data`).

The `./data` directory (SQLite DB + secrets) is gitignored.

## Notes

This is a personal, self-hosted client for your own PMVHaven account. It stores
**your own** watch history in **your own** database. All content is served from
PMVHaven's infrastructure.
