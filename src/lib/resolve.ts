import "server-only";
import { getVideo } from "./pmvhaven";
import { cacheVideo, getCachedSummary } from "./repo";
import type { VideoSummary } from "./types";

/** Resolve a video summary from the local cache, falling back to the API. */
export async function resolveSummary(id: string): Promise<VideoSummary | null> {
  const cached = getCachedSummary(id);
  if (cached) return cached;
  try {
    const detail = await getVideo(id);
    if (!detail.id) return null;
    cacheVideo(detail);
    return detail;
  } catch {
    return null;
  }
}
