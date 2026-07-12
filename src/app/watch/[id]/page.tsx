import Link from "next/link";
import { notFound } from "next/navigation";
import { Eye, Music, Star, ThumbsUp } from "lucide-react";
import { getRelated, getVideo } from "@/lib/pmvhaven";
import { cacheVideo, cacheVideos } from "@/lib/repo";
import { Player } from "@/components/Player";
import { WatchActions } from "@/components/WatchActions";
import { VideoCard } from "@/components/VideoCard";
import { formatViews, ratingColor, timeAgo } from "@/lib/format";
import type { VideoDetail, VideoSummary } from "@/lib/types";

export const dynamic = "force-dynamic";

export default async function WatchPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  let video: VideoDetail;
  let related: VideoSummary[] = [];
  try {
    video = await getVideo(id);
    if (!video.id) notFound();
    cacheVideo(video);
    related = await getRelated(id).catch(() => []);
    cacheVideos(related);
  } catch {
    notFound();
  }

  return (
    <div className="flex flex-col gap-6 lg:flex-row">
      <div className="flex min-w-0 flex-1 flex-col gap-4">
        <Player video={video} />

        <div className="flex flex-col gap-3">
          <h1 className="text-xl font-bold leading-tight sm:text-2xl">{video.title}</h1>

          <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-muted">
            <span className="flex items-center gap-1.5">
              <Eye size={15} /> {formatViews(video.views)} views
            </span>
            {video.rating > 0 && (
              <span className={`flex items-center gap-1.5 ${ratingColor(video.rating)}`}>
                <Star size={15} className="fill-current" /> {Math.round(video.rating)}%
              </span>
            )}
            <span className="flex items-center gap-1.5">
              <ThumbsUp size={15} /> {formatViews(video.likes)}
            </span>
            {video.uploadDate && <span>{timeAgo(video.uploadDate)}</span>}
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3">
            <Link
              href={`/browse?creator=${encodeURIComponent(video.uploaderUsername)}`}
              className="flex items-center gap-3"
            >
              {video.uploaderAvatarUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={video.uploaderAvatarUrl}
                  alt={video.uploaderUsername}
                  className="h-10 w-10 rounded-full object-cover ring-1 ring-border"
                />
              ) : (
                <span className="grid h-10 w-10 place-items-center rounded-full bg-surface-2 font-bold">
                  {video.uploaderUsername?.[0]?.toUpperCase()}
                </span>
              )}
              <div>
                <p className="font-semibold">{video.uploaderUsername}</p>
                <p className="text-xs text-muted">Creator</p>
              </div>
            </Link>
            <WatchActions video={video} />
          </div>

          {video.description && (
            <div className="rounded-xl border border-border bg-surface p-4 text-sm leading-relaxed text-foreground/90 whitespace-pre-wrap">
              {video.description}
            </div>
          )}

          {video.tags.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {video.tags.map((t) => (
                <Link
                  key={t}
                  href={`/browse?tags=${encodeURIComponent(t)}`}
                  className="rounded-full border border-border bg-surface px-3 py-1 text-xs capitalize text-muted transition hover:border-accent/60 hover:text-foreground"
                >
                  #{t}
                </Link>
              ))}
            </div>
          )}

          {video.music.length > 0 && (
            <div className="flex flex-col gap-1.5 rounded-xl border border-border bg-surface p-4">
              <p className="flex items-center gap-2 text-sm font-semibold">
                <Music size={15} /> Music
              </p>
              <ul className="text-sm text-muted">
                {video.music.map((m, i) => (
                  <li key={i}>
                    {m.artist} — {m.song}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </div>

      <aside className="flex w-full shrink-0 flex-col gap-4 lg:w-[360px]">
        <h2 className="text-lg font-bold">Related</h2>
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-1">
          {related.map((v) => (
            <VideoCard key={v.id} video={v} />
          ))}
          {related.length === 0 && (
            <p className="text-sm text-muted">No related videos found.</p>
          )}
        </div>
      </aside>
    </div>
  );
}
