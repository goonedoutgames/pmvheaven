import { NextResponse } from "next/server";
import { isAppAuthenticated } from "@/lib/session";
import { isSyncing, lastSync, syncProgress, syncWatchHistory } from "@/lib/sync";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";
// Sync pages through the history window; allow a generous window.
export const maxDuration = 300;

export async function POST() {
  if (!(await isAppAuthenticated())) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }
  const result = await syncWatchHistory();
  const status = result.status === "error" ? 502 : 200;
  return NextResponse.json(result, { status });
}

// Lightweight polling endpoint for live progress during a running sync.
export async function GET() {
  if (!(await isAppAuthenticated())) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }
  return NextResponse.json({
    running: isSyncing(),
    progress: syncProgress(),
    last: lastSync(),
  });
}
