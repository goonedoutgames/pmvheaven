import type { HistoryEntry, VideoSummary } from "@/lib/types";
import { VideoCard } from "./VideoCard";

export function VideoGrid({
  videos,
  progressById,
}: {
  videos: VideoSummary[];
  progressById?: Record<string, number>;
}) {
  return (
    <div className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {videos.map((v) => (
        <VideoCard key={v.id} video={v} watchedProgress={progressById?.[v.id]} />
      ))}
    </div>
  );
}

export function HistoryGrid({ entries }: { entries: HistoryEntry[] }) {
  return (
    <div className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {entries.map((e) => (
        <VideoCard
          key={`${e.video.id}-${e.watchedAt}`}
          video={e.video}
          watchedProgress={e.progress}
        />
      ))}
    </div>
  );
}

export function GridSkeleton({ count = 10 }: { count?: number }) {
  return (
    <div className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="flex flex-col gap-2">
          <div className="aspect-video animate-pulse rounded-xl bg-surface-2" />
          <div className="h-4 w-4/5 animate-pulse rounded bg-surface-2" />
          <div className="h-3 w-1/2 animate-pulse rounded bg-surface-2" />
        </div>
      ))}
    </div>
  );
}
