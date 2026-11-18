/* Paper Explainer — offline service worker.
 *
 * Strategy:
 *  - navigations + API GETs: network first, cached copy as offline fallback
 *  - hashed static assets: cache first with background revalidation
 *  - never cache writes or cross-origin requests
 *
 * Service workers only run in secure contexts (HTTPS or localhost). On a plain
 * HTTP address the browser will refuse to register this script; the app still
 * works normally online, it just has no offline cache there.
 */
const VERSION = 'v1';
const SHELL = `paper-explainer-shell-${VERSION}`;
const ASSETS = `paper-explainer-assets-${VERSION}`;
const API = `paper-explainer-api-${VERSION}`;
const CACHES = [SHELL, ASSETS, API];
const APP_SHELL = [
  '/',
  '/index.html',
  '/manifest.webmanifest',
  '/icon.svg',
  '/icon-maskable.svg',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(SHELL)
      .then((cache) => cache.addAll(APP_SHELL))
      .then(() => self.skipWaiting())
      .catch(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(
        keys
          .filter((key) => key.startsWith('paper-explainer-') && !CACHES.includes(key))
          .map((key) => caches.delete(key)),
      ))
      .then(() => self.clients.claim()),
  );
});

async function networkFirst(request, cacheName, fallback) {
  const cache = await caches.open(cacheName);
  try {
    const response = await fetch(request);
    if (response && response.ok) {
      cache.put(request, response.clone());
      if (fallback) cache.put(fallback, response.clone());
    }
    return response;
  } catch (error) {
    const cached = (await cache.match(request)) || (fallback ? await cache.match(fallback) : null);
    if (cached) return cached;
    throw error;
  }
}

async function cacheFirst(request, cacheName) {
  const cache = await caches.open(cacheName);
  const cached = await cache.match(request);
  if (cached) {
    // Revalidate quietly so a redeploy is picked up on the next visit.
    fetch(request)
      .then((response) => { if (response && response.ok) cache.put(request, response.clone()); })
      .catch(() => undefined);
    return cached;
  }
  const response = await fetch(request);
  if (response && response.ok) cache.put(request, response.clone());
  return response;
}

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  // Let the browser manage its own script lifecycle.
  if (url.pathname === '/sw.js') return;

  if (request.mode === 'navigate') {
    event.respondWith(networkFirst(request, SHELL, '/index.html'));
    return;
  }
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(networkFirst(request, API));
    return;
  }
  event.respondWith(cacheFirst(request, ASSETS));
});

self.addEventListener('message', (event) => {
  if (event.data && event.data.type === 'SKIP_WAITING') self.skipWaiting();
});
