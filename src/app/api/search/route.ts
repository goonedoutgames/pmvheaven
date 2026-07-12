import { NextRequest, NextResponse } from "next/server";
import { getVideos, isConnected, search } from "@/lib/pmvhaven";
import { cacheVideos } from "@/lib/repo";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  const sp = req.nextUrl.searchParams;
  const q = (sp.get("q") || "").trim();
  const page = Number(sp.get("page")) || 1;
  const limit = Number(sp.get("limit")) || 32;
  if (!q) return NextResponse.json({ items: [], pagination: null });

  try {
    // Full-text search needs an authenticated session; fall back to the
    // public tag filter for signed-out browsing.
    const result = isConnected()
      ? await search(q, page, limit)
      : await getVideos({ page, limit, tags: q });
    cacheVideos(result.items);
    return NextResponse.json(result);
  } catch (err) {
    // If the authed search fails for any reason, degrade to tag filtering.
    try {
      const result = await getVideos({ page, limit, tags: q });
      cacheVideos(result.items);
      return NextResponse.json(result);
    } catch {
      const message = err instanceof Error ? err.message : "Search failed";
      return NextResponse.json({ error: message }, { status: 502 });
    }
  }
}
