"use client";

import { useEffect } from "react";
import { ExternalLink, Play } from "lucide-react";
import type { VideoDetail } from "@/lib/types";
import { openPlayerWindow } from "@/lib/player";
import { usePlayer } from "./PlayerProvider";

/**
 * On the watch page the video itself plays in the persistent rail / separate
 * window — not inline — so navigating away never interrupts it. This kicks off
 * playback and shows a poster that links to wherever the video is playing.
 */
export function WatchAutoplay({ video }: { video: VideoDetail }) {
  const { play, separateWindow, video: current } = usePlayer();

  useEffect(() => {
    const resumeAt =
      video.watchProgress && video.watchProgress > 0 && video.watchProgress < 0.98
        ? video.watchProgress * (video.durationSeconds || 0)
        : 0;
    play(
      {
        id: video.id,
        title: video.title,
        thumbnailUrl: video.thumbnailUrl,
        videoUrl: video.videoUrl,
        hlsEnabled: video.hlsEnabled,
        hlsMasterPlaylistUrl: video.hlsMasterPlaylistUrl,
        durationSeconds: video.durationSeconds,
      },
      resumeAt,
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [video.id]);

  const isCurrent = current?.id === video.id;

  return (
    <button
      onClick={() => (separateWindow ? void openPlayerWindow() : undefined)}
      className="group relative block aspect-video w-full overflow-hidden rounded-xl border border-border bg-black"
    >
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={video.thumbnailUrl}
        alt={video.title}
        className="h-full w-full object-cover opacity-60 transition group-hover:opacity-40"
      />
      <span className="absolute inset-0 flex flex-col items-center justify-center gap-2 text-white">
        <span className="grid h-14 w-14 place-items-center rounded-full bg-accent/90 shadow-lg">
          {separateWindow ? <ExternalLink size={24} /> : <Play size={24} className="fill-white" />}
        </span>
        <span className="text-sm font-medium">
          {separateWindow
            ? "Playing in player window"
            : isCurrent
              ? "Playing in the player panel"
              : "Starting…"}
        </span>
      </span>
    </button>
  );
}
