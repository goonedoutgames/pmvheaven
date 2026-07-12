"use client";

import { ListVideo, PanelRight } from "lucide-react";
import { usePlayer } from "./PlayerProvider";
import { useQueue } from "./QueueProvider";
import { VideoStage } from "./VideoStage";
import { QueuePanel } from "./QueuePanel";

/** Full-window player used inside the dedicated player window (route `/player`). */
export function PlayerWindow() {
  const { video, fullscreen, returnToRail } = usePlayer();
  const { queue, toggle } = useQueue();

  return (
    <div className="flex h-full min-h-0 flex-col bg-black text-foreground">
      <div className="relative flex min-h-0 flex-1 items-center justify-center">
        {video ? (
          <VideoStage fill className="h-full w-full" />
        ) : (
          <div className="flex flex-col items-center gap-2 text-muted">
            <ListVideo size={40} className="opacity-40" />
            <p className="text-sm">Nothing playing. Pick a video in the main window.</p>
          </div>
        )}
      </div>

      {!fullscreen && (
        <div className="flex items-center gap-3 border-t border-border bg-background px-4 py-2.5">
          <p className="line-clamp-1 flex-1 text-sm font-medium">
            {video?.title ?? "PMVHeaven Player"}
          </p>
          <button
            onClick={toggle}
            title="Queue"
            className="relative flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-sm text-muted transition hover:bg-surface-2 hover:text-foreground"
          >
            <ListVideo size={16} /> Queue
            {queue.length > 0 && (
              <span className="grid h-4 min-w-4 place-items-center rounded-full bg-accent px-1 text-[10px] font-bold text-white">
                {queue.length}
              </span>
            )}
          </button>
          <button
            onClick={returnToRail}
            title="Move playback back into the app"
            className="flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1.5 text-sm text-muted transition hover:bg-surface-2 hover:text-foreground"
          >
            <PanelRight size={16} /> Return to app
          </button>
        </div>
      )}

      <QueuePanel />
    </div>
  );
}
