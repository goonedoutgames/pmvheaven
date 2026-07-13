import "server-only";
import crypto from "node:crypto";
import { cookies } from "next/headers";
import { getDb } from "./db";

/**
 * App-level session: a random opaque token stored in an httpOnly cookie and
 * validated against the app_sessions table. This gates our own authenticated
 * pages; the actual PMVHaven credentials/cookies never reach the browser.
 */

const COOKIE = "ph_auth";
const TTL_MS = 1000 * 60 * 60 * 24 * 60; // 60 days

export async function createAppSession(): Promise<void> {
  const token = crypto.randomBytes(32).toString("hex");
  const now = Date.now();
  getDb()
    .prepare("INSERT INTO app_sessions (token, created_at, expires_at) VALUES (?, ?, ?)")
    .run(token, now, now + TTL_MS);

  const store = await cookies();
  store.set(COOKIE, token, {
    httpOnly: true,
    sameSite: "lax",
    // The desktop app serves over http://127.0.0.1, where WebKitGTK refuses to
    // store `Secure` cookies — so the session would never persist and login
    // silently fails. Only mark Secure for a real HTTPS web deployment.
    secure: process.env.NODE_ENV === "production" && process.env.PH_DESKTOP !== "1",
    path: "/",
    maxAge: Math.floor(TTL_MS / 1000),
  });
}

export async function destroyAppSession(): Promise<void> {
  const store = await cookies();
  const token = store.get(COOKIE)?.value;
  if (token) {
    getDb().prepare("DELETE FROM app_sessions WHERE token = ?").run(token);
  }
  store.delete(COOKIE);
}

export async function isAppAuthenticated(): Promise<boolean> {
  const store = await cookies();
  const token = store.get(COOKIE)?.value;
  if (!token) return false;
  const row = getDb()
    .prepare("SELECT expires_at FROM app_sessions WHERE token = ?")
    .get(token) as { expires_at: number } | undefined;
  if (!row) return false;
  if (row.expires_at < Date.now()) {
    getDb().prepare("DELETE FROM app_sessions WHERE token = ?").run(token);
    return false;
  }
  return true;
}
