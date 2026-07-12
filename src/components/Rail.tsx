import Link from "next/link";
import { ChevronRight } from "lucide-react";
import type { VideoSummary } from "@/lib/types";
import { VideoCard } from "./VideoCard";

export function Rail({
  title,
  href,
  videos,
}: {
  title: string;
  href?: string;
  videos: VideoSummary[];
}) {
  if (!videos.length) return null;
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-bold tracking-tight sm:text-xl">{title}</h2>
        {href && (
          <Link
            href={href}
            className="flex items-center gap-0.5 text-sm text-muted transition hover:text-accent"
          >
            See all <ChevronRight size={16} />
          </Link>
        )}
      </div>
      <div className="-mx-3 flex snap-x gap-4 overflow-x-auto px-3 pb-2 sm:mx-0 sm:px-0">
        {videos.map((v) => (
          <div key={v.id} className="w-[70vw] shrink-0 snap-start sm:w-72">
            <VideoCard video={v} />
          </div>
        ))}
      </div>
    </section>
  );
}
