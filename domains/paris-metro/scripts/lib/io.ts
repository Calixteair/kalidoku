/**
 * Helpers I/O : cache fichier, fetch HTTP avec retry, hash sha256.
 *
 * Le pipeline doit être idempotent : si un cache existe et est récent (< maxAgeDays),
 * on ne retape pas le réseau.
 */

import { createHash } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { dirname } from "node:path";

const DEFAULT_TIMEOUT_MS = 90_000;
const FETCH_RETRIES = 2;

export async function readJsonIfFresh<T>(path: string, maxAgeDays: number): Promise<T | null> {
  try {
    const info = await stat(path);
    const ageMs = Date.now() - info.mtimeMs;
    const maxAgeMs = maxAgeDays * 24 * 3600 * 1000;
    if (ageMs > maxAgeMs) {
      return null;
    }
    const raw = await readFile(path, "utf8");
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export async function writeJson(path: string, value: unknown): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, JSON.stringify(value, null, 2) + "\n", "utf8");
}

export async function fetchJson<T>(url: string, body?: string): Promise<T> {
  let lastErr: unknown = null;
  for (let attempt = 0; attempt <= FETCH_RETRIES; attempt += 1) {
    try {
      return await fetchOnce<T>(url, body);
    } catch (err) {
      lastErr = err;
      const backoffMs = 1500 * (attempt + 1);
      await sleep(backoffMs);
    }
  }
  throw lastErr instanceof Error ? lastErr : new Error("fetch failed");
}

async function fetchOnce<T>(url: string, body?: string): Promise<T> {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), DEFAULT_TIMEOUT_MS);
  try {
    const headers: Record<string, string> = {
      "User-Agent": "kalidoku-ingest/0.2 (https://github.com/Calixteair/kalidoku)",
      Accept: "application/json",
    };
    if (body) headers["Content-Type"] = "application/x-www-form-urlencoded";
    const response = await fetch(url, {
      method: body ? "POST" : "GET",
      headers,
      body,
      signal: ctrl.signal,
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} on ${url}`);
    }
    return (await response.json()) as T;
  } finally {
    clearTimeout(timer);
  }
}

export function sha256Hex(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
