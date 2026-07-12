import { NextResponse } from "next/server";
import { getAccountUser, isConnected } from "@/lib/pmvhaven";
import { isAppAuthenticated } from "@/lib/session";
import { historyCount } from "@/lib/repo";
import { lastSync, isSyncing } from "@/lib/sync";

export const runtime = "nodejs";

export async function GET() {
  const authed = await isAppAuthenticated();
  return NextResponse.json({
    authenticated: authed && isConnected(),
    user: authed ? getAccountUser() : null,
    historyCount: historyCount(),
    lastSync: lastSync(),
    syncing: isSyncing(),
  });
}
