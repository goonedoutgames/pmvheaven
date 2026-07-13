"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { VideoSummary } from "@/lib/types";

/**
 * A Spotify-style ephemeral play queue. Videos can be appended ("Add to queue")
 * or inserted at the front ("Play next"). When a video finishes on the watch
 * page, the head of the queue is consumed and played next (autoplay).
 *
 * Persisted to localStorage so it survives full-page navigations between watch
 * pages, but it is never saved to the account (ephemeral).
 */

const STORAGE_KEY = "ph_queue_v1";

interface QueueState {
  queue: VideoSummary[];
  isOpen: boolean;
  setOpen: (open: boolean) => void;
  toggle: () => void;
  add: (video: VideoSummary) => void;
  playNext: (video: VideoSummary) => void;
  remove: (id: string) => void;
  move: (id: string, dir: -1 | 1) => void;
  /** Move the item at `from` to position `to` (drag-and-drop reordering). */
  reorder: (from: number, to: number) => void;
  clear: () => void;
  /** Remove and return the first queued item (used for autoplay). */
  shift: () => VideoSummary | null;
  /** Remove the item at id plus everything before it; return that item. */
  consumeTo: (id: string) => VideoSummary | null;
  /** Peek the next item without removing it. */
  peek: () => VideoSummary | null;
  isQueued: (id: string) => boolean;
}

const QueueContext = createContext<QueueState | null>(null);

export function QueueProvider({ children }: { children: React.ReactNode }) {
  const [queue, setQueue] = useState<VideoSummary[]>([]);
  const [isOpen, setOpen] = useState(false);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) setQueue(JSON.parse(raw));
    } catch {
      /* ignore */
    }
    setLoaded(true);
  }, []);

  useEffect(() => {
    if (!loaded) return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(queue));
    } catch {
      /* ignore */
    }
  }, [queue, loaded]);

  // Keep the queue in sync when another window (e.g. the separate player window)
  // changes it.
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY) {
        try {
          setQueue(e.newValue ? JSON.parse(e.newValue) : []);
        } catch {
          /* ignore */
        }
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const add = useCallback((video: VideoSummary) => {
    setQueue((q) => (q.some((v) => v.id === video.id) ? q : [...q, video]));
  }, []);

  const playNext = useCallback((video: VideoSummary) => {
    setQueue((q) => [video, ...q.filter((v) => v.id !== video.id)]);
  }, []);

  const remove = useCallback((id: string) => {
    setQueue((q) => q.filter((v) => v.id !== id));
  }, []);

  const move = useCallback((id: string, dir: -1 | 1) => {
    setQueue((q) => {
      const i = q.findIndex((v) => v.id === id);
      const j = i + dir;
      if (i < 0 || j < 0 || j >= q.length) return q;
      const next = [...q];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
  }, []);

  const reorder = useCallback((from: number, to: number) => {
    setQueue((q) => {
      if (from === to || from < 0 || to < 0 || from >= q.length || to >= q.length) {
        return q;
      }
      const next = [...q];
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return next;
    });
  }, []);

  const clear = useCallback(() => setQueue([]), []);

  // Reads the current queue synchronously (deps include `queue`) so callers
  // get the item immediately, then removes it.
  const shift = useCallback((): VideoSummary | null => {
    const next = queue.length ? queue[0] : null;
    if (next) setQueue((q) => q.slice(1));
    return next;
  }, [queue]);

  /** Remove the item at `id` plus everything before it; return that item. */
  const consumeTo = useCallback(
    (id: string): VideoSummary | null => {
      const idx = queue.findIndex((v) => v.id === id);
      if (idx < 0) return null;
      const target = queue[idx];
      setQueue((q) => q.slice(idx + 1));
      return target;
    },
    [queue],
  );

  const peek = useCallback(() => (queue.length ? queue[0] : null), [queue]);

  const isQueued = useCallback((id: string) => queue.some((v) => v.id === id), [queue]);

  const value = useMemo(
    () => ({
      queue,
      isOpen,
      setOpen,
      toggle: () => setOpen((o) => !o),
      add,
      playNext,
      remove,
      move,
      reorder,
      clear,
      shift,
      consumeTo,
      peek,
      isQueued,
    }),
    [queue, isOpen, add, playNext, remove, move, reorder, clear, shift, consumeTo, peek, isQueued],
  );

  return <QueueContext.Provider value={value}>{children}</QueueContext.Provider>;
}

export function useQueue(): QueueState {
  const ctx = useContext(QueueContext);
  if (!ctx) throw new Error("useQueue must be used within QueueProvider");
  return ctx;
}
