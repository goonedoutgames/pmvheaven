import { NextRequest, NextResponse } from "next/server";
import { getVideos } from "@/lib/pmvhaven";
import { cacheVideos } from "@/lib/repo";
import type { VideoSort } from "@/lib/types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const SORTS: VideoSort[] = [
  "-uploadDate",
  "uploadDate",
  "-views",
  "views",
  "-likes",
  "-bayesianRating",
];

export async function GET(req: NextRequest) {
  const sp = req.nextUrl.searchParams;
  const sortParam = sp.get("sort");
  const sort = SORTS.includes(sortParam as VideoSort)
    ? (sortParam as VideoSort)
    : undefined;

  try {
    const result = await getVideos({
      page: Number(sp.get("page")) || 1,
      limit: Number(sp.get("limit")) || 32,
      sort,
      tags: sp.get("tags") || undefined,
      creator: sp.get("creator") || undefined,
      uploader: sp.get("uploader") || undefined,
    });
    cacheVideos(result.items);
    return NextResponse.json(result);
  } catch (err) {
    const message = err instanceof Error ? err.message : "Failed to load feed";
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
