import { NextRequest, NextResponse } from "next/server";
import { getHistoryPage } from "@/lib/repo";
import { isAppAuthenticated } from "@/lib/session";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export async function GET(req: NextRequest) {
  if (!(await isAppAuthenticated())) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }
  const sp = req.nextUrl.searchParams;
  const page = Number(sp.get("page")) || 1;
  const limit = Math.min(Number(sp.get("limit")) || 60, 200);
  const { items, total } = getHistoryPage(page, limit);
  return NextResponse.json({
    items,
    pagination: {
      page,
      limit,
      total,
      totalPages: Math.max(1, Math.ceil(total / limit)),
      hasNext: page * limit < total,
      hasPrev: page > 1,
    },
  });
}
