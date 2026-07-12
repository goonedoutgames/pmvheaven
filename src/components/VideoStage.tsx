"use client";

import { useEffect, useRef } from "react";
import { Maximize, Minimize } from "lucide-react";
import { attachStream } from "@/lib/playback";
import { usePlayer, type MiniVideo } from "./PlayerProvider";
import { useQueue } from "./QueueProvider";

/**
 * The single <video> surface. Attaches the stream only when the *video* changes
 * (not on navigation), so progress survives page changes. Reports watch
 * progress and auto-advances through the queue when a clip ends. When the queue
 * is empty it simply stops on the last frame (controls stay usable) instead of
 * tearing the player down.
 */
export function VideoStage({
  className = "",
  fill = false,
}: {
  className?: string;
  /** Fill the container (letterboxed) instead of forcing an aspect-video box. */
  fill?: boolean;
}) {
  const { video, startAt, play, fullscreen, toggleFullscreen, reportTime } = usePlayer();
  const { shift } = useQueue();
  const videoRef = useRef<HTMLVideoElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const counted = useRef(false);
  const lastReport = useRef(0);
  const advanceRef = useRef<() => void>(() => {});

  // Always call the latest advance logic from the (stable) ended handler.
  useEffect(() => {
    advanceRef.current = async () => {
      const next = shift();
      if (!next) return; // nothing queued: leave the last frame + controls.
      try {
        const res = await fetch(`/api/video/${next.id}`, { cache: "no-store" });
        if (!res.ok) throw new Error();
        play((await res.json()) as MiniVideo, 0);
      } catch {
        /* ignore — keep current video */
      }
    };
  }, [shift, play]);

  useEffect(() => {
    const el = videoRef.current;
    if (!el || !video) return;
    counted.current = false;

    const detach = attachStream(el, video, startAt);
    void el.play?.().catch(() => {});

    const report = (progress: number, countView: boolean) => {
      void fetch("/api/view", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: video.id, progress, countView }),
        keepalive: true,
      }).catch(() => {});
    };

    const onTime = () => {
      if (!el.duration) return;
      reportTime(el.currentTime);
      const p = el.currentTime / el.duration;
      const now = Date.now();
      if (!counted.current && el.currentTime > 5) {
        counted.current = true;
        report(p, true);
        lastReport.current = now;
      } else if (now - lastReport.current > 15000) {
        lastReport.current = now;
        report(p, false);
      }
    };
    const onEnded = () => {
      report(1, false);
      advanceRef.current();
    };

    el.addEventListener("timeupdate", onTime);
    el.addEventListener("ended", onEnded);
    return () => {
      el.removeEventListener("timeupdate", onTime);
      el.removeEventListener("ended", onEnded);
      if (el.duration && el.currentTime > 0) {
        report(el.currentTime / el.duration, false);
      }
      detach();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [video?.id]);

  if (!video) return null;

  // Fill (letterboxed) when explicitly asked or while the player is fullscreen.
  const filled = fill || fullscreen;

  return (
    <div ref={wrapRef} className={`group relative bg-black ${className}`}>
      <video
        ref={videoRef}
        controls
        autoPlay
        playsInline
        poster={video.thumbnailUrl}
        className={`player-video bg-black ${
          filled ? "h-full w-full object-contain" : "aspect-video h-full w-full"
        }`}
      />
      <button
        onClick={toggleFullscreen}
        title={fullscreen ? "Exit fullscreen (Esc)" : "Fullscreen"}
        className="absolute right-2 top-2 z-10 grid h-7 w-7 place-items-center rounded-md bg-black/60 text-white opacity-0 backdrop-blur transition hover:bg-accent group-hover:opacity-100"
      >
        {fullscreen ? <Minimize size={15} /> : <Maximize size={15} />}
      </button>
    </div>
  );
}
