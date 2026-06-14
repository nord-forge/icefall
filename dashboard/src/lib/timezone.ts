// Single source of truth for "what timezone does this user work in".
//
// The user picks a timezone in their profile preferences (IF-084). Everything
// that shows or accepts a wall-clock time — currently scheduled deploys
// (IF-179) — must interpret it in that zone so the value the user types is the
// value that fires, regardless of the browser's own timezone. UTC is always
// what we store and send to the API; this module only handles the conversion
// at the display/input boundary.

import { api } from './api';

const STORAGE_KEY = 'icefall-timezone';

function browserTimeZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  } catch {
    return 'UTC';
  }
}

let cached: string | null = null;
let inflight: Promise<string> | null = null;

/** Best-known timezone available synchronously (cache → localStorage → browser). */
export function currentTimeZone(): string {
  if (cached) return cached;
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) return stored;
  } catch {
    /* localStorage unavailable */
  }
  return browserTimeZone();
}

/** Persist the preferred timezone so other pages resolve it synchronously. */
export function rememberTimeZone(tz: string): void {
  cached = tz;
  try {
    localStorage.setItem(STORAGE_KEY, tz);
  } catch {
    /* ignore */
  }
}

/** Fetch the user's preferred timezone once and cache it. Falls back gracefully. */
export async function loadTimeZone(): Promise<string> {
  if (cached) return cached;
  if (!inflight) {
    inflight = api
      .getPreferences()
      .then(({ data }) => {
        const tz = (data.timezone as string) || browserTimeZone();
        rememberTimeZone(tz);
        return tz;
      })
      .catch(() => currentTimeZone());
  }
  return inflight;
}

/** Wall-clock components of an instant as displayed in `tz`. */
function partsInTimeZone(date: Date, tz: string) {
  const dtf = new Intl.DateTimeFormat('en-CA', {
    timeZone: tz,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });
  const map: Record<string, string> = {};
  for (const p of dtf.formatToParts(date)) map[p.type] = p.value;
  return {
    y: +map.year,
    mo: +map.month,
    d: +map.day,
    // Intl can report midnight as "24" in some engines.
    h: +(map.hour === '24' ? '0' : map.hour),
    mi: +map.minute,
    s: +map.second,
  };
}

/** Milliseconds `tz` is ahead of UTC at the given instant. */
function offsetMs(date: Date, tz: string): number {
  const p = partsInTimeZone(date, tz);
  const asUtc = Date.UTC(p.y, p.mo - 1, p.d, p.h, p.mi, p.s);
  return asUtc - date.getTime();
}

/**
 * A `datetime-local` value (`YYYY-MM-DDTHH:mm`) read as wall-clock time in `tz`,
 * converted to a UTC ISO 8601 string. Two-step offset inversion keeps it correct
 * across DST boundaries (ambiguous DST-fold times resolve to one side).
 */
export function wallTimeToUtcIso(local: string, tz: string): string {
  const [datePart, timePart = '00:00'] = local.split('T');
  const [y, mo, d] = datePart.split('-').map(Number);
  const [h, mi] = timePart.split(':').map(Number);
  const naiveUtc = Date.UTC(y, mo - 1, d, h, mi);
  const offset1 = offsetMs(new Date(naiveUtc), tz);
  const offset2 = offsetMs(new Date(naiveUtc - offset1), tz);
  return new Date(naiveUtc - offset2).toISOString();
}

/** A UTC ISO string as a `datetime-local` wall-clock value (`YYYY-MM-DDTHH:mm`) in `tz`. */
export function utcIsoToWallTime(iso: string, tz: string): string {
  const p = partsInTimeZone(new Date(iso), tz);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${p.y}-${pad(p.mo)}-${pad(p.d)}T${pad(p.h)}:${pad(p.mi)}`;
}

/** Human-readable instant in `tz`, e.g. "Jun 15, 2026, 2:30 PM CEST". */
export function formatInTimeZone(iso: string, tz: string): string {
  return new Date(iso).toLocaleString(undefined, {
    timeZone: tz,
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZoneName: 'short',
  });
}
