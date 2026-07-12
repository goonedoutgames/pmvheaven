"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useSession } from "./SessionProvider";
import { VideoGrid, GridSkeleton } from "./VideoGrid";
import type { VideoSummary } from "@/lib/types";

export function LibraryList({
  endpoint,
  title,
  emptyLabel,
}: {
  endpoint: string;
  title: string;
  emptyLabel: string;
}) {
  const { authenticated, loading: sessionLoading } = useSession();
  const [items, setItems] = useState<VideoSummary[] | null>(null);

  useEffect(() => {
    if (!authenticated) return;
    void fetch(endpoint, { cache: "no-store" })
      .then((r) => r.json())
      .then((d) => setItems(d.items ?? []))
      .catch(() => setItems([]));
  }, [authenticated, endpoint]);

  if (sessionLoading) return <GridSkeleton count={10} />;

  if (!authenticated) {
    return (
      <div className="py-24 text-center">
        <h1 className="text-2xl font-bold">{title}</h1>
        <p className="mt-3 text-muted">
          <Link href="/login" className="text-accent underline">
            Sign in
          </Link>{" "}
          to view your {title.toLowerCase()}.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <h1 className="text-2xl font-bold tracking-tight">{title}</h1>
      {items === null ? (
        <GridSkeleton count={10} />
      ) : items.length === 0 ? (
        <p className="py-16 text-center text-muted">{emptyLabel}</p>
      ) : (
        <VideoGrid videos={items} />
      )}
    </div>
  );
}
