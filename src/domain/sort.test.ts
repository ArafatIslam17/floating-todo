import { describe, expect, it } from 'vitest';
import { countOverdue, sortAlerts, sortCompleted, sortPending } from './sort';
import type { ActiveAlert, Task } from './types';

const base: Task = {
  id: '',
  title: '',
  notes: null,
  dueAtUtc: null,
  timezoneId: null,
  leadMinutes: null,
  status: 'pending',
  createdAtUtc: 0,
  updatedAtUtc: 0,
  completedAtUtc: null,
  snoozedUntilUtc: null,
};
const t = (id: string, o: Partial<Task>): Task => ({ ...base, id, title: id, ...o });

describe('sorting', () => {
  it('orders overdue, then upcoming, then unscheduled newest first', () => {
    const tasks = [
      t('noTimeOld', { createdAtUtc: 1 }),
      t('later', { dueAtUtc: 300 }),
      t('overdue', { dueAtUtc: 50 }),
      t('noTimeNew', { createdAtUtc: 9 }),
      t('done', { status: 'completed', completedAtUtc: 5 }),
      t('soon', { dueAtUtc: 200 }),
    ];
    expect(sortPending(tasks).map((x) => x.id)).toEqual(['overdue', 'soon', 'later', 'noTimeNew', 'noTimeOld']);
    expect(sortCompleted(tasks).map((x) => x.id)).toEqual(['done']);
    expect(countOverdue(tasks, 100)).toBe(1);
  });

  it('puts due alerts before lead warnings', () => {
    const a = (id: number, type: 'lead' | 'due', fired: number): ActiveAlert => ({
      reminderId: id,
      taskId: String(id),
      title: '',
      reminderType: type,
      dueAtUtc: null,
      firedAtUtc: fired,
    });
    expect(sortAlerts([a(1, 'lead', 1), a(2, 'due', 5), a(3, 'due', 2)]).map((x) => x.reminderId)).toEqual([3, 2, 1]);
  });
});
