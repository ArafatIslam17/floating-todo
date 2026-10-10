// In-memory backend for browser development and Playwright tests only.
// The UI shows a banner whenever this backend is active.
import { MINUTE } from '../domain/time';
import {
  DEFAULT_SETTINGS,
  type ActiveAlert,
  type Settings,
  type Snapshot,
  type Task,
  type TaskInput,
} from '../domain/types';
import type { Backend, BackendEvents } from './backend';

interface Reminder {
  id: number;
  taskId: string;
  type: 'lead' | 'due';
  at: number;
  status: 'pending' | 'snoozed' | 'fired' | 'acknowledged' | 'cancelled';
  firedAt: number | null;
  snoozedUntil: number | null;
}

const err = (code: string, message: string) => ({ code, message });

export function createPreviewBackend(): Backend {
  const tasks = new Map<string, Task & { deleted: boolean }>();
  let reminders: Reminder[] = [];
  let settings: Settings = { ...DEFAULT_SETTINGS };
  let nextReminderId = 1;
  const listeners = new Set<BackendEvents>();

  const now = () => Date.now();

  const plan = (t: Task) => {
    reminders = reminders.filter((r) => r.taskId !== t.id);
    if (t.status !== 'pending' || t.dueAtUtc == null) return;
    const n = now();
    if (t.leadMinutes && t.dueAtUtc - t.leadMinutes * MINUTE > n) {
      reminders.push({ id: nextReminderId++, taskId: t.id, type: 'lead', at: t.dueAtUtc - t.leadMinutes * MINUTE, status: 'pending', firedAt: null, snoozedUntil: null });
    }
    reminders.push({ id: nextReminderId++, taskId: t.id, type: 'due', at: t.dueAtUtc, status: t.dueAtUtc > n ? 'pending' : 'acknowledged', firedAt: null, snoozedUntil: null });
  };

  const live = () => [...tasks.values()].filter((t) => !t.deleted);

  const alerts = (): ActiveAlert[] =>
    reminders
      .filter((r) => r.status === 'fired')
      .flatMap((r) => {
        const t = tasks.get(r.taskId);
        if (!t || t.deleted || t.status !== 'pending') return [];
        return [{ reminderId: r.id, taskId: t.id, title: t.title, reminderType: r.type, dueAtUtc: t.dueAtUtc, firedAtUtc: r.firedAt ?? 0 }];
      });

  const snapshot = (): Snapshot => ({
    productName: 'Floating Todo',
    version: 'preview',
    tasks: live().map(({ deleted: _d, ...t }) => ({
      ...t,
      snoozedUntilUtc: reminders.find((r) => r.taskId === t.id && r.status === 'snoozed')?.snoozedUntil ?? null,
    })),
    alerts: alerts(),
    settings,
    issues: [],
    dbPath: '(browser preview — nothing is saved)',
    startupError: null,
    nowUtc: now(),
  });

  const emit = () => listeners.forEach((l) => l.onSnapshot(snapshot()));

  const tick = () => {
    const n = now();
    let fired = 0;
    for (const r of reminders) {
      const at = r.status === 'pending' ? r.at : r.status === 'snoozed' ? r.snoozedUntil : null;
      if (at != null && at <= n) {
        r.status = 'fired';
        r.firedAt = n;
        r.snoozedUntil = null;
        fired++;
      }
    }
    if (fired) {
      emit();
      listeners.forEach((l) => l.onAlert({ alerts: alerts(), playedSound: false, count: fired }));
    }
  };
  setInterval(tick, 1000);

  const get = (id: string) => {
    const t = tasks.get(id);
    if (!t || t.deleted) throw err('not_found', 'That task no longer exists.');
    return t;
  };
  const strip = ({ deleted: _d, ...t }: Task & { deleted: boolean }): Task => t;
  const apply = (t: Task & { deleted: boolean }, input: TaskInput) => {
    const title = input.title.trim();
    if (!title) throw err('validation', 'Enter a task title.');
    t.title = title;
    t.notes = input.notes ?? null;
    t.dueAtUtc = input.dueAtUtc ?? null;
    t.timezoneId = input.timezoneId ?? null;
    t.leadMinutes = t.dueAtUtc != null ? (input.leadMinutes ?? null) : null;
    t.updatedAtUtc = now();
  };

  return {
    kind: 'preview',
    getSnapshot: async () => snapshot(),
    createTask: async (input) => {
      const n = now();
      const t = { id: crypto.randomUUID(), title: '', notes: null, dueAtUtc: null, timezoneId: null, leadMinutes: null, status: 'pending' as const, createdAtUtc: n, updatedAtUtc: n, completedAtUtc: null, snoozedUntilUtc: null, deleted: false };
      apply(t, input);
      tasks.set(t.id, t);
      plan(t);
      emit();
      return strip(t);
    },
    updateTask: async (id, input) => {
      const t = get(id);
      const timing = t.dueAtUtc !== (input.dueAtUtc ?? null) || t.leadMinutes !== (input.leadMinutes ?? null);
      apply(t, input);
      if (timing) plan(t);
      emit();
      return strip(t);
    },
    setTaskCompleted: async (id, completed) => {
      const t = get(id);
      t.status = completed ? 'completed' : 'pending';
      t.completedAtUtc = completed ? now() : null;
      plan(t);
      emit();
      return strip(t);
    },
    deleteTask: async (id) => {
      get(id).deleted = true;
      reminders = reminders.filter((r) => r.taskId !== id);
      emit();
    },
    restoreTask: async (id) => {
      const t = tasks.get(id);
      if (!t) throw err('not_found', 'That task can no longer be restored.');
      t.deleted = false;
      plan(t);
      emit();
      return strip(t);
    },
    snoozeAlert: async (rid, minutes) => {
      const r = reminders.find((x) => x.id === rid && x.status === 'fired');
      if (!r) throw err('not_found', 'That alert is no longer active.');
      r.status = 'snoozed';
      r.snoozedUntil = now() + minutes * MINUTE;
      emit();
      return r.snoozedUntil;
    },
    dismissAlert: async (rid) => {
      const r = reminders.find((x) => x.id === rid && x.status === 'fired');
      if (!r) throw err('not_found', 'That alert is no longer active.');
      r.status = 'acknowledged';
      emit();
    },
    updateSettings: async (patch) => {
      settings = { ...settings, ...patch };
      emit();
      return settings;
    },
    testSound: async () => {
      throw err('audio', 'Sound plays only in the desktop app.');
    },
    setCompact: async (compact) => {
      settings = { ...settings, compact };
      emit();
    },
    hideWidget: async () => {},
    quitApp: async () => {},
    retryStartup: async () => snapshot(),
    startResizeDragging: async () => {},
    subscribe: async (ev) => {
      listeners.add(ev);
      return () => listeners.delete(ev);
    },
  };
}
