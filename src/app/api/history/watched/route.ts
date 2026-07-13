import { NextResponse } from "next/server";
import { watchedProgressMap } from "@/lib/repo";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

/** Returns every watched video id -> progress so the UI can badge watched items. */
export async function GET() {
  return NextResponse.json(
    { watched: watchedProgressMap() },
    { headers: { "Cache-Control": "no-store" } },
  );
}
