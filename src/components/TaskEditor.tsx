import { useEffect, useRef, useState } from 'react';
import { utcToLocalParts } from '../domain/time';
import type { Task, TaskInput } from '../domain/types';
import { buildTaskInput, type FormField, type TaskForm } from '../domain/validate';
import { TaskFields } from './TaskFields';

interface Props {
  task: Task;
  now: number;
  defaultLeadMinutes: number | null;
  onSave(input: TaskInput): Promise<void>;
  onCancel(): void;
}

export function TaskEditor({ task, now, defaultLeadMinutes, onSave, onCancel }: Props) {
  const parts = task.dueAtUtc != null ? utcToLocalParts(task.dueAtUtc) : { date: '', time: '' };
  const [form, setForm] = useState<TaskForm>({
    title: task.title,
    date: parts.date,
    time: parts.time,
    leadMinutes: task.leadMinutes,
    notes: task.notes ?? '',
  });
  const [error, setError] = useState<{ field: FormField; message: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const titleRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    titleRef.current?.focus();
    titleRef.current?.select();
  }, []);

  const save = async () => {
    if (saving) return;
    const r = buildTaskInput(form);
    if (!r.ok) {
      setError({ field: r.field, message: r.message });
      return;
    }
    setSaving(true);
    try {
      await onSave(r.input);
    } catch (e) {
      setError({ field: 'title', message: `Couldn't save: ${(e as { message?: string }).message ?? String(e)}` });
      setSaving(false);
    }
  };

  const titleErr = error?.field === 'title' ? error.message : null;

  return (
    <form
      className="grid gap-2 p-2.5"
      aria-label={`Edit ${task.title}`}
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          onCancel();
        } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
          e.preventDefault();
          void save();
        }
      }}
    >
      <div>
        <label htmlFor={`edit-title-${task.id}`} className="sr-only">
          Title
        </label>
        <input
          ref={titleRef}
          id={`edit-title-${task.id}`}
          className="field font-semibold"
          value={form.title}
          maxLength={400}
          aria-invalid={!!titleErr}
          onChange={(e) => {
            setForm({ ...form, title: e.target.value });
            if (error?.field === 'title') setError(null);
          }}
        />
        {titleErr && (
          <p className="field-error" role="alert">
            {titleErr}
          </p>
        )}
      </div>
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
      <div className="flex items-center justify-end gap-1.5">
        <span className="mr-auto text-small text-ink-faint">Ctrl+Enter to save</span>
        <button type="button" className="text-btn" onClick={onCancel}>
          Cancel
        </button>
        <button type="submit" className="text-btn" data-primary disabled={saving}>
          Save
        </button>
      </div>
    </form>
  );
}
