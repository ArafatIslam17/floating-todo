import type { ActiveAlert, Task } from './types';

/** Scheduled pending tasks by deadline (overdue naturally first), then unscheduled newest first. */
export function sortPending(tasks: Task[]): Task[] {
  return tasks
    .filter((t) => t.status === 'pending')
    .sort((a, b) => {
      if (a.dueAtUtc != null && b.dueAtUtc != null) return a.dueAtUtc - b.dueAtUtc || a.createdAtUtc - b.createdAtUtc;
      if (a.dueAtUtc != null) return -1;
      if (b.dueAtUtc != null) return 1;
      return b.createdAtUtc - a.createdAtUtc;
    });
}

export function sortCompleted(tasks: Task[]): Task[] {
  return tasks
    .filter((t) => t.status === 'completed')
    .sort((a, b) => (b.completedAtUtc ?? 0) - (a.completedAtUtc ?? 0));
}

export function countOverdue(tasks: Task[], now: number): number {
  return tasks.filter((t) => t.status === 'pending' && t.dueAtUtc != null && t.dueAtUtc <= now).length;
}

/** Due alerts first (more urgent than lead warnings), oldest first within each. */
export function sortAlerts(alerts: ActiveAlert[]): ActiveAlert[] {
  return [...alerts].sort((a, b) => {
    if (a.reminderType !== b.reminderType) return a.reminderType === 'due' ? -1 : 1;
    return a.firedAtUtc - b.firedAtUtc;
  });
}
