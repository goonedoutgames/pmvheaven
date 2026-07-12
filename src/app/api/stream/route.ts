import { NextRequest } from "next/server";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

// Only allow proxying PMVHaven's own media hosts (SSRF guard).
const ALLOWED_HOST_SUFFIXES = [
  ".pmvhaven.com",
  "pmvhaven.com",
  ".io.cloud.ovh.net",
  ".r2.cloudflarestorage.com",
];

function isAllowed(url: URL): boolean {
  const h = url.hostname.toLowerCase();
  return ALLOWED_HOST_SUFFIXES.some((s) => h === s || h.endsWith(s));
}

function isPlaylist(url: string, contentType: string | null): boolean {
  return (
    url.split("?")[0].endsWith(".m3u8") ||
    (contentType?.includes("mpegurl") ?? false)
  );
}

/** Rewrite every URI in an HLS playlist to route back through this proxy. */
function rewritePlaylist(text: string, base: string): string {
  const rewrite = (uri: string) => {
    const abs = new URL(uri, base).toString();
    return `/api/stream?url=${encodeURIComponent(abs)}`;
  };
  return text
    .split("\n")
    .map((line) => {
      const trimmed = line.trim();
      if (!trimmed) return line;
      // Rewrite URI="..." attributes (EXT-X-KEY, EXT-X-MAP, etc.)
      if (trimmed.startsWith("#")) {
        return line.replace(/URI="([^"]+)"/g, (_m, uri) => `URI="${rewrite(uri)}"`);
      }
      // Plain URI line (variant playlist or segment)
      return rewrite(trimmed);
    })
    .join("\n");
}

export async function GET(req: NextRequest) {
  const target = req.nextUrl.searchParams.get("url");
  if (!target) return new Response("Missing url", { status: 400 });

  let url: URL;
  try {
    url = new URL(target);
  } catch {
    return new Response("Invalid url", { status: 400 });
  }
  if (url.protocol !== "https:" || !isAllowed(url)) {
    return new Response("Host not allowed", { status: 403 });
  }

  const headers: Record<string, string> = {
    "User-Agent": UA,
    Referer: "https://pmvhaven.com/",
    Accept: "*/*",
  };
  const range = req.headers.get("range");
  if (range) headers["Range"] = range;

  const upstream = await fetch(url.toString(), { headers, cache: "no-store" });
  const contentType = upstream.headers.get("content-type");

  if (isPlaylist(url.toString(), contentType)) {
    const text = await upstream.text();
    const rewritten = rewritePlaylist(text, url.toString());
    return new Response(rewritten, {
      status: upstream.status,
      headers: {
        "Content-Type": "application/vnd.apple.mpegurl",
        "Cache-Control": "no-store",
      },
    });
  }

  // Stream media bytes through, preserving range semantics.
  const outHeaders = new Headers();
  const passthrough = [
    "content-type",
    "content-length",
    "content-range",
    "accept-ranges",
    "cache-control",
  ];
  for (const h of passthrough) {
    const v = upstream.headers.get(h);
    if (v) outHeaders.set(h, v);
  }
  if (!outHeaders.has("accept-ranges")) outHeaders.set("accept-ranges", "bytes");

  return new Response(upstream.body, {
    status: upstream.status,
    headers: outHeaders,
  });
}
