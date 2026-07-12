import { NextResponse } from "next/server";
import { getVideo } from "@/lib/pmvhaven";
import { cacheVideo } from "@/lib/repo";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

/** Returns the stream info the client mini-player needs for a single video. */
export async function GET(
  _req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  try {
    const video = await getVideo(id);
    if (!video.id) {
      return NextResponse.json({ error: "Not found" }, { status: 404 });
    }
    cacheVideo(video);
    return NextResponse.json({
      id: video.id,
      title: video.title,
      thumbnailUrl: video.thumbnailUrl,
      videoUrl: video.videoUrl,
      hlsEnabled: video.hlsEnabled,
      hlsMasterPlaylistUrl: video.hlsMasterPlaylistUrl,
      durationSeconds: video.durationSeconds,
    });
  } catch {
    return NextResponse.json({ error: "Not found" }, { status: 404 });
  }
}
