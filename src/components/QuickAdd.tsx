import { CalendarClock, Plus } from 'lucide-react';
import { forwardRef, useImperativeHandle, useRef, useState } from 'react';
import { buildTaskInput, EMPTY_FORM, type FormField, type TaskForm } from '../domain/validate';
import type { Task, TaskInput } from '../domain/types';
import { TaskFields } from './TaskFields';

export interface QuickAddHandle {
  focus(): void;
}

interface Props {
  onCreate(input: TaskInput): Promise<Task>;
  defaultLeadMinutes: number | null;
  now: number;
}

export const QuickAdd = forwardRef<QuickAddHandle, Props>(function QuickAdd({ onCreate, defaultLeadMinutes, now }, ref) {
  const [form, setForm] = useState<TaskForm>(EMPTY_FORM);
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<{ field: FormField; message: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useImperativeHandle(ref, () => ({ focus: () => inputRef.current?.focus() }), []);

  const submit = async () => {
    if (saving) return;
    const r = buildTaskInput(form);
    if (!r.ok) {
      setError({ field: r.field, message: r.message });
      if (r.field !== 'title') setOpen(true);
      inputRef.current?.focus();
      return;
    }
    setSaving(true);
    try {
      await onCreate(r.input);
      setForm(EMPTY_FORM);
      setError(null);
      setOpen(false);
    } catch (e) {
      setError({ field: 'title', message: `Couldn't save: ${(e as { message?: string }).message ?? String(e)}` });
    } finally {
      setSaving(false);
      inputRef.current?.focus();
    }
  };

  const titleError = error?.field === 'title' ? error.message : null;

  return (
    <form
      className="border-t border-[var(--hairline)] px-2.5 pt-2 pb-2"
      aria-label="Add a task"
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <div className="flex items-center gap-1.5">
        <label htmlFor="quick-add" className="sr-only">
          New task title
        </label>
        <input
          ref={inputRef}
          id="quick-add"
          className="field"
          placeholder="Add a task…"
          autoComplete="off"
          maxLength={400}
          value={form.title}
          aria-invalid={!!titleError}
          aria-describedby={titleError ? 'quick-add-error' : 'quick-add-hint'}
          onChange={(e) => {
            setForm({ ...form, title: e.target.value });
            if (error?.field === 'title') setError(null);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && e.altKey) {
              e.preventDefault();
              setOpen((o) => !o);
            } else if (e.key === 'Escape' && (form.title || open)) {
              e.preventDefault();
              e.stopPropagation();
              setForm(EMPTY_FORM);
              setOpen(false);
              setError(null);
            }
          }}
        />
        <button
          type="button"
          className="icon-btn shrink-0"
          aria-label={open ? 'Hide date, time and notes' : 'Add date, time and notes'}
          title="Date, time and notes (Alt+Enter)"
          aria-expanded={open}
          aria-controls="quick-add-details"
          data-active={open || Boolean(form.date) || undefined}
          onClick={() => setOpen((o) => !o)}
        >
          <CalendarClock size={17} strokeWidth={2} />
        </button>
        <button type="submit" className="icon-btn shrink-0" data-primary aria-label="Add task" title="Add task (Enter)" disabled={saving}>
          <Plus size={17} strokeWidth={2.4} />
        </button>
      </div>
      {titleError ? (
        <p id="quick-add-error" className="field-error" role="alert">
          {titleError}
        </p>
      ) : (
        <p id="quick-add-hint" className="sr-only">
          Press Enter to add. Alt+Enter shows date, time and notes.
        </p>
      )}
      {open && (
        <div id="quick-add-details" className="mt-2">
          <TaskFields
            form={form}
            onChange={(f) => {
              setForm(f);
              if (error && error.field !== 'title') setError(null);
            }}
            error={error && error.field !== 'title' ? error : null}
            defaultLeadMinutes={defaultLeadMinutes}
            now={now}
          />
        </div>
      )}
    </form>
  );
});
