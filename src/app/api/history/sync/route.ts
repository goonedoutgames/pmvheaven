import { NextRequest, NextResponse } from "next/server";
import { isAppAuthenticated } from "@/lib/session";
import { syncWatchHistory } from "@/lib/sync";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";
// Sync can page through thousands of entries; allow a generous window.
export const maxDuration = 300;

export async function POST(req: NextRequest) {
  if (!(await isAppAuthenticated())) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }
  const full = req.nextUrl.searchParams.get("full") === "1";
  const result = await syncWatchHistory(full);
  const status = result.status === "error" ? 502 : 200;
  return NextResponse.json(result, { status });
}
