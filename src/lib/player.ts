"use client";

/** Minimal fields the player needs to stream a video. */
export interface MiniVideo {
  id: string;
  title: string;
  thumbnailUrl: string;
  videoUrl: string;
  hlsEnabled?: boolean;
  hlsMasterPlaylistUrl?: string | null;
  durationSeconds?: number;
}

export const NOW_PLAYING_KEY = "ph_now_playing_v1";
export const PLAYER_WINDOW_SETTING_KEY = "ph_player_window_v1";
export const PLAYER_CHANNEL = "ph_player";
export const PLAYER_WINDOW_LABEL = "player";

export type PlayerMessage =
  | { type: "play"; video: MiniVideo; at: number }
  | { type: "close" }
  | { type: "ready" }
  /** Player window → main window: take playback back into the in-app rail. */
  | { type: "handoff"; video: MiniVideo; at: number };

export interface NowPlaying {
  video: MiniVideo;
  at: number;
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function getPlayerChannel(): BroadcastChannel | null {
  if (typeof window === "undefined" || !("BroadcastChannel" in window)) return null;
  return new BroadcastChannel(PLAYER_CHANNEL);
}

export function readNowPlaying(): NowPlaying | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = localStorage.getItem(NOW_PLAYING_KEY);
    return raw ? (JSON.parse(raw) as NowPlaying) : null;
  } catch {
    return null;
  }
}

export function writeNowPlaying(np: NowPlaying | null) {
  if (typeof window === "undefined") return;
  try {
    if (np) localStorage.setItem(NOW_PLAYING_KEY, JSON.stringify(np));
    else localStorage.removeItem(NOW_PLAYING_KEY);
  } catch {
    /* ignore */
  }
}

/** Open (or focus) the dedicated player window — a native window in Tauri, a popup in the browser. */
export async function openPlayerWindow() {
  if (typeof window === "undefined") return;
  // Load from the same origin as the main window so it works in dev and behind
  // the production sidecar server alike.
  const url = new URL("/player", window.location.origin).toString();
  if (isTauri()) {
    try {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const existing = await WebviewWindow.getByLabel(PLAYER_WINDOW_LABEL);
      if (existing) {
        await existing.setFocus().catch(() => {});
        return;
      }
      new WebviewWindow(PLAYER_WINDOW_LABEL, {
        url,
        title: "PMVHeaven — Player",
        width: 1000,
        height: 640,
        minWidth: 480,
        minHeight: 320,
        resizable: true,
        // Use our custom titlebar (WindowChrome), like the main window.
        decorations: false,
      });
    } catch {
      /* ignore */
    }
  } else {
    window.open(url, PLAYER_WINDOW_LABEL, "width=1000,height=640");
  }
}

/** Close the current window (used by the player window to return to the app). */
export async function closeCurrentWindow() {
  if (isTauri()) {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      /* ignore */
    }
  } else if (typeof window !== "undefined") {
    window.close();
  }
}

/** Best-effort toggle of the current OS window's fullscreen state (Tauri only). */
export async function setWindowFullscreen(on: boolean) {
  if (!isTauri()) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setFullscreen(on);
  } catch {
    /* ignore */
  }
}
