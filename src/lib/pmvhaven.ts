import "server-only";
import { getDb } from "./db";
import { decrypt, encrypt } from "./crypto";
import type {
  AccountUser,
  FeedParams,
  Paged,
  Pagination,
  PopularTag,
  VideoDetail,
  VideoSummary,
} from "./types";

const BASE = "https://pmvhaven.com/api";
const UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

// Better Auth cookie names we care about capturing/forwarding.
const AUTH_COOKIE_PREFIXES = ["better-auth.", "__Secure-better-auth."];

export class PmvError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

/* ----------------------------- account state ----------------------------- */

interface AccountRow {
  pmv_user_id: string | null;
  username: string | null;
  email: string | null;
  avatar_url: string | null;
  enc_email: string | null;
  enc_password: string | null;
  cookies: string | null;
}

function readAccount(): AccountRow | null {
  return (
    (getDb().prepare("SELECT * FROM account WHERE id = 1").get() as
      | AccountRow
      | undefined) ?? null
  );
}

export function getAccountUser(): AccountUser | null {
  const row = readAccount();
  if (!row || !row.cookies || !row.username) return null;
  return {
    id: row.pmv_user_id ?? "",
    username: row.username,
    email: row.email ?? undefined,
    avatarUrl: row.avatar_url ?? undefined,
  };
}

export function isConnected(): boolean {
  const row = readAccount();
  return !!(row && row.cookies);
}

function getStoredCookieHeader(): string | null {
  const row = readAccount();
  if (!row?.cookies) return null;
  try {
    const map = JSON.parse(decrypt(row.cookies)) as Record<string, string>;
    const pairs = Object.entries(map).map(([k, v]) => `${k}=${v}`);
    return pairs.length ? pairs.join("; ") : null;
  } catch {
    return null;
  }
}

function storeCookiesFromResponse(res: Response) {
  const setCookies = res.headers.getSetCookie?.() ?? [];
  if (!setCookies.length) return;

  const existing = getStoredCookieHeader();
  const map: Record<string, string> = {};
  if (existing) {
    for (const pair of existing.split("; ")) {
      const idx = pair.indexOf("=");
      if (idx > 0) map[pair.slice(0, idx)] = pair.slice(idx + 1);
    }
  }
  for (const line of setCookies) {
    const first = line.split(";")[0];
    const idx = first.indexOf("=");
    if (idx <= 0) continue;
    const name = first.slice(0, idx).trim();
    const value = first.slice(idx + 1).trim();
    if (AUTH_COOKIE_PREFIXES.some((p) => name.startsWith(p))) {
      map[name] = value;
    }
  }
  if (Object.keys(map).length) {
    getDb()
      .prepare("UPDATE account SET cookies = ? WHERE id = 1")
      .run(encrypt(JSON.stringify(map)));
  }
}

/* ------------------------------- low level ------------------------------- */

interface RequestOpts {
  method?: string;
  body?: unknown;
  auth?: boolean; // attach stored session cookies
  query?: Record<string, string | number | undefined>;
}

function buildUrl(path: string, query?: RequestOpts["query"]): string {
  const url = new URL(BASE + path);
  if (query) {
    for (const [k, v] of Object.entries(query)) {
      if (v !== undefined && v !== null && v !== "") url.searchParams.set(k, String(v));
    }
  }
  return url.toString();
}

async function rawRequest(path: string, opts: RequestOpts = {}): Promise<Response> {
  const headers: Record<string, string> = {
    "User-Agent": UA,
    Accept: "application/json",
    Referer: "https://pmvhaven.com/",
    Origin: "https://pmvhaven.com",
  };
  if (opts.body !== undefined) headers["Content-Type"] = "application/json";
  if (opts.auth) {
    const cookie = getStoredCookieHeader();
    if (cookie) headers["Cookie"] = cookie;
  }
  return fetch(buildUrl(path, opts.query), {
    method: opts.method ?? "GET",
    headers,
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
    cache: "no-store",
  });
}

async function request<T = unknown>(path: string, opts: RequestOpts = {}): Promise<T> {
  let res = await rawRequest(path, opts);

  // Silent re-auth: if an authenticated call is rejected, try to re-login
  // using stored credentials once, then retry the original request.
  if (opts.auth && (res.status === 401 || res.status === 403)) {
    const reauthed = await tryReauth();
    if (reauthed) res = await rawRequest(path, opts);
  }

  if (!res.ok) {
    let msg = `PMVHaven request failed (${res.status})`;
    try {
      const j = (await res.json()) as { message?: string };
      if (j?.message) msg = j.message;
    } catch {
      /* ignore */
    }
    throw new PmvError(msg, res.status);
  }
  return (await res.json()) as T;
}

