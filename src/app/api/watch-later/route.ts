import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { isConnected, setWatchLater } from "@/lib/pmvhaven";
import { getBucket, setLocalBucket } from "@/lib/repo";
import { resolveSummary } from "@/lib/resolve";
import { isAppAuthenticated } from "@/lib/session";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET() {
  if (!(await isAppAuthenticated())) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }
  return NextResponse.json({ items: getBucket("watch_later") });
}

const schema = z.object({ id: z.string().min(1), on: z.boolean() });

export async function POST(req: NextRequest) {
  if (!(await isAppAuthenticated())) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }
  const parsed = schema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "id and on are required" }, { status: 400 });
  }
  const { id, on } = parsed.data;

  const summary = await resolveSummary(id);
  if (summary) setLocalBucket("watch_later", summary, on);

  if (isConnected()) {
    try {
      await setWatchLater(id, on);
    } catch {
      /* keep local state even if remote write fails */
    }
  }
  return NextResponse.json({ ok: true, watchLater: on });
}
