// kalidoku service worker — minimal precache + cache-first on the daily grid.
// Plain JS for direct browser execution. No build step.

const SHELL_CACHE = "kalidoku-shell-v1";
const API_CACHE = "kalidoku-api-v1";
const SHELL_ASSETS = ["/", "/manifest.webmanifest", "/favicon.svg"];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(SHELL_CACHE).then((cache) =>
      cache.addAll(SHELL_ASSETS).catch(() => {
        // never fail install over a missing optional asset
        return undefined;
      }),
    ),
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys.filter((k) => k !== SHELL_CACHE && k !== API_CACHE).map((k) => caches.delete(k)),
        ),
      ),
  );
  self.clients.claim();
});

const isTodayGrid = (url) => /\/api\/grids\/[a-z0-9-]+\/today$/.test(url.pathname);

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);

  // Cache-first on today's grid: makes the home work offline once visited.
  if (isTodayGrid(url)) {
    event.respondWith(
      caches.open(API_CACHE).then(async (cache) => {
        const cached = await cache.match(req);
        const network = fetch(req)
          .then((res) => {
            if (res.ok) cache.put(req, res.clone());
            return res;
          })
          .catch(() => cached);
        return cached ?? network;
      }),
    );
    return;
  }

  // Stale-while-revalidate on shell navigations.
  if (req.mode === "navigate") {
    event.respondWith(
      caches.open(SHELL_CACHE).then(async (cache) => {
        const cached = await cache.match(req);
        const network = fetch(req)
          .then((res) => {
            if (res.ok) cache.put(req, res.clone());
            return res;
          })
          .catch(() => cached ?? cache.match("/"));
        return cached ?? network;
      }),
    );
  }
});