/* --------------------------------- auth ---------------------------------- */

export interface SignInResult {
  ok: boolean;
  user?: AccountUser;
  error?: string;
}

/** Sign in with email + password (Better Auth) and persist the session. */
export async function signIn(
  email: string,
  password: string,
  rememberMe = true,
): Promise<SignInResult> {
  const res = await rawRequest("/auth/sign-in/email", {
    method: "POST",
    body: { email, password, rememberMe },
  });

  if (!res.ok) {
    let error = "Invalid email or password";
    try {
      const j = (await res.json()) as { message?: string };
      if (j?.message) error = j.message;
    } catch {
      /* ignore */
    }
    return { ok: false, error };
  }

  // Persist row + captured cookies, then hydrate the profile.
  const now = Date.now();
  getDb()
    .prepare(
      `INSERT INTO account (id, enc_email, enc_password, created_at, last_login_at)
       VALUES (1, ?, ?, ?, ?)
       ON CONFLICT(id) DO UPDATE SET enc_email = excluded.enc_email,
         enc_password = excluded.enc_password, last_login_at = excluded.last_login_at`,
    )
    .run(encrypt(email), encrypt(password), now, now);

  storeCookiesFromResponse(res);

  const user = await refreshProfile();
  return { ok: true, user: user ?? undefined };
}

async function tryReauth(): Promise<boolean> {
  const row = readAccount();
  if (!row?.enc_email || !row?.enc_password) return false;
  try {
    const email = decrypt(row.enc_email);
    const password = decrypt(row.enc_password);
    const res = await rawRequest("/auth/sign-in/email", {
      method: "POST",
      body: { email, password, rememberMe: true },
    });
    if (!res.ok) return false;
    storeCookiesFromResponse(res);
    getDb().prepare("UPDATE account SET last_login_at = ? WHERE id = 1").run(Date.now());
    return true;
  } catch {
    return false;
  }
}

interface SessionResponse {
  user?: {
    id?: string;
    _id?: string;
    username?: string;
    name?: string;
    email?: string;
    avatarUrl?: string;
    image?: string;
  } | null;
}

/** Fetch the current session/profile and cache it on the account row. */
export async function refreshProfile(): Promise<AccountUser | null> {
  try {
    const data = await request<SessionResponse>("/auth/session", { auth: true });
    const u = data.user;
    if (!u) return getAccountUser();
    const user: AccountUser = {
      id: u.id ?? u._id ?? "",
      username: u.username ?? u.name ?? "",
      email: u.email,
      avatarUrl: u.avatarUrl ?? u.image,
    };
    getDb()
      .prepare(
        "UPDATE account SET pmv_user_id = ?, username = ?, email = ?, avatar_url = ? WHERE id = 1",
      )
      .run(user.id, user.username, user.email ?? null, user.avatarUrl ?? null);
    return user;
  } catch {
    return getAccountUser();
  }
}

export async function signOut(): Promise<void> {
  try {
    await rawRequest("/auth/sign-out", { method: "POST", auth: true });
  } catch {
    /* ignore */
  }
  getDb().prepare("DELETE FROM account WHERE id = 1").run();
}

/* ------------------------------ normalizers ------------------------------ */

type RawVideo = Record<string, unknown>;

function num(v: unknown, d = 0): number {
  return typeof v === "number" && !Number.isNaN(v) ? v : d;
}
function str(v: unknown, d = ""): string {
  return typeof v === "string" ? v : d;
}
function arr<T = unknown>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

export function normalizeSummary(v: RawVideo): VideoSummary {
  const tags = arr<string>(v.tags).length
    ? arr<string>(v.tags)
    : arr<string>(v.top5Tags);
  return {
    id: str(v._id) || str(v.id),
    title: str(v.title, "Untitled"),
    uploader: str(v.uploader),
    uploaderUsername: str(v.uploaderUsername) || str(v.uploader),
    thumbnailUrl: str(v.thumbnailUrl),
    previewUrl: str(v.previewUrl) || undefined,
    views: num(v.views),
    duration: str(v.duration),
    durationSeconds: num(v.durationSeconds),
    aspectRatio: num(v.aspectRatio, 1.7778),
    likes: num(v.likes),
    dislikes: num(v.dislikes),
    rating: num(v.bayesianRating),
    tags,
    uploadDate: str(v.uploadDate) || str(v.releaseDate),
    isRemix: Boolean(v.isRemix),
    hasVoiceOver: Boolean(v.hasVoiceOver),
    hasExtremeContent: Boolean(v.hasExtremeContent),
  };
}

