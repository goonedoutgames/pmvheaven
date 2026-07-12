import { NextResponse } from "next/server";
import { signOut } from "@/lib/pmvhaven";
import { destroyAppSession } from "@/lib/session";

export const runtime = "nodejs";

export async function POST() {
  await destroyAppSession();
  await signOut();
  return NextResponse.json({ ok: true });
}
