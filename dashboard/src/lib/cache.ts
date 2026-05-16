/**
 * Simple in-memory cache for API GET responses.
 * Entries expire after TTL milliseconds and are lazily evicted on read.
 */

const store = new Map<string, { data: unknown; timestamp: number }>();
const TTL = 30_000; // 30 seconds

export function getCached<T>(key: string): T | null {
  const entry = store.get(key);
  if (!entry) return null;
  if (Date.now() - entry.timestamp > TTL) {
    store.delete(key);
    return null;
  }
  return entry.data as T;
}

export function setCache(key: string, data: unknown): void {
  store.set(key, { data, timestamp: Date.now() });
}

export function invalidateCache(key: string): void {
  store.delete(key);
}

/**
 * Remove all cache entries whose key starts with the given prefix.
 * Useful after mutations (e.g. invalidatePrefix('/apps') after creating an app).
 */
export function invalidatePrefix(prefix: string): void {
  for (const key of store.keys()) {
    if (key.startsWith(prefix)) store.delete(key);
  }
}

export function clearAll(): void {
  store.clear();
}

/** Periodic sweep to remove stale entries that were never read. */
const SWEEP_INTERVAL = 60_000;
setInterval(() => {
  const now = Date.now();
  for (const [key, entry] of store) {
    if (now - entry.timestamp > TTL) store.delete(key);
  }
}, SWEEP_INTERVAL);

/** Clear cache on View Transition navigation so pages always get fresh data. */
if (typeof document !== 'undefined') {
  document.addEventListener('astro:before-swap', () => store.clear());
}