export function normalizeDetail(v: RawVideo): VideoDetail {
  const summary = normalizeSummary(v);
  return {
    ...summary,
    description: str(v.description),
    videoUrl: str(v.videoUrl),
    hlsMasterPlaylistUrl: str(v.hlsMasterPlaylistUrl) || undefined,
    hlsEnabled: Boolean(v.hlsEnabled),
    hlsVariants: arr<Record<string, unknown>>(v.hlsVariants).map((h) => ({
      resolution: str(h.resolution),
      width: num(h.width),
      height: num(h.height),
      bandwidth: num(h.bandwidth),
      playlistUrl: str(h.playlistUrl),
    })),
    width: num(v.width),
    height: num(v.height),
    favorites: num(v.favorites),
    creator: arr<string>(v.creator),
    stars: arr<string>(v.starsTags),
    music: arr<Record<string, unknown>>(v.music).map((m) => ({
      artist: str(m.artist),
      song: str(m.song),
    })),
    timelineThumbnails: arr<Record<string, unknown>>(v.timelineThumbnails).map((t) => ({
      url: str(t.url),
      captureTime: num(t.captureTime),
    })),
    uploaderAvatarUrl: str(v.uploaderAvatarUrl) || undefined,
    uploaderId: str(v.uploaderId) || undefined,
    watchProgress: num(v.watchProgress),
    isLiked: Boolean(v.isLiked),
    isDisliked: Boolean(v.isDisliked),
    isFavorited: Boolean(v.isFavorited),
    isWatchLater: Boolean(v.isWatchLater),
  };
}

function normalizePagination(p: Record<string, unknown> | undefined, fallbackLen: number): Pagination {
  return {
    page: num(p?.page, 1),
    limit: num(p?.limit, fallbackLen),
    total: num(p?.total ?? p?.totalVideos, fallbackLen),
    totalPages: num(p?.totalPages, 1),
    hasNext: Boolean(p?.hasNext),
    hasPrev: Boolean(p?.hasPrev),
  };
}

/* ------------------------------- endpoints ------------------------------- */

interface ListResponse {
  data?: RawVideo[];
  videos?: RawVideo[];
  results?: RawVideo[];
  pagination?: Record<string, unknown>;
}

function pickList(r: ListResponse): RawVideo[] {
  return r.data ?? r.videos ?? r.results ?? [];
}

export async function getVideos(params: FeedParams = {}): Promise<Paged<VideoSummary>> {
  const r = await request<ListResponse>("/videos", {
    query: {
      page: params.page ?? 1,
      limit: params.limit ?? 32,
      sort: params.sort,
      tags: params.tags,
      creator: params.creator,
      uploader: params.uploader,
    },
  });
  const list = pickList(r);
  return {
    items: list.map(normalizeSummary),
    pagination: normalizePagination(r.pagination, list.length),
  };
}

export async function getTrending(): Promise<VideoSummary[]> {
  const r = await request<ListResponse>("/videos/trending");
  return pickList(r).map(normalizeSummary);
}

export async function getRandom(limit = 12): Promise<VideoSummary[]> {
  const r = await request<ListResponse>("/videos/random", { query: { limit } });
  return pickList(r).map(normalizeSummary);
}

export async function getVideo(id: string): Promise<VideoDetail> {
  const r = await request<{ data?: RawVideo } & RawVideo>(`/videos/${id}`, {
    auth: isConnected(),
  });
  const raw = (r.data as RawVideo) ?? r;
  return normalizeDetail(raw);
}

export async function getRelated(id: string): Promise<VideoSummary[]> {
  const r = await request<ListResponse>(`/videos/${id}/related`);
  return pickList(r).map(normalizeSummary);
}

