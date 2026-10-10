import { describe, expect, it } from 'vitest';
import { EMPTY_FORM, buildTaskInput } from './validate';

describe('buildTaskInput', () => {
  it('rejects empty titles with a helpful message', () => {
    expect(buildTaskInput({ ...EMPTY_FORM, title: '   ' })).toEqual({
      ok: false,
      field: 'title',
      message: 'Enter a task title.',
    });
  });

  it('accepts a title-only task', () => {
    const r = buildTaskInput({ ...EMPTY_FORM, title: '  Reply to supervisor ' });
    expect(r).toEqual({
      ok: true,
      input: { title: 'Reply to supervisor', notes: null, dueAtUtc: null, timezoneId: null, leadMinutes: null },
    });
  });

  it('builds a scheduled task with lead time and timezone', () => {
    const r = buildTaskInput({ title: 'Tutor', date: '2027-03-03', time: '17:30', leadMinutes: 15, notes: 'ch 5' });
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.input.dueAtUtc).toBe(Date.UTC(2027, 2, 3, 17, 30));
      expect(r.input.leadMinutes).toBe(15);
      expect(r.input.timezoneId).toBe('Europe/London');
      expect(r.input.notes).toBe('ch 5');
    }
  });

  it('drops lead time when there is no deadline', () => {
    const r = buildTaskInput({ ...EMPTY_FORM, title: 'x', leadMinutes: 10 });
    expect(r.ok && r.input.leadMinutes).toBeNull();
  });

  it('requires both date and time once either is set', () => {
    expect(buildTaskInput({ ...EMPTY_FORM, title: 'x', time: '10:00' })).toMatchObject({ ok: false, field: 'date' });
    expect(buildTaskInput({ ...EMPTY_FORM, title: 'x', date: '2027-03-03' })).toMatchObject({ ok: false, field: 'time' });
  });

  it('explains nonexistent DST times', () => {
    const r = buildTaskInput({ ...EMPTY_FORM, title: 'x', date: '2027-03-28', time: '01:30' });
    expect(r).toMatchObject({ ok: false, field: 'time' });
    expect(!r.ok && r.message).toMatch(/clock change/);
  });

  it('enforces length by characters', () => {
    expect(buildTaskInput({ ...EMPTY_FORM, title: 'ক'.repeat(200) }).ok).toBe(true);
    expect(buildTaskInput({ ...EMPTY_FORM, title: 'a'.repeat(201) }).ok).toBe(false);
    expect(buildTaskInput({ ...EMPTY_FORM, title: 'x', notes: 'n'.repeat(2001) })).toMatchObject({ field: 'notes' });
  });

  it('rejects control characters', () => {
    expect(buildTaskInput({ ...EMPTY_FORM, title: 'a\u0007b' }).ok).toBe(false);
  });
});
