import { useId } from 'react';
import { LEAD_OPTIONS } from '../domain/types';
import type { FormField, TaskForm } from '../domain/validate';
import { utcToLocalParts } from '../domain/time';

interface Props {
  form: TaskForm;
  onChange(next: TaskForm): void;
  error: { field: FormField; message: string } | null;
  defaultLeadMinutes: number | null;
  now: number;
}

/** Date, time, reminder and notes. Title is rendered by the parent. */
export function TaskFields({ form, onChange, error, defaultLeadMinutes, now }: Props) {
  const id = useId();
  const set = (patch: Partial<TaskForm>) => onChange({ ...form, ...patch });
  const errFor = (f: FormField) => (error?.field === f ? error.message : null);

  const onDate = (date: string) => {
    // Picking a date first fills a visible default time and the default reminder.
    const patch: Partial<TaskForm> = { date };
    if (date && !form.time) patch.time = '09:00';
    if (date && !form.date && form.leadMinutes == null) patch.leadMinutes = defaultLeadMinutes;
    if (!date) {
      patch.time = '';
      patch.leadMinutes = null;
    }
    set(patch);
  };
  const onTime = (time: string) => {
    const patch: Partial<TaskForm> = { time };
    if (time && !form.date) {
      patch.date = utcToLocalParts(now).date;
      if (form.leadMinutes == null) patch.leadMinutes = defaultLeadMinutes;
    }
    set(patch);
  };

  const scheduled = Boolean(form.date || form.time);
  const dateErr = errFor('date');
  const timeErr = errFor('time');
  const notesErr = errFor('notes');

  return (
    <div className="grid gap-2">
      <div className="grid grid-cols-[1fr_auto] gap-2">
        <div>
          <label htmlFor={`${id}-date`} className="sr-only">
            Date
          </label>
          <input
            id={`${id}-date`}
            type="date"
            className="field tnum"
            value={form.date}
            min="2000-01-01"
            max="2099-12-31"
            aria-invalid={!!dateErr}
            aria-describedby={dateErr ? `${id}-err` : undefined}
            onChange={(e) => onDate(e.target.value)}
          />
        </div>
        <div>
          <label htmlFor={`${id}-time`} className="sr-only">
            Time
          </label>
          <input
            id={`${id}-time`}
            type="time"
            className="field tnum w-[92px]"
            value={form.time}
            aria-invalid={!!timeErr}
            aria-describedby={timeErr ? `${id}-err` : undefined}
            onChange={(e) => onTime(e.target.value)}
          />
        </div>
      </div>
      {(dateErr || timeErr) && (
        <p id={`${id}-err`} className="field-error" role="alert">
          {dateErr ?? timeErr}
        </p>
      )}
      <div className="flex items-center gap-2">
        <label htmlFor={`${id}-lead`} className="shrink-0 text-meta font-semibold text-ink-muted">
          Remind me
        </label>
        <select
          id={`${id}-lead`}
          className="field"
          disabled={!scheduled}
          value={form.leadMinutes ?? ''}
          onChange={(e) => set({ leadMinutes: e.target.value ? Number(e.target.value) : null })}
        >
          <option value="">{scheduled ? 'At the due time only' : 'Set a date first'}</option>
          {LEAD_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </div>
      <div>
        <label htmlFor={`${id}-notes`} className="sr-only">
          Notes
        </label>
        <textarea
          id={`${id}-notes`}
          className="field"
          placeholder="Notes (optional)"
          rows={2}
          value={form.notes}
          aria-invalid={!!notesErr}
          onChange={(e) => set({ notes: e.target.value })}
        />
        {notesErr && <p className="field-error">{notesErr}</p>}
      </div>
    </div>
  );
}
