"use client";

import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";
import { ListPlus, Play, X } from "lucide-react";
import type { VideoSummary } from "@/lib/types";
import { usePlayer, type MiniVideo } from "./PlayerProvider";
import { useQueue } from "./QueueProvider";

interface PlayChoiceState {
  /** Ask the user whether to play `video` now (replacing the current one) or queue it. */
  request: (video: VideoSummary) => void;
}

const PlayChoiceContext = createContext<PlayChoiceState | null>(null);

export function PlayChoiceProvider({ children }: { children: React.ReactNode }) {
  const { video: current, play } = usePlayer();
  const { add } = useQueue();
  const [pending, setPending] = useState<VideoSummary | null>(null);
  const [loading, setLoading] = useState(false);

  const request = useCallback((video: VideoSummary) => {
    setPending(video);
  }, []);

  const close = useCallback(() => {
    setPending(null);
    setLoading(false);
  }, []);

  const playNow = useCallback(async () => {
    if (!pending) return;
    setLoading(true);
    try {
      const res = await fetch(`/api/video/${pending.id}`, { cache: "no-store" });
      if (!res.ok) throw new Error();
      play((await res.json()) as MiniVideo, 0);
    } catch {
      /* ignore — leave current video playing */
    } finally {
      close();
    }
  }, [pending, play, close]);

  const addToQueue = useCallback(() => {
    if (pending) add(pending);
    close();
  }, [pending, add, close]);

  const value = useMemo(() => ({ request }), [request]);

  return (
    <PlayChoiceContext.Provider value={value}>
      {children}
      {pending && (
        <div
          className="fixed inset-0 z-90 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm animate-fade-in"
          onClick={close}
        >
          <div
            className="w-full max-w-sm overflow-hidden rounded-2xl border border-border bg-surface shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="relative">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={pending.thumbnailUrl}
                alt={pending.title}
                className="aspect-video w-full object-cover"
              />
              <button
                onClick={close}
                aria-label="Cancel"
                className="absolute right-2 top-2 grid h-8 w-8 place-items-center rounded-lg bg-black/60 text-white backdrop-blur transition hover:bg-black/80"
              >
                <X size={16} />
              </button>
            </div>

            <div className="flex flex-col gap-4 p-4">
              <div className="flex flex-col gap-1">
                <p className="line-clamp-2 text-sm font-semibold leading-snug">
                  {pending.title}
                </p>
                {current && (
                  <p className="line-clamp-1 text-xs text-muted">
                    Currently playing: {current.title}
                  </p>
                )}
              </div>

              <div className="flex flex-col gap-2">
                <button
                  onClick={playNow}
                  disabled={loading}
                  className="flex items-center justify-center gap-2 rounded-lg bg-accent px-4 py-2.5 text-sm font-semibold text-white transition hover:bg-accent/90 disabled:opacity-60"
                >
                  <Play size={16} className="fill-current" />
                  {loading ? "Loading…" : "Play over current"}
                </button>
                <button
                  onClick={addToQueue}
                  disabled={loading}
                  className="flex items-center justify-center gap-2 rounded-lg border border-border bg-surface-2 px-4 py-2.5 text-sm font-medium text-foreground transition hover:bg-surface disabled:opacity-60"
                >
                  <ListPlus size={16} />
                  Add to queue
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </PlayChoiceContext.Provider>
  );
}

export function usePlayChoice(): PlayChoiceState {
  const ctx = useContext(PlayChoiceContext);
  if (!ctx) throw new Error("usePlayChoice must be used within PlayChoiceProvider");
  return ctx;
}
