// UX validation. The Rust backend repeats these checks authoritatively.
import { localTimezoneId, localToUtc } from './time';
import type { TaskInput } from './types';

export const TITLE_MAX = 200;
export const NOTES_MAX = 2000;

export interface TaskForm {
  title: string;
  date: string; // YYYY-MM-DD or ''
  time: string; // HH:MM or ''
  leadMinutes: number | null;
  notes: string;
}

export const EMPTY_FORM: TaskForm = { title: '', date: '', time: '', leadMinutes: null, notes: '' };

export type FormField = 'title' | 'date' | 'time' | 'notes';
export type FormResult = { ok: true; input: TaskInput } | { ok: false; field: FormField; message: string };

const CONTROL = /\p{Cc}/u;

export function buildTaskInput(form: TaskForm): FormResult {
  const title = form.title.trim();
  if (!title) return { ok: false, field: 'title', message: 'Enter a task title.' };
  if ([...title].length > TITLE_MAX)
    return { ok: false, field: 'title', message: `Keep the title under ${TITLE_MAX} characters.` };
  if (CONTROL.test(title))
    return { ok: false, field: 'title', message: 'The title contains unsupported characters.' };

  const notes = form.notes.trim();
  if ([...notes].length > NOTES_MAX)
    return { ok: false, field: 'notes', message: `Keep notes under ${NOTES_MAX} characters.` };

  let dueAtUtc: number | null = null;
  if (form.date || form.time) {
    if (!form.date) return { ok: false, field: 'date', message: 'Pick a date for this time.' };
    if (!form.time) return { ok: false, field: 'time', message: 'Pick a time for this date.' };
    const r = localToUtc(form.date, form.time);
    if (!r.ok) {
      return r.reason === 'nonexistent'
        ? { ok: false, field: 'time', message: "That time doesn't exist on this date (clock change). Choose another time." }
        : { ok: false, field: 'date', message: 'Enter a valid date and time.' };
    }
    const year = new Date(r.utc).getFullYear();
    if (year < 2000 || year >= 2100) return { ok: false, field: 'date', message: 'Choose a date between 2000 and 2100.' };
    dueAtUtc = r.utc;
  }

  return {
    ok: true,
    input: {
      title,
      notes: notes || null,
      dueAtUtc,
      timezoneId: dueAtUtc != null ? localTimezoneId() : null,
      leadMinutes: dueAtUtc != null ? form.leadMinutes : null,
    },
  };
}
