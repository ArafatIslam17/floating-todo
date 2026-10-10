// Pure temporal logic. Every function takes `now` explicitly so it is deterministic in tests.
import type { Task } from './types';

export const MINUTE = 60_000;
export const HOUR = 60 * MINUTE;
export const DAY = 24 * HOUR;
/** "Due soon" window. */
export const SOON_MS = 30 * MINUTE;
/** "Due now" lasts for the first minute after the deadline. */
export const DUE_NOW_MS = MINUTE;
/** The fuse starts shrinking during the last hour. */
export const FUSE_WINDOW_MS = HOUR;

export type TemporalState = 'none' | 'upcoming' | 'soon' | 'now' | 'overdue' | 'completed';

export function temporalState(task: Pick<Task, 'status' | 'dueAtUtc'>, now: number): TemporalState {
  if (task.status === 'completed') return 'completed';
  if (task.dueAtUtc == null) return 'none';
  const d = task.dueAtUtc - now;
  if (d > SOON_MS) return 'upcoming';
  if (d > 0) return 'soon';
  if (d > -DUE_NOW_MS) return 'now';
  return 'overdue';
}

function hm(totalMinutes: number): string {
  const h = Math.floor(totalMinutes / 60);
  const m = totalMinutes % 60;
  return m === 0 ? `${h} hr` : `${h} hr ${m} min`;
}

/** "in 25 min", "in 2 hr 10 min", "in 3 days", "Due now", "Overdue 12 min". */
export function formatCountdown(dueAtUtc: number, now: number): string {
  const d = dueAtUtc - now;
  if (d > 0) {
    const mins = Math.ceil(d / MINUTE);
    if (mins < 60) return `in ${mins} min`;
    // Minutes matter for the next few hours; beyond that, whole hours read faster.
    if (d < 3 * HOUR) return `in ${hm(Math.round(d / MINUTE))}`;
    if (d < DAY) return `in ${Math.round(d / HOUR)} hr`;
    const days = Math.floor(d / DAY);
    return days === 1 ? 'in 1 day' : `in ${days} days`;
  }
  if (d > -DUE_NOW_MS) return 'Due now';
  const late = -d;
  const mins = Math.floor(late / MINUTE);
  if (mins < 60) return `Overdue ${mins} min`;
  if (late < 3 * HOUR) return `Overdue ${hm(mins)}`;
  if (late < DAY) return `Overdue ${Math.floor(late / HOUR)} hr`;
  const days = Math.floor(late / DAY);
  return days === 1 ? 'Overdue 1 day' : `Overdue ${days} days`;
}

function startOfLocalDay(ms: number): number {
  const d = new Date(ms);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** Whole local calendar days between two instants (DST-safe: compares calendar dates). */
export function calendarDayDiff(a: number, b: number): number {
  const da = new Date(startOfLocalDay(a));
  const db = new Date(startOfLocalDay(b));
  const ua = Date.UTC(da.getFullYear(), da.getMonth(), da.getDate());
  const ub = Date.UTC(db.getFullYear(), db.getMonth(), db.getDate());
  return Math.round((ub - ua) / DAY);
}

// Formatters are created per call (cheap at this volume) so a timezone change on the computer
// is reflected immediately; only the locale's hour-cycle preference is cached.
const twentyFourHour = new Map<string, boolean>();

/** "17:30" / "09:00" in 24-hour locales, "5:30 PM" in 12-hour locales. */
export function formatClock(ms: number, locale?: string): string {
  const key = locale ?? '';
  let h24 = twentyFourHour.get(key);
  if (h24 === undefined) {
    const cycle = new Intl.DateTimeFormat(locale, { hour: 'numeric' }).resolvedOptions().hourCycle;
    h24 = cycle === 'h23' || cycle === 'h24';
    twentyFourHour.set(key, h24);
  }
  return new Intl.DateTimeFormat(locale, { hour: h24 ? '2-digit' : 'numeric', minute: '2-digit' }).format(ms);
}

/** "Today 17:30", "Tomorrow 09:00", "Yesterday 23:59", "Mon 17:30", "Mon 14 Oct 17:30", "14 Oct 2027 17:30". */
export function formatDueLabel(dueAtUtc: number, now: number, locale?: string): string {
  const time = formatClock(dueAtUtc, locale);
  const diff = calendarDayDiff(now, dueAtUtc);
  if (diff === 0) return `Today ${time}`;
  if (diff === 1) return `Tomorrow ${time}`;
  if (diff === -1) return `Yesterday ${time}`;
  const due = new Date(dueAtUtc);
  if (diff > 1 && diff < 7) {
    return `${new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(due)} ${time}`;
  }
  const sameYear = due.getFullYear() === new Date(now).getFullYear();
  const date = new Intl.DateTimeFormat(locale, {
    weekday: sameYear ? 'short' : undefined,
    day: 'numeric',
    month: 'short',
    year: sameYear ? undefined : 'numeric',
  }).format(due);
  return `${date} ${time}`;
}

/** Fraction of the fuse remaining (1 = full, 0 = burnt). */
export function fuseFraction(dueAtUtc: number, now: number): number {
  const d = dueAtUtc - now;
  if (d >= FUSE_WINDOW_MS) return 1;
  if (d <= 0) return 0;
  return d / FUSE_WINDOW_MS;
}

/**
 * Milliseconds until the UI should next recompute temporal labels: the next minute boundary,
 * or the next 15-second boundary while any deadline is within two minutes.
 */
export function msUntilNextRefresh(tasks: Pick<Task, 'status' | 'dueAtUtc'>[], now: number): number {
  const close = tasks.some(
    (t) => t.status === 'pending' && t.dueAtUtc != null && Math.abs(t.dueAtUtc - now) <= 2 * MINUTE,
  );
  const step = close ? 15_000 : MINUTE;
  return step - (now % step) + 5;
}

export type LocalToUtc = { ok: true; utc: number } | { ok: false; reason: 'invalid' | 'nonexistent' };

/**
 * Convert a local wall-clock date ("YYYY-MM-DD") and time ("HH:MM") to a UTC instant using the
 * system timezone. Times skipped by a DST change are rejected; ambiguous times (clocks going
 * back) resolve to the earlier occurrence, which is what the platform's Date does.
 */
export function localToUtc(date: string, time: string): LocalToUtc {
  const dm = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date);
  const tm = /^(\d{2}):(\d{2})$/.exec(time);
  if (!dm || !tm) return { ok: false, reason: 'invalid' };
  const [y, mo, d] = [Number(dm[1]), Number(dm[2]), Number(dm[3])];
  const [h, mi] = [Number(tm[1]), Number(tm[2])];
  if (mo < 1 || mo > 12 || d < 1 || d > 31 || h > 23 || mi > 59) return { ok: false, reason: 'invalid' };
  const dt = new Date(y, mo - 1, d, h, mi, 0, 0);
  if (dt.getFullYear() !== y || dt.getMonth() !== mo - 1 || dt.getDate() !== d) {
    return { ok: false, reason: 'invalid' };
  }
  if (dt.getHours() !== h || dt.getMinutes() !== mi) return { ok: false, reason: 'nonexistent' };
  return { ok: true, utc: dt.getTime() };
}

/** Inverse of localToUtc for filling form fields. */
export function utcToLocalParts(utc: number): { date: string; time: string } {
  const d = new Date(utc);
  const p = (n: number) => String(n).padStart(2, '0');
  return {
    date: `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`,
    time: `${p(d.getHours())}:${p(d.getMinutes())}`,
  };
}

export function localTimezoneId(): string | null {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone ?? null;
  } catch {
    return null;
  }
}
