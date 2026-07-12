"use client";

import { Check, ListPlus } from "lucide-react";
import { useQueue } from "./QueueProvider";
import type { VideoSummary } from "@/lib/types";

/** Compact icon button used on video cards. */
export function QueueButton({ video }: { video: VideoSummary }) {
  const { add, isQueued } = useQueue();
  const queued = isQueued(video.id);

  return (
    <button
      title={queued ? "In queue" : "Add to queue"}
      onClick={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (!queued) add(video);
      }}
      className={`grid h-8 w-8 place-items-center rounded-lg backdrop-blur transition ${
        queued
          ? "bg-accent/90 text-white"
          : "bg-black/60 text-white hover:bg-accent/90"
      }`}
    >
      {queued ? <Check size={16} /> : <ListPlus size={16} />}
    </button>
  );
}

/** Full-width labelled button used on the watch page. */
export function QueueButtonLabeled({ video }: { video: VideoSummary }) {
  const { add, playNext, isQueued } = useQueue();
  const queued = isQueued(video.id);

  return (
    <div className="flex gap-2">
      <button
        onClick={() => (queued ? null : add(video))}
        className={`flex items-center gap-2 rounded-lg border px-4 py-2 text-sm font-medium transition ${
          queued
            ? "border-accent bg-accent/15 text-accent"
            : "border-border bg-surface text-foreground hover:bg-surface-2"
        }`}
      >
        {queued ? <Check size={16} /> : <ListPlus size={16} />}
        {queued ? "In queue" : "Add to queue"}
      </button>
      <button
        onClick={() => playNext(video)}
        className="flex items-center gap-2 rounded-lg border border-border bg-surface px-4 py-2 text-sm font-medium text-foreground transition hover:bg-surface-2"
      >
        Play next
      </button>
    </div>
  );
}
