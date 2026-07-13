"use client";

import { useRouter } from "next/navigation";
import {
  ExternalLink,
  GripVertical,
  ListVideo,
  MonitorPlay,
  Play,
  SquareArrowOutUpRight,
  Trash2,
  X,
} from "lucide-react";
import { formatDuration } from "@/lib/format";
import { openPlayerWindow, type MiniVideo } from "@/lib/player";
import { usePlayer } from "./PlayerProvider";
import { useQueue } from "./QueueProvider";
import { useListDrag } from "./useListDrag";
import { VideoStage } from "./VideoStage";

export function PlayerRail() {
  const {
    video,
    isPlayerWindow,
    separateWindow,
    remoteActive,
    fullscreen,
    close,
    setSeparateWindow,
    play,
  } = usePlayer();
  const { queue, remove, reorder, consumeTo, clear } = useQueue();
  const router = useRouter();

  // Pointer-based drag reordering (HTML5 DnD is unreliable in WebKitGTK).
  const { dragIndex, overIndex, start } = useListDrag(reorder);

  // The dedicated player window renders its own player, not this rail.
  if (isPlayerWindow) return null;

  // Separate-window mode: no in-app player, just a status chip while it plays.
  if (separateWindow) {
    if (!remoteActive) return null;
    return (
      <div className="fixed bottom-4 left-4 z-70 flex items-center gap-2 rounded-full border border-border bg-surface/95 px-3 py-2 text-sm shadow-2xl backdrop-blur">
        <span className="flex h-2 w-2 animate-pulse rounded-full bg-accent" />
        <span className="text-muted">Playing in player window</span>
        <button
          onClick={() => void openPlayerWindow()}
          title="Focus player window"
          className="rounded-md p-1 text-muted transition hover:bg-surface-2 hover:text-foreground"
        >
          <ExternalLink size={15} />
        </button>
        <button
          onClick={close}
          title="Stop"
          className="rounded-md p-1 text-muted transition hover:bg-surface-2 hover:text-rose-400"
        >
          <X size={15} />
        </button>
      </div>
    );
  }

  // In-app docked rail — content reflows to make room.
  if (!video) return null;

  const playFrom = async (id: string) => {
    const target = consumeTo(id);
    if (!target) return;
    try {
      const res = await fetch(`/api/video/${target.id}`, { cache: "no-store" });
      if (!res.ok) throw new Error();
      play((await res.json()) as MiniVideo, 0);
    } catch {
      router.push(`/watch/${target.id}`);
    }
  };

  // While fullscreen the rail fills the whole window (content column is hidden).
  if (fullscreen) {
    return (
      <aside className="flex min-h-0 flex-1 items-center justify-center bg-black">
        <VideoStage fill className="h-full w-full" />
      </aside>
    );
  }

  return (
    <aside className="order-1 flex w-full shrink-0 flex-col border-b border-border bg-surface lg:order-2 lg:h-full lg:w-[clamp(380px,40vw,960px)] lg:overflow-y-auto lg:border-b-0 lg:border-l">
      <VideoStage className="w-full" />

      <div className="flex items-start gap-2 p-3">
        <p className="line-clamp-2 flex-1 text-sm font-semibold leading-snug">{video.title}</p>
        <button
          onClick={() => router.push(`/watch/${video.id}`)}
          title="Open video page"
          className="shrink-0 rounded-md p-1.5 text-muted transition hover:bg-surface-2 hover:text-foreground"
        >
          <SquareArrowOutUpRight size={16} />
        </button>
        <button
          onClick={() => setSeparateWindow(true)}
          title="Play in a separate window"
          className="shrink-0 rounded-md p-1.5 text-muted transition hover:bg-surface-2 hover:text-foreground"
        >
          <MonitorPlay size={16} />
        </button>
        <button
          onClick={close}
          title="Close player"
          className="shrink-0 rounded-md p-1.5 text-muted transition hover:bg-surface-2 hover:text-rose-400"
        >
          <X size={16} />
        </button>
      </div>

      {/* Up next / queue (desktop rail only) */}
      <div className="hidden min-h-0 flex-1 flex-col border-t border-border lg:flex">
        <div className="flex items-center justify-between px-3 py-2">
          <p className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted">
            <ListVideo size={14} /> Up next
            <span className="rounded-full bg-surface-2 px-1.5 py-0.5 text-[10px] normal-case tracking-normal">
              {queue.length}
            </span>
          </p>
          {queue.length > 0 && (
            <button
              onClick={clear}
              className="rounded-md px-2 py-1 text-[11px] text-muted transition hover:bg-surface-2 hover:text-rose-400"
            >
              Clear
            </button>
          )}
        </div>
        <ul className="flex flex-col gap-1.5 overflow-y-auto px-2 pb-3">
          {queue.length === 0 && (
            <li className="px-2 py-6 text-center text-xs text-muted">
              Nothing queued. Add videos with the <strong>+</strong> button.
            </li>
          )}
          {queue.map((v, i) => (
            <li
              key={v.id}
              data-drag-index={i}
              className={`group flex gap-1.5 rounded-lg border p-1.5 transition ${
                dragIndex === i
                  ? "border-border bg-surface-2 opacity-50"
                  : overIndex === i && dragIndex !== null
                    ? "border-accent bg-surface-2"
                    : "border-transparent hover:border-border hover:bg-surface-2"
              }`}
            >
              <button
                onPointerDown={(e) => start(i, e)}
                aria-label="Drag to reorder"
                className="flex shrink-0 touch-none cursor-grab items-center self-stretch text-muted transition hover:text-foreground active:cursor-grabbing"
              >
                <GripVertical size={14} />
              </button>
              <button
                onClick={() => playFrom(v.id)}
                className="relative aspect-video w-24 shrink-0 overflow-hidden rounded-md bg-surface-2"
              >
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img
                  src={v.thumbnailUrl}
                  alt={v.title}
                  draggable={false}
                  className="h-full w-full object-cover"
                />
                <span className="absolute inset-0 grid place-items-center bg-black/40 opacity-0 transition group-hover:opacity-100">
                  <Play size={16} className="fill-white" />
                </span>
                <span className="absolute bottom-0.5 right-0.5 rounded bg-black/80 px-1 text-[9px] tabular-nums">
                  {v.duration || formatDuration(v.durationSeconds)}
                </span>
              </button>
              <div className="flex min-w-0 flex-1 flex-col justify-center">
                <p className="line-clamp-2 text-xs font-medium leading-snug">{v.title}</p>
                <p className="truncate text-[11px] text-muted">{v.uploaderUsername}</p>
              </div>
              <button
                onClick={() => remove(v.id)}
                title="Remove"
                className="self-center rounded p-1 text-muted opacity-0 transition hover:text-rose-400 group-hover:opacity-100"
              >
                <Trash2 size={14} />
              </button>
            </li>
          ))}
        </ul>
      </div>
    </aside>
  );
}
