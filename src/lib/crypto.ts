import crypto from "node:crypto";
import { getSetting, setSetting } from "./db";

/**
 * AES-256-GCM encryption for data at rest (PMVHaven credentials + session
 * cookies). The key is derived from the PH_SECRET env var when present;
 * otherwise a random key is generated once and persisted in the settings
 * table so a restart doesn't invalidate stored data. Set PH_SECRET in
 * production for a stable, externally-managed key.
 */

let cachedKey: Buffer | null = null;

function getKey(): Buffer {
  if (cachedKey) return cachedKey;

  const envSecret = process.env.PH_SECRET;
  if (envSecret && envSecret.length > 0) {
    cachedKey = crypto.createHash("sha256").update(envSecret).digest();
    return cachedKey;
  }

  let stored = getSetting("crypto_key");
  if (!stored) {
    stored = crypto.randomBytes(32).toString("hex");
    setSetting("crypto_key", stored);
  }
  cachedKey = Buffer.from(stored, "hex");
  return cachedKey;
}

export function encrypt(plain: string): string {
  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv("aes-256-gcm", getKey(), iv);
  const enc = Buffer.concat([cipher.update(plain, "utf8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  return `${iv.toString("base64")}.${tag.toString("base64")}.${enc.toString("base64")}`;
}

export function decrypt(payload: string): string {
  const [ivB64, tagB64, dataB64] = payload.split(".");
  if (!ivB64 || !tagB64 || !dataB64) throw new Error("Malformed ciphertext");
  const decipher = crypto.createDecipheriv(
    "aes-256-gcm",
    getKey(),
    Buffer.from(ivB64, "base64"),
  );
  decipher.setAuthTag(Buffer.from(tagB64, "base64"));
  return Buffer.concat([
    decipher.update(Buffer.from(dataB64, "base64")),
    decipher.final(),
  ]).toString("utf8");
}
