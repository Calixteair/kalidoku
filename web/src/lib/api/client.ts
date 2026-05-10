import type { paths } from "./types.js";

/**
 * Lightweight typed wrapper around `fetch`.
 *
 * Usage:
 *   const grid = await api.get("/api/grids/{domain}/today", { domain: "paris-metro" });
 *   const game = await api.post("/api/games", undefined, { domain: "paris-metro", mode: "daily" });
 *
 * The first argument is the literal OpenAPI path (with `{}` placeholders).
 * Path params are filled from `params`. Query params and body are typed when present.
 */

export class ApiError extends Error {
  public readonly status: number;
  public readonly code: string;
  public readonly details: unknown;

  public constructor(status: number, code: string, message: string, details?: unknown) {
    super(message);
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

type JsonContent<T> = T extends { content: { "application/json": infer J } } ? J : never;

type GetSuccess<P extends keyof paths, M extends keyof paths[P]> = paths[P][M] extends {
  responses: infer R;
}
  ? R extends { 200: infer S }
    ? JsonContent<S>
    : R extends { 201: infer S }
      ? JsonContent<S>
      : never
  : never;

type GetParams<P extends keyof paths, M extends keyof paths[P]> = paths[P][M] extends {
  parameters: infer Params;
}
  ? Params extends { path?: infer Path }
    ? Path extends Record<string, unknown>
      ? Path
      : Record<string, never>
    : Record<string, never>
  : Record<string, never>;

type GetQuery<P extends keyof paths, M extends keyof paths[P]> = paths[P][M] extends {
  parameters: infer Params;
}
  ? Params extends { query?: infer Q }
    ? Q
    : undefined
  : undefined;

type GetBody<P extends keyof paths, M extends keyof paths[P]> = paths[P][M] extends {
  requestBody?: { content: { "application/json": infer B } };
}
  ? B
  : undefined;

interface ApiOptions {
  signal?: AbortSignal;
  headers?: Record<string, string>;
}

const fillPath = (path: string, params: Record<string, unknown>): string => {
  return path.replace(/\{(\w+)\}/g, (_match, key: string) => {
    const v = params[key];
    if (v === undefined || v === null) {
      throw new Error(`Missing path param: ${key}`);
    }
    return encodeURIComponent(String(v));
  });
};

const buildQuery = (query: Record<string, unknown> | undefined): string => {
  if (!query) return "";
  const usp = new URLSearchParams();
  for (const [k, v] of Object.entries(query)) {
    if (v !== undefined && v !== null) usp.set(k, String(v));
  }
  const s = usp.toString();
  return s ? `?${s}` : "";
};

const parseError = async (res: Response): Promise<ApiError> => {
  let code = "http_error";
  let message = res.statusText || `HTTP ${res.status}`;
  let details: unknown = undefined;
  try {
    const data = (await res.json()) as { code?: string; message?: string; details?: unknown };
    if (data && typeof data === "object") {
      if (typeof data.code === "string") code = data.code;
      if (typeof data.message === "string") message = data.message;
      details = data.details;
    }
  } catch {
    /* ignore non-JSON bodies */
  }
  return new ApiError(res.status, code, message, details);
};

const request = async <T>(
  method: string,
  url: string,
  body: unknown,
  opts: ApiOptions,
): Promise<T> => {
  const init: RequestInit = {
    method,
    credentials: "include",
    headers: {
      Accept: "application/json",
      ...(body !== undefined ? { "Content-Type": "application/json" } : {}),
      ...(opts.headers ?? {}),
    },
    ...(opts.signal !== undefined ? { signal: opts.signal } : {}),
  };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
  }
  let res: Response;
  try {
    res = await fetch(url, init);
  } catch (err) {
    throw new ApiError(0, "network_error", err instanceof Error ? err.message : "Network error");
  }
  if (!res.ok) {
    throw await parseError(res);
  }
  if (res.status === 204) {
    return undefined as T;
  }
  const ct = res.headers.get("content-type") ?? "";
  if (!ct.includes("application/json")) {
    return undefined as T;
  }
  return (await res.json()) as T;
};

export const api = {
  async get<P extends keyof paths & string>(
    path: P,
    params?: GetParams<P, "get">,
    query?: GetQuery<P, "get">,
    opts: ApiOptions = {},
  ): Promise<GetSuccess<P, "get">> {
    const url =
      fillPath(path, (params ?? {}) as Record<string, unknown>) +
      buildQuery(query as Record<string, unknown> | undefined);
    return request<GetSuccess<P, "get">>("GET", url, undefined, opts);
  },

  async post<P extends keyof paths & string>(
    path: P,
    params?: GetParams<P, "post">,
    body?: GetBody<P, "post">,
    opts: ApiOptions = {},
  ): Promise<GetSuccess<P, "post">> {
    const url = fillPath(path, (params ?? {}) as Record<string, unknown>);
    return request<GetSuccess<P, "post">>("POST", url, body, opts);
  },
};

export type { paths };
