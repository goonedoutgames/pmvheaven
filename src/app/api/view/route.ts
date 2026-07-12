import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { isConnected, recordView } from "@/lib/pmvhaven";
import { upsertHistory } from "@/lib/repo";
import { resolveSummary } from "@/lib/resolve";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const schema = z.object({
  id: z.string().min(1),
  progress: z.number().min(0).max(1).optional(),
  countView: z.boolean().optional(),
});

export async function POST(req: NextRequest) {
  const parsed = schema.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return NextResponse.json({ error: "id is required" }, { status: 400 });
  }
  const { id, progress = 0, countView = false } = parsed.data;

  const summary = await resolveSummary(id);
  if (summary) {
    upsertHistory(
      { video: summary, watchedAt: new Date().toISOString(), progress },
      "local",
    );
  }

  // Only bump the remote view counter once per session start.
  if (countView && isConnected()) {
    void recordView(id).catch(() => {});
  }
  return NextResponse.json({ ok: true });
}
