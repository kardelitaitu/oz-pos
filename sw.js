/* KDS prototype service worker — cache-first with network fallback.
   Enables offline play and satisfies Chrome's installability check
   (needs HTTPS in the browser — use the tunnel URL for full install). */
const CACHE = 'kds-proto-v1';
self.addEventListener('install', (e) => {
  e.waitUntil(
    caches.open(CACHE)
      .then(c => c.addAll([
        '/kds-prototype.html',
        '/kds-pwa/icon-192.png',
        '/kds-pwa/icon-512.png',
      ]))
      .then(() => self.skipWaiting())
  );
});
self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches.keys()
      .then(keys => Promise.all(keys.filter(k => k !== CACHE).map(k => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});
self.addEventListener('fetch', (e) => {
  if (e.request.method !== 'GET') return;
  e.respondWith(
    caches.match(e.request).then(cached =>
      cached || fetch(e.request).then((res) => {
        const copy = res.clone();
        caches.open(CACHE).then(c => c.put(e.request, copy));
        return res;
      })
    )
  );
});