export async function getPopularTags(): Promise<PopularTag[]> {
  const r = await request<{ data?: Array<Record<string, unknown>> }>("/tags/popular");
  return arr<Record<string, unknown>>(r.data).map((t) => ({
    name: str(t.name),
    usageCount: num(t.usageCount),
  }));
}

/** Full-text search (requires an authenticated session). */
export async function search(
  q: string,
  page = 1,
  limit = 32,
): Promise<Paged<VideoSummary>> {
  const r = await request<ListResponse>("/search", {
    auth: true,
    query: { q, page, limit },
  });
  const list = pickList(r);
  return {
    items: list.map(normalizeSummary),
    pagination: normalizePagination(r.pagination, list.length),
  };
}

export interface RemoteHistoryEntry {
  videoId: string;
  watchedAt: string;
  progress: number; // 0..1 fraction
}

export interface RemoteHistory {
  totalRetained: number; // watchHistoryCount reported by PMVHaven
  entries: RemoteHistoryEntry[]; // most-recent window (free tier caps at ~500)
}

interface SessionUserHistory {
  user?: {
    watchHistory?: Array<{ videoId: string; watchedAt?: string }>;
    watchProgress?: Array<{ videoId: string; progress?: number; duration?: number }>;
    watchHistoryCount?: number;
  } | null;
}

/**
 * PMVHaven's documented `/user/watch-history` endpoint is broken server-side
 * (500: "$slice path collision"). The watch history is instead embedded in the
 * user object returned by `/auth/session`: `watchHistory` [{videoId, watchedAt}]
 * plus `watchProgress` [{videoId, progress(seconds), duration}]. Free accounts
 * only expose the most recent ~500 entries (of `watchHistoryCount` total).
 */
export async function fetchRemoteHistory(): Promise<RemoteHistory> {
  const data = await request<SessionUserHistory>("/auth/session", { auth: true });
  const u = data.user;
  if (!u) return { totalRetained: 0, entries: [] };

  const progressByVideo = new Map<string, number>();
  for (const p of u.watchProgress ?? []) {
    if (!p.videoId) continue;
    const dur = num(p.duration);
    const secs = num(p.progress);
    const fraction = dur > 0 ? Math.min(1, Math.max(0, secs / dur)) : 0;
    progressByVideo.set(p.videoId, fraction);
  }

  const entries: RemoteHistoryEntry[] = (u.watchHistory ?? [])
    .filter((h) => h.videoId)
    .map((h) => ({
      videoId: h.videoId,
      watchedAt: str(h.watchedAt) || new Date().toISOString(),
      progress: progressByVideo.get(h.videoId) ?? 0,
    }));

  return { totalRetained: num(u.watchHistoryCount, entries.length), entries };
}

/** Hydrate a list of video IDs into summaries via the public bulk endpoint. */
export async function getVideosBulk(ids: string[]): Promise<VideoSummary[]> {
  const out: VideoSummary[] = [];
  const CHUNK = 100;
  for (let i = 0; i < ids.length; i += CHUNK) {
    const chunk = ids.slice(i, i + CHUNK);
    if (!chunk.length) continue;
    const r = await request<{ data?: RawVideo[]; videos?: RawVideo[] }>(
      "/videos/bulk",
      { query: { ids: chunk.join(",") } },
    );
    out.push(...pickList(r).map(normalizeSummary));
  }
  return out;
}

export async function getRemoteFavorites(limit = 100): Promise<VideoSummary[]> {
  const r = await request<ListResponse>("/user/favorites", {
    auth: true,
    query: { limit },
  });
  return pickList(r).map(normalizeSummary);
}

export async function getRemoteWatchLater(limit = 100): Promise<VideoSummary[]> {
  const r = await request<ListResponse>("/user/watch-later", {
    auth: true,
    query: { limit },
  });
  return pickList(r).map(normalizeSummary);
}

export async function setFavorite(id: string, on: boolean): Promise<void> {
  await request(`/videos/${id}/favorite`, {
    method: on ? "POST" : "DELETE",
    auth: true,
  });
}

export async function setWatchLater(id: string, on: boolean): Promise<void> {
  await request(`/videos/${id}/watch-later`, {
    method: on ? "POST" : "DELETE",
    auth: true,
  });
}

/** Record a view against a video (fire-and-forget on the watch page). */
export async function recordView(id: string): Promise<void> {
  try {
    await rawRequest(`/videos/${id}`, { method: "POST", auth: isConnected() });
  } catch {
    /* best effort */
  }
}
