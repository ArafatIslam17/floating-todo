import { AlarmClock, BellOff, BellRing, Clock3, Pencil, Trash2, TriangleAlert } from 'lucide-react';
import { useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';
import {
  FUSE_WINDOW_MS,
  formatClock,
  formatCountdown,
  formatDueLabel,
  fuseFraction,
  temporalState,
  type TemporalState,
} from '../domain/time';
import type { Task, TaskInput } from '../domain/types';
import { rowEnter, rowLeave, rowRestore } from '../lib/motion';
import { TaskEditor } from './TaskEditor';

export interface RowActions {
  onToggle(task: Task, completed: boolean): Promise<unknown>;
  onSave(task: Task, input: TaskInput): Promise<unknown>;
  onDelete(task: Task): Promise<unknown>;
}

interface Props extends RowActions {
  task: Task;
  now: number;
  defaultLeadMinutes: number | null;
  /** Roving tabindex: only one row is in the tab order. */
  tabbable: boolean;
  onFocusRow(id: string): void;
  onMove(id: string, dir: 1 | -1): void;
  editing: boolean;
  setEditing(id: string | null): void;
  onError(message: string): void;
}

const STATE_LABEL: Record<TemporalState, string> = {
  none: '',
  upcoming: 'Upcoming',
  soon: 'Due soon',
  now: 'Due now',
  overdue: 'Overdue',
  completed: 'Completed',
};

function StateIcon({ state, snoozed }: { state: TemporalState; snoozed: boolean }) {
  const p = { size: 13, strokeWidth: 2.2, 'aria-hidden': true } as const;
  if (snoozed) return <BellOff {...p} />;
  switch (state) {
    case 'soon':
      return <AlarmClock {...p} className="text-soon" />;
    case 'now':
      return <BellRing {...p} className="text-overdue" />;
    case 'overdue':
      return <TriangleAlert {...p} className="text-overdue" />;
    default:
      return <Clock3 {...p} />;
  }
}

export function TaskRow(props: Props) {
  const { task, now, editing, setEditing, tabbable } = props;
  const ref = useRef<HTMLLIElement>(null);
  const [busy, setBusy] = useState(false);
  // Shown immediately on click so completion feels instant while the row animates out.
  const [optimisticDone, setOptimisticDone] = useState<boolean | null>(null);
  const state = temporalState(task, now);
  const snoozed = task.snoozedUntilUtc != null && task.status === 'pending';

  // A row mounting for a task that just changed (created, restored, completed, reopened) animates in;
  // rows present at startup do not.
  const updatedAt = useRef(task.updatedAtUtc);
  useLayoutEffect(() => {
    if (ref.current && Date.now() - updatedAt.current < 1500) rowEnter(ref.current);
  }, []);

  const leaveThen = async (action: () => Promise<unknown>, failMsg: string) => {
    const el = ref.current;
    if (!el || busy) return;
    setBusy(true);
    // Keyboard users keep their place: focus moves to the neighbouring row.
    if (el.contains(document.activeElement)) props.onMove(task.id, 1);
    await rowLeave(el);
    try {
      await action();
    } catch (e) {
      rowRestore(el);
      setBusy(false);
      setOptimisticDone(null);
      props.onError(`${failMsg}: ${(e as { message?: string }).message ?? String(e)}`);
    }
  };

  const toggle = () => {
    if (busy) return;
    const done = task.status !== 'completed';
    setOptimisticDone(done);
    return leaveThen(() => props.onToggle(task, done), "Couldn't update the task");
  };
  const remove = () => leaveThen(() => props.onDelete(task), "Couldn't delete the task");

  const onKeyDown = (e: KeyboardEvent<HTMLLIElement>) => {
    if (e.target !== e.currentTarget) return;
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        props.onMove(task.id, 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        props.onMove(task.id, -1);
        break;
      case 'Enter':
      case 'e':
      case 'E':
        e.preventDefault();
        setEditing(task.id);
        break;
      case ' ':
        e.preventDefault();
        void toggle();
        break;
      case 'Delete':
      case 'Backspace':
        e.preventDefault();
        void remove();
        break;
    }
  };

  if (editing) {
    return (
      <li ref={ref} className="row" data-state={state} style={{ background: 'var(--surface-raised)' }}>
        <TaskEditor
          task={task}
          now={now}
          defaultLeadMinutes={props.defaultLeadMinutes}
          onCancel={() => {
            setEditing(null);
            requestAnimationFrame(() => ref.current?.focus());
          }}
          onSave={async (input) => {
            await props.onSave(task, input);
            setEditing(null);
            requestAnimationFrame(() => ref.current?.focus());
          }}
        />
      </li>
    );
  }

  const due = task.dueAtUtc;
  const countdown = due != null && task.status === 'pending' ? formatCountdown(due, now) : null;
  const accessibleState = STATE_LABEL[state];

  return (
    <li
      ref={ref}
      className="row"
      data-state={state}
      data-task-id={task.id}
      tabIndex={tabbable ? 0 : -1}
      aria-label={`${task.title}${accessibleState ? `, ${accessibleState}` : ''}${countdown ? `, ${countdown}` : ''}`}
      onFocus={(e) => e.target === e.currentTarget && props.onFocusRow(task.id)}
      onKeyDown={onKeyDown}
    >
      <div className="row-inner">
        <input
          type="checkbox"
          className="check"
          tabIndex={-1}
          checked={optimisticDone ?? task.status === 'completed'}
          aria-busy={busy}
          aria-label={task.status === 'completed' ? `Reopen ${task.title}` : `Complete ${task.title}`}
          title={task.status === 'completed' ? 'Reopen (Space)' : 'Complete (Space)'}
          onChange={() => void toggle()}
        />
        <div className="min-w-0">
          <p className="row-title" title={task.title}>
            {task.title}
          </p>
          {due != null && (
            <div className="row-meta tnum">
              <span className="when">
                <StateIcon state={state} snoozed={snoozed} />
                <span>{formatDueLabel(due, now)}</span>
              </span>
              {countdown && (
                <span className="countdown" data-state={state}>
                  {countdown}
                </span>
              )}
              {snoozed && <span>Snoozed until {formatClock(task.snoozedUntilUtc!)}</span>}
            </div>
          )}
          {task.notes && <p className="row-notes">{task.notes}</p>}
        </div>
        <div className="row-actions">
          <button
            type="button"
            className="icon-btn"
            tabIndex={-1}
            aria-label={`Edit ${task.title}`}
            title="Edit (E)"
            onClick={() => setEditing(task.id)}
          >
            <Pencil size={14} strokeWidth={2} />
          </button>
          <button
            type="button"
            className="icon-btn"
            data-danger
            tabIndex={-1}
            aria-label={`Delete ${task.title}`}
            title="Delete (Del)"
            onClick={() => void remove()}
          >
            <Trash2 size={14} strokeWidth={2} />
          </button>
        </div>
      </div>
      {due != null && task.status === 'pending' && (
        <div
          className="fuse"
          data-state={state}
          data-within-hour={due - now < FUSE_WINDOW_MS}
          aria-hidden="true"
        >
          <div className="fuse-burn" style={{ width: `${fuseFraction(due, now) * 100}%` }} />
        </div>
      )}
    </li>
  );
}
