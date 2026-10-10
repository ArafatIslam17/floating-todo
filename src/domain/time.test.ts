import { describe, expect, it } from 'vitest';
import {
  DAY,
  HOUR,
  MINUTE,
  calendarDayDiff,
  formatCountdown,
  formatDueLabel,
  fuseFraction,
  localToUtc,
  msUntilNextRefresh,
  temporalState,
  utcToLocalParts,
} from './time';

// Wednesday 3 March 2027, 10:00 London (GMT).
const NOW = Date.UTC(2027, 2, 3, 10, 0, 0);

describe('temporalState', () => {
  const t = (due: number | null, status: 'pending' | 'completed' = 'pending') => ({ dueAtUtc: due, status });
  it('covers every state at its boundaries', () => {
    expect(temporalState(t(null), NOW)).toBe('none');
    expect(temporalState(t(NOW + 31 * MINUTE), NOW)).toBe('upcoming');
    expect(temporalState(t(NOW + 30 * MINUTE), NOW)).toBe('soon');
    expect(temporalState(t(NOW + 1), NOW)).toBe('soon');
    expect(temporalState(t(NOW), NOW)).toBe('now');
    expect(temporalState(t(NOW - 59_999), NOW)).toBe('now');
    expect(temporalState(t(NOW - MINUTE), NOW)).toBe('overdue');
    expect(temporalState(t(NOW - DAY, 'completed'), NOW)).toBe('completed');
  });
});

describe('formatCountdown', () => {
  it.each([
    [25 * MINUTE, 'in 25 min'],
    [25 * MINUTE - 30_000, 'in 25 min'],
    [30_000, 'in 1 min'],
    [2 * HOUR, 'in 2 hr'],
    [2 * HOUR + 10 * MINUTE, 'in 2 hr 10 min'],
    [17 * HOUR + 59 * MINUTE, 'in 18 hr'],
    [DAY + HOUR, 'in 1 day'],
    [3 * DAY + 5 * HOUR, 'in 3 days'],
    [0, 'Due now'],
    [-30_000, 'Due now'],
    [-12 * MINUTE, 'Overdue 12 min'],
    [-(HOUR + 5 * MINUTE), 'Overdue 1 hr 5 min'],
    [-(5 * HOUR + 40 * MINUTE), 'Overdue 5 hr'],
    [-2 * DAY, 'Overdue 2 days'],
  ])('offset %i → %s', (offset, text) => {
    expect(formatCountdown(NOW + offset, NOW)).toBe(text);
  });
});

describe('formatClock', () => {
  it('follows the locale convention for 12/24-hour time', () => {
    expect(formatDueLabel(Date.UTC(2027, 2, 3, 17, 30), NOW, 'en-US')).toBe('Today 5:30 PM');
  });
});

describe('formatDueLabel', () => {
  const L = 'en-GB';
  it('uses relative day names near today', () => {
    expect(formatDueLabel(Date.UTC(2027, 2, 3, 17, 30), NOW, L)).toBe('Today 17:30');
    expect(formatDueLabel(Date.UTC(2027, 2, 4, 9, 0), NOW, L)).toBe('Tomorrow 09:00');
    expect(formatDueLabel(Date.UTC(2027, 2, 2, 23, 59), NOW, L)).toBe('Yesterday 23:59');
  });
  it('uses weekday within a week and full date beyond', () => {
    expect(formatDueLabel(Date.UTC(2027, 2, 6, 8, 0), NOW, L)).toBe('Sat 08:00');
    expect(formatDueLabel(Date.UTC(2027, 2, 15, 8, 0), NOW, L)).toBe('Mon 15 Mar 08:00');
    expect(formatDueLabel(Date.UTC(2028, 0, 5, 8, 0), NOW, L)).toBe('5 Jan 2028 08:00');
  });
  it('counts calendar days correctly across the DST change', () => {
    // Sat 27 Mar 23:30 GMT → Sun 28 Mar 23:30 BST is one calendar day, though only 23 hours.
    const before = Date.UTC(2027, 2, 27, 23, 30);
    const after = Date.UTC(2027, 2, 28, 22, 30);
    expect(after - before).toBe(23 * HOUR);
    expect(calendarDayDiff(before, after)).toBe(1);
    expect(formatDueLabel(after, before, L)).toBe('Tomorrow 23:30');
  });
});

describe('localToUtc', () => {
  it('converts local wall time with the current offset', () => {
    expect(localToUtc('2027-03-03', '17:30')).toEqual({ ok: true, utc: Date.UTC(2027, 2, 3, 17, 30) });
    // Summer time: 17:30 BST = 16:30 UTC
    expect(localToUtc('2027-07-01', '17:30')).toEqual({ ok: true, utc: Date.UTC(2027, 6, 1, 16, 30) });
  });
  it('rejects a time skipped by the spring-forward change', () => {
    expect(localToUtc('2027-03-28', '01:30')).toEqual({ ok: false, reason: 'nonexistent' });
  });
  it('resolves an ambiguous fall-back time to the earlier occurrence', () => {
    const r = localToUtc('2027-10-31', '01:30');
    expect(r).toEqual({ ok: true, utc: Date.UTC(2027, 9, 31, 0, 30) });
  });
  it('rejects malformed and impossible dates', () => {
    expect(localToUtc('2027-02-30', '10:00').ok).toBe(false);
    expect(localToUtc('2027-13-01', '10:00').ok).toBe(false);
    expect(localToUtc('03/03/2027', '10:00').ok).toBe(false);
    expect(localToUtc('2027-03-03', '24:00').ok).toBe(false);
  });
  it('round-trips through utcToLocalParts', () => {
    const r = localToUtc('2027-07-01', '08:05');
    expect(r.ok && utcToLocalParts(r.utc)).toEqual({ date: '2027-07-01', time: '08:05' });
  });
});

describe('fuseFraction', () => {
  it('is full beyond an hour, shrinks within it, and empties at the deadline', () => {
    expect(fuseFraction(NOW + 2 * HOUR, NOW)).toBe(1);
    expect(fuseFraction(NOW + 30 * MINUTE, NOW)).toBeCloseTo(0.5);
    expect(fuseFraction(NOW - MINUTE, NOW)).toBe(0);
  });
});

describe('msUntilNextRefresh', () => {
  it('aligns to the next minute normally', () => {
    const now = NOW + 20_000;
    expect(msUntilNextRefresh([], now)).toBe(40_005);
  });
  it('refreshes every 15 s when a deadline is within two minutes', () => {
    const now = NOW + 20_000;
    expect(msUntilNextRefresh([{ status: 'pending', dueAtUtc: NOW + 90_000 }], now)).toBe(10_005);
    expect(msUntilNextRefresh([{ status: 'completed', dueAtUtc: NOW + 90_000 }], now)).toBe(40_005);
  });
});
