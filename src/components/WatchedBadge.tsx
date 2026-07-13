"use client";

import { Check } from "lucide-react";
import { useWatched } from "./WatchedProvider";

/** Inline "Watched" pill shown when the given video is in the user's history. */
export function WatchedBadge({ id, className = "" }: { id: string; className?: string }) {
  const { isWatched } = useWatched();
  if (!isWatched(id)) return null;
  return (
    <span
      className={`inline-flex w-fit items-center gap-1 rounded bg-accent px-2 py-0.5 text-xs font-bold uppercase tracking-wide text-white ${className}`}
    >
      <Check size={12} strokeWidth={3} />
      Watched
    </span>
  );
}
