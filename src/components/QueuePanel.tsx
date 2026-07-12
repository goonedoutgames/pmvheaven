"use client";

import { useRouter } from "next/navigation";
import { ChevronDown, ChevronUp, ListVideo, Play, Trash2, X } from "lucide-react";
import { useQueue } from "./QueueProvider";
import { usePlayer, type MiniVideo } from "./PlayerProvider";
import { formatDuration } from "@/lib/format";

export function QueuePanel() {
  const { queue, isOpen, setOpen, remove, move, clear, consumeTo } = useQueue();
  const { play } = usePlayer();
  const router = useRouter();

  const playFrom = async (id: string) => {
    // Consume everything up to and including the chosen item, then play it in
    // the persistent player (rail / separate window), without leaving the page.
    const target = consumeTo(id);
    if (!target) return;
    setOpen(false);
    try {
      const res = await fetch(`/api/video/${target.id}`, { cache: "no-store" });
      if (!res.ok) throw new Error();
      play((await res.json()) as MiniVideo, 0);
    } catch {
      router.push(`/watch/${target.id}`);
    }
  };

  if (!isOpen) return null;

  return (
    <>
      <div
        className="fixed inset-0 z-[60] bg-black/50 backdrop-blur-sm"
        onClick={() => setOpen(false)}
      />
      <aside
        className="fixed right-0 z-[70] flex w-full max-w-md flex-col border-l border-border bg-surface shadow-2xl animate-fade-in"
        style={{ top: "var(--titlebar-h)", height: "calc(100% - var(--titlebar-h))" }}
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-4">
          <h2 className="flex items-center gap-2 text-lg font-bold">
            <ListVideo size={20} /> Queue
            <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs font-medium text-muted">
              {queue.length}
            </span>
          </h2>
          <div className="flex items-center gap-1">
            {queue.length > 0 && (
              <button
                onClick={clear}
                className="rounded-lg px-2.5 py-1.5 text-xs text-muted transition hover:bg-surface-2 hover:text-rose-400"
              >
                Clear
              </button>
            )}
            <button
              onClick={() => setOpen(false)}
              className="rounded-lg p-2 text-muted transition hover:bg-surface-2 hover:text-foreground"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-3">
          {queue.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-center text-muted">
              <ListVideo size={40} className="opacity-40" />
              <p className="font-medium">Your queue is empty</p>
              <p className="text-sm">
                Add videos with the <strong>+</strong> button to build a play
                queue. They&apos;ll autoplay one after another.
              </p>
            </div>
          ) : (
            <ul className="flex flex-col gap-2">
              {queue.map((v, i) => (
                <li
                  key={v.id}
                  className="group flex gap-3 rounded-xl border border-transparent p-2 transition hover:border-border hover:bg-surface-2"
                >
                  <button
                    onClick={() => playFrom(v.id)}
                    className="relative aspect-video w-28 shrink-0 overflow-hidden rounded-lg bg-surface-2"
                  >
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img
                      src={v.thumbnailUrl}
                      alt={v.title}
                      className="h-full w-full object-cover"
                    />
                    <span className="absolute inset-0 grid place-items-center bg-black/40 opacity-0 transition group-hover:opacity-100">
                      <Play size={20} className="fill-white" />
                    </span>
                    <span className="absolute bottom-1 right-1 rounded bg-black/80 px-1 text-[10px] tabular-nums">
                      {v.duration || formatDuration(v.durationSeconds)}
                    </span>
                  </button>

                  <div className="flex min-w-0 flex-1 flex-col justify-center">
                    <p className="line-clamp-2 text-sm font-medium leading-snug">
                      {v.title}
                    </p>
                    <p className="truncate text-xs text-muted">{v.uploaderUsername}</p>
                  </div>

                  <div className="flex flex-col items-center justify-center gap-0.5 opacity-0 transition group-hover:opacity-100">
                    <button
                      onClick={() => move(v.id, -1)}
                      disabled={i === 0}
                      className="rounded p-1 text-muted transition hover:text-foreground disabled:opacity-30"
                    >
                      <ChevronUp size={16} />
                    </button>
                    <button
                      onClick={() => move(v.id, 1)}
                      disabled={i === queue.length - 1}
                      className="rounded p-1 text-muted transition hover:text-foreground disabled:opacity-30"
                    >
                      <ChevronDown size={16} />
                    </button>
                  </div>
                  <button
                    onClick={() => remove(v.id)}
                    className="self-center rounded p-1.5 text-muted opacity-0 transition hover:text-rose-400 group-hover:opacity-100"
                  >
                    <Trash2 size={15} />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </aside>
    </>
  );
}
