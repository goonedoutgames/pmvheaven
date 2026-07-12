"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { usePathname } from "next/navigation";
import {
  closeCurrentWindow,
  getPlayerChannel,
  NOW_PLAYING_KEY,
  openPlayerWindow,
  PLAYER_WINDOW_SETTING_KEY,
  readNowPlaying,
  setWindowFullscreen,
  writeNowPlaying,
  type MiniVideo,
  type NowPlaying,
  type PlayerMessage,
} from "@/lib/player";

export type { MiniVideo };

interface PlayerState {
  /** The video playing *in this window* (null in the main window when using a separate window). */
  video: MiniVideo | null;
  startAt: number;
  /** True in the dedicated player window/route. */
  isPlayerWindow: boolean;
  /** App-wide setting: play videos in a separate window instead of the in-app rail. */
  separateWindow: boolean;
  setSeparateWindow: (v: boolean) => void;
  /** True when playback is happening in the separate window (seen from the main window). */
  remoteActive: boolean;
  /** Player is filling the window (window-level fullscreen, painted in-flow). */
  fullscreen: boolean;
  toggleFullscreen: () => void;
  /** Report the current playback position (used for handoff + resume). */
  reportTime: (t: number) => void;
  /** From the player window: move playback back into the main window's rail. */
  returnToRail: () => void;
  /**
   * Start playing a video. Routes to the separate window when that setting is on
   * (and we're the main window); otherwise plays locally. Same video = no-op so
   * progress is preserved.
   */
  play: (video: MiniVideo, at?: number) => void;
  close: () => void;
}

const PlayerContext = createContext<PlayerState | null>(null);

