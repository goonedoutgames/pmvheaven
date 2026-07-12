export function formatViews(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}K`;
  return String(n);
}

export function formatDuration(seconds: number, fallback = ""): string {
  if (!seconds || seconds < 0) return fallback;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const pad = (x: number) => String(x).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

export function timeAgo(dateStr: string): string {
  const then = Date.parse(dateStr);
  if (Number.isNaN(then)) return "";
  const diff = Date.now() - then;
  const sec = Math.floor(diff / 1000);
  const units: [number, string][] = [
    [60, "s"],
    [60, "m"],
    [24, "h"],
    [30, "d"],
    [12, "mo"],
    [Number.POSITIVE_INFINITY, "y"],
  ];
  let value = sec;
  let unit = "s";
  for (const [size, label] of units) {
    if (value < size) {
      unit = label;
      break;
    }
    value = Math.floor(value / size);
    unit = label;
  }
  if (unit === "s" && value < 5) return "just now";
  return `${value}${unit} ago`;
}

/** Same-origin proxy URL for an HLS playlist / segment / media file. */
export function streamProxyUrl(absoluteUrl: string): string {
  return `/api/stream?url=${encodeURIComponent(absoluteUrl)}`;
}

export function ratingColor(rating: number): string {
  if (rating >= 80) return "text-emerald-400";
  if (rating >= 60) return "text-lime-400";
  if (rating >= 40) return "text-amber-400";
  return "text-rose-400";
}
