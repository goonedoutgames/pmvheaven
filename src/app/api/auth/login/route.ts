import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { signIn } from "@/lib/pmvhaven";
import { createAppSession } from "@/lib/session";
import { syncWatchHistory } from "@/lib/sync";

export const runtime = "nodejs";

const schema = z.object({
  email: z.string().email(),
  password: z.string().min(1),
});

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => null);
  const parsed = schema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: "Email and password are required" }, { status: 400 });
  }

  const result = await signIn(parsed.data.email, parsed.data.password);
  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 401 });
  }

  await createAppSession();

  // Kick off an initial full history snapshot in the background.
  void syncWatchHistory(true).catch(() => {});

  return NextResponse.json({ ok: true, user: result.user });
}