export function PlayerProvider({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const isPlayerWindow = pathname === "/player";

  const [video, setVideo] = useState<MiniVideo | null>(null);
  const [startAt, setStartAt] = useState(0);
  const [separateWindow, setSeparateWindowState] = useState(false);
  const [remoteActive, setRemoteActive] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const idRef = useRef<string | null>(null);
  const channelRef = useRef<BroadcastChannel | null>(null);
  const currentTimeRef = useRef(0);
  const lastPersist = useRef(0);

  // Play locally in this window (bypasses the separate-window routing).
  const applyLocal = useCallback((v: MiniVideo | null, at = 0) => {
    if (v && idRef.current === v.id) return; // keep progress
    idRef.current = v?.id ?? null;
    setVideo(v);
    setStartAt(at);
  }, []);

  // Load the persisted separate-window setting and keep it in sync across windows.
  useEffect(() => {
    try {
      setSeparateWindowState(localStorage.getItem(PLAYER_WINDOW_SETTING_KEY) === "1");
    } catch {
      /* ignore */
    }
    const onStorage = (e: StorageEvent) => {
      if (e.key === PLAYER_WINDOW_SETTING_KEY) {
        setSeparateWindowState(e.newValue === "1");
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  // Fallback sync (when BroadcastChannel is unavailable): the player window
  // follows now-playing changes via storage events; the main window tracks
  // whether the separate window has something to play.
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key !== NOW_PLAYING_KEY) return;
      if (isPlayerWindow) {
        const np = e.newValue ? (JSON.parse(e.newValue) as NowPlaying) : null;
        applyLocal(np?.video ?? null, np?.at ?? 0);
      } else {
        setRemoteActive(!!e.newValue);
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, [isPlayerWindow, applyLocal]);

  // Cross-window command channel.
  useEffect(() => {
    const ch = getPlayerChannel();
    channelRef.current = ch;
    if (!ch) return;

    ch.onmessage = (e: MessageEvent<PlayerMessage>) => {
      const msg = e.data;
      if (isPlayerWindow) {
        if (msg.type === "play") applyLocal(msg.video, msg.at);
        else if (msg.type === "close") applyLocal(null);
      } else {
        // Main window.
        if (msg.type === "play") setRemoteActive(true);
        else if (msg.type === "close") setRemoteActive(false);
        else if (msg.type === "handoff") {
          // The player window handed control back to the in-app rail.
          setSeparateWindowState(false);
          setRemoteActive(false);
          applyLocal(msg.video, msg.at);
        }
      }
    };

    if (isPlayerWindow) {
      // Pick up whatever the main window last requested.
      const np = readNowPlaying();
      if (np) applyLocal(np.video, np.at);
      ch.postMessage({ type: "ready" } as PlayerMessage);
    } else {
      setRemoteActive(!!readNowPlaying());
    }

    return () => ch.close();
  }, [isPlayerWindow, applyLocal]);

  const play = useCallback(
    (v: MiniVideo, at = 0) => {
      if (separateWindow && !isPlayerWindow) {
        const np: NowPlaying = { video: v, at };
        writeNowPlaying(np);
        channelRef.current?.postMessage({ type: "play", video: v, at } as PlayerMessage);
        setRemoteActive(true);
        void openPlayerWindow();
        return;
      }
      applyLocal(v, at);
    },
    [separateWindow, isPlayerWindow, applyLocal],
  );

  const close = useCallback(() => {
    if (separateWindow && !isPlayerWindow) {
      writeNowPlaying(null);
      channelRef.current?.postMessage({ type: "close" } as PlayerMessage);
      setRemoteActive(false);
      return;
    }
    if (isPlayerWindow) writeNowPlaying(null);
    applyLocal(null);
  }, [separateWindow, isPlayerWindow, applyLocal]);

  // Keep now-playing current in the player window (including queue auto-advance)
  // so closing/reopening the window resumes the *current* clip near its spot.
  useEffect(() => {
    if (isPlayerWindow && video) writeNowPlaying({ video, at: 0 });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isPlayerWindow, video?.id]);

  const reportTime = useCallback(
    (t: number) => {
      currentTimeRef.current = t;
      if (isPlayerWindow && video) {
        const now = Date.now();
        if (now - lastPersist.current > 5000) {
          lastPersist.current = now;
          writeNowPlaying({ video, at: t });
        }
      }
    },
    [isPlayerWindow, video],
  );

  const toggleFullscreen = useCallback(() => {
    setFullscreen((f) => {
      const next = !f;
      void setWindowFullscreen(next);
      return next;
    });
  }, []);

  useEffect(() => {
    if (!fullscreen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setFullscreen(false);
        void setWindowFullscreen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [fullscreen]);

  const returnToRail = useCallback(() => {
    if (!isPlayerWindow) return;
    const at = currentTimeRef.current;
    try {
      localStorage.setItem(PLAYER_WINDOW_SETTING_KEY, "0");
    } catch {
      /* ignore */
    }
    if (video) {
      writeNowPlaying({ video, at });
      channelRef.current?.postMessage({ type: "handoff", video, at } as PlayerMessage);
    }
    applyLocal(null);
    void setWindowFullscreen(false);
    void closeCurrentWindow();
  }, [isPlayerWindow, video, applyLocal]);

  const setSeparateWindow = useCallback(
    (v: boolean) => {
      try {
        localStorage.setItem(PLAYER_WINDOW_SETTING_KEY, v ? "1" : "0");
      } catch {
        /* ignore */
      }
      setSeparateWindowState(v);

      // Migrate whatever is currently playing to the new destination, resuming
      // at the *live* playback position (not where the clip was first loaded).
      if (v) {
        if (video) {
          const at = currentTimeRef.current || startAt;
          const np: NowPlaying = { video, at };
          writeNowPlaying(np);
          channelRef.current?.postMessage({ type: "play", video, at } as PlayerMessage);
          setRemoteActive(true);
          void openPlayerWindow();
          applyLocal(null);
        }
      } else {
        const np = readNowPlaying();
        channelRef.current?.postMessage({ type: "close" } as PlayerMessage);
        setRemoteActive(false);
        if (np) applyLocal(np.video, np.at);
      }
    },
    [video, startAt, applyLocal],
  );

  const value = useMemo(
    () => ({
      video,
      startAt,
      isPlayerWindow,
      separateWindow,
      setSeparateWindow,
      remoteActive,
      fullscreen,
      toggleFullscreen,
      reportTime,
      returnToRail,
      play,
      close,
    }),
    [
      video,
      startAt,
      isPlayerWindow,
      separateWindow,
      setSeparateWindow,
      remoteActive,
      fullscreen,
      toggleFullscreen,
      reportTime,
      returnToRail,
      play,
      close,
    ],
  );

  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
}

export function usePlayer(): PlayerState {
  const ctx = useContext(PlayerContext);
  if (!ctx) throw new Error("usePlayer must be used within PlayerProvider");
  return ctx;
}
