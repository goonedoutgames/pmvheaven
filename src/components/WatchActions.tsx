"use client";

import { useState } from "react";
import { Clock, Heart } from "lucide-react";
import { useSession } from "./SessionProvider";
import type { VideoDetail } from "@/lib/types";

export function WatchActions({ video }: { video: VideoDetail }) {
  const { authenticated } = useSession();
  const [fav, setFav] = useState(!!video.isFavorited);
  const [later, setLater] = useState(!!video.isWatchLater);
  const [busy, setBusy] = useState<"fav" | "later" | null>(null);

  const toggle = async (kind: "fav" | "later") => {
    if (!authenticated || busy) return;
    setBusy(kind);
    const on = kind === "fav" ? !fav : !later;
    const endpoint = kind === "fav" ? "/api/favorites" : "/api/watch-later";
    // Optimistic update.
    if (kind === "fav") setFav(on);
    else setLater(on);
    try {
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: video.id, on }),
      });
      if (!res.ok) throw new Error();
    } catch {
      // revert on failure
      if (kind === "fav") setFav(!on);
      else setLater(!on);
    } finally {
      setBusy(null);
    }
  };

  if (!authenticated) return null;

  return (
    <div className="flex flex-wrap gap-2">
      <button
        onClick={() => toggle("fav")}
        disabled={busy === "fav"}
        className={`flex items-center gap-2 rounded-lg border px-4 py-2 text-sm font-medium transition disabled:opacity-60 ${
          fav
            ? "border-accent bg-accent/15 text-accent"
            : "border-border bg-surface text-foreground hover:bg-surface-2"
        }`}
      >
        <Heart size={16} className={fav ? "fill-current" : ""} />
        {fav ? "Favorited" : "Favorite"}
      </button>
      <button
        onClick={() => toggle("later")}
        disabled={busy === "later"}
        className={`flex items-center gap-2 rounded-lg border px-4 py-2 text-sm font-medium transition disabled:opacity-60 ${
          later
            ? "border-accent-2 bg-accent-2/15 text-accent-2"
            : "border-border bg-surface text-foreground hover:bg-surface-2"
        }`}
      >
        <Clock size={16} />
        {later ? "Saved" : "Watch later"}
      </button>
    </div>
  );
}
