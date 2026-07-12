"use client";

import Link from "next/link";
import { useRef, useState } from "react";
import { Eye, Star } from "lucide-react";
import type { VideoSummary } from "@/lib/types";
import { formatDuration, formatViews, ratingColor, timeAgo } from "@/lib/format";
import { QueueButton } from "./QueueButton";

export function VideoCard({
  video,
  watchedProgress,
}: {
  video: VideoSummary;
  watchedProgress?: number;
}) {
  const [hover, setHover] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);

  const onEnter = () => {
    setHover(true);
    const el = videoRef.current;
    if (el && video.previewUrl) {
      el.currentTime = 0;
      void el.play().catch(() => {});
    }
  };
  const onLeave = () => {
    setHover(false);
    videoRef.current?.pause();
  };

  return (
    <Link
      href={`/watch/${video.id}`}
      className="group flex flex-col gap-2 animate-fade-in"
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
    >
      <div className="relative aspect-video overflow-hidden rounded-xl border border-border bg-surface">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src={video.thumbnailUrl}
          alt={video.title}
          loading="lazy"
          className={`h-full w-full object-cover transition duration-300 group-hover:scale-[1.03] ${
            hover && video.previewUrl ? "opacity-0" : "opacity-100"
          }`}
        />
        {video.previewUrl && (
          <video
            ref={videoRef}
            src={video.previewUrl}
            muted
            loop
            playsInline
            preload="none"
            className={`absolute inset-0 h-full w-full object-cover transition ${
              hover ? "opacity-100" : "opacity-0"
            }`}
          />
        )}

        <span className="absolute bottom-1.5 right-1.5 rounded bg-black/80 px-1.5 py-0.5 text-xs font-medium tabular-nums">
          {video.duration || formatDuration(video.durationSeconds)}
        </span>

        <div className="absolute right-1.5 top-1.5 opacity-0 transition group-hover:opacity-100">
          <QueueButton video={video} />
        </div>

        {video.rating > 0 && (
          <span
            className={`absolute left-1.5 top-1.5 flex items-center gap-0.5 rounded bg-black/80 px-1.5 py-0.5 text-xs font-semibold ${ratingColor(
              video.rating,
            )}`}
          >
            <Star size={11} className="fill-current" />
            {Math.round(video.rating)}%
          </span>
        )}

        {typeof watchedProgress === "number" && watchedProgress > 0 && (
          <span className="absolute inset-x-0 bottom-0 h-1 bg-black/50">
            <span
              className="block h-full bg-accent"
              style={{ width: `${Math.min(100, watchedProgress * 100)}%` }}
            />
          </span>
        )}
      </div>

      <div className="flex flex-col gap-0.5">
        <h3 className="line-clamp-2 text-sm font-semibold leading-snug transition group-hover:text-accent">
          {video.title}
        </h3>
        <p className="truncate text-xs text-muted">{video.uploaderUsername}</p>
        <p className="flex items-center gap-2 text-xs text-muted">
          <span className="flex items-center gap-1">
            <Eye size={12} />
            {formatViews(video.views)}
          </span>
          {video.uploadDate && <span>· {timeAgo(video.uploadDate)}</span>}
        </p>
      </div>
    </Link>
  );
}
