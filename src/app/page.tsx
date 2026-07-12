import Link from "next/link";
import { getPopularTags, getTrending, getVideos } from "@/lib/pmvhaven";
import { cacheVideos } from "@/lib/repo";
import { Rail } from "@/components/Rail";
import { VideoGrid } from "@/components/VideoGrid";
import type { PopularTag, VideoSummary } from "@/lib/types";

export const dynamic = "force-dynamic";

export default async function HomePage() {
  let trending: VideoSummary[] = [];
  let topRated: VideoSummary[] = [];
  let newest: VideoSummary[] = [];
  let tags: PopularTag[] = [];
  let failed = false;

  try {
    const [t, r, n, pt] = await Promise.all([
      getTrending().catch(() => []),
      getVideos({ sort: "-bayesianRating", limit: 12 }).catch(() => ({ items: [] as VideoSummary[] } as never)),
      getVideos({ sort: "-uploadDate", limit: 20 }).catch(() => ({ items: [] as VideoSummary[] } as never)),
      getPopularTags().catch(() => []),
    ]);
    trending = t;
    topRated = r.items ?? [];
    newest = n.items ?? [];
    tags = pt;
    cacheVideos([...trending, ...topRated, ...newest]);
  } catch {
    failed = true;
  }

  if (failed || (!trending.length && !newest.length)) {
    return (
      <div className="py-24 text-center text-muted">
        <p className="text-lg">Couldn&apos;t reach PMVHaven right now.</p>
        <p className="mt-2 text-sm">Please try again in a moment.</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-10">
      <Rail title="Trending now" href="/browse?sort=-views" videos={trending} />

      {tags.length > 0 && (
        <section className="flex flex-col gap-3">
          <h2 className="text-lg font-bold tracking-tight sm:text-xl">Popular tags</h2>
          <div className="flex flex-wrap gap-2">
            {tags.slice(0, 24).map((tag) => (
              <Link
                key={tag.name}
                href={`/browse?tags=${encodeURIComponent(tag.name)}`}
                className="rounded-full border border-border bg-surface px-3 py-1.5 text-sm capitalize text-muted transition hover:border-accent/60 hover:text-foreground"
              >
                {tag.name}
              </Link>
            ))}
          </div>
        </section>
      )}

      <Rail title="Top rated" href="/browse?sort=-bayesianRating" videos={topRated} />

      <section className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-bold tracking-tight sm:text-xl">Freshly uploaded</h2>
          <Link
            href="/browse?sort=-uploadDate"
            className="text-sm text-muted transition hover:text-accent"
          >
            Browse all
          </Link>
        </div>
        <VideoGrid videos={newest} />
      </section>
    </div>
  );
}
