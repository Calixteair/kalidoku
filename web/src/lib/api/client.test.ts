import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { api, ApiError } from "./client.js";

describe("api client", () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });
  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("substitutes path params and builds query string", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      expect(String(input)).toBe("/api/domains/paris-metro/autocomplete?q=ch%C3%A2&limit=8");
      expect(init?.method).toBe("GET");
      return new Response(JSON.stringify([]), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    globalThis.fetch = fetchMock as typeof fetch;
    const result = await api.get(
      "/api/domains/{domain}/autocomplete",
      { domain: "paris-metro" },
      { q: "châ", limit: 8 },
    );
    expect(result).toEqual([]);
  });

  it("parses error JSON into ApiError", async () => {
    globalThis.fetch = vi.fn(
      async () =>
        new Response(JSON.stringify({ code: "rate_limited", message: "slow down" }), {
          status: 429,
          headers: { "content-type": "application/json" },
        }),
    ) as typeof fetch;
    await expect(api.get("/api/health")).rejects.toBeInstanceOf(ApiError);
    try {
      await api.get("/api/health");
    } catch (err) {
      const e = err as ApiError;
      expect(e.status).toBe(429);
      expect(e.code).toBe("rate_limited");
    }
  });

  it("wraps network errors", async () => {
    globalThis.fetch = vi.fn(async () => {
      throw new TypeError("Failed to fetch");
    }) as typeof fetch;
    await expect(api.get("/api/health")).rejects.toMatchObject({
      status: 0,
      code: "network_error",
    });
  });

  it("returns undefined on 204 no-content", async () => {
    globalThis.fetch = vi.fn(async () => new Response(null, { status: 204 })) as typeof fetch;
    const r = await api.post("/api/auth/logout");
    expect(r).toBeUndefined();
  });
});
