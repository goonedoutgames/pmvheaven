"use client";

import { useEffect, useRef } from "react";
import Hls from "hls.js";
import type { VideoDetail } from "@/lib/types";
import { streamProxyUrl } from "@/lib/format";

export function Player({ video }: { video: VideoDetail }) {
  const ref = useRef<HTMLVideoElement>(null);
  const lastReport = useRef(0);
  const counted = useRef(false);

  const useHls = video.hlsEnabled && !!video.hlsMasterPlaylistUrl;
  const src = streamProxyUrl(
    useHls ? video.hlsMasterPlaylistUrl! : video.videoUrl,
  );

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const resumeAt =
      video.watchProgress && video.watchProgress > 0 && video.watchProgress < 0.98
        ? video.watchProgress * (video.durationSeconds || 0)
        : 0;

    let hls: Hls | null = null;

    if (useHls && Hls.isSupported()) {
      hls = new Hls({ enableWorker: true, startPosition: resumeAt || -1 });
      hls.loadSource(src);
      hls.attachMedia(el);
    } else {
      // Native HLS (Safari) or progressive MP4.
      el.src = src;
      if (resumeAt) {
        const onMeta = () => {
          el.currentTime = resumeAt;
          el.removeEventListener("loadedmetadata", onMeta);
        };
        el.addEventListener("loadedmetadata", onMeta);
      }
    }

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

    const onPause = () => {
      if (el.duration) report(el.currentTime / el.duration, false);
    };

    el.addEventListener("timeupdate", onTime);
    el.addEventListener("pause", onPause);

    return () => {
      el.removeEventListener("timeupdate", onTime);
      el.removeEventListener("pause", onPause);
      if (el.duration && el.currentTime > 0) {
        report(el.currentTime / el.duration, false);
      }
      hls?.destroy();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [video.id, src, useHls]);

  return (
    <div className="relative w-full overflow-hidden rounded-xl border border-border bg-black">
      <video
        ref={ref}
        controls
        autoPlay
        playsInline
        poster={video.thumbnailUrl}
        className="aspect-video w-full bg-black"
      />
    </div>
  );
}
