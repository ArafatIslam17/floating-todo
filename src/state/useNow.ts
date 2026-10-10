import { useEffect, useState } from 'react';
import { msUntilNextRefresh } from '../domain/time';
import type { Task } from '../domain/types';

/**
 * Current time, refreshed on minute boundaries (every 15 s near a deadline), and immediately
 * when the window regains focus or becomes visible (e.g. after the computer wakes).
 * One timer for the whole list — never one per task.
 */
export function useNow(tasks: Pick<Task, 'status' | 'dueAtUtc'>[]): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = window.setTimeout(() => setNow(Date.now()), msUntilNextRefresh(tasks, now));
    return () => window.clearTimeout(id);
  }, [tasks, now]);

  useEffect(() => {
    const refresh = () => setNow(Date.now());
    const onVisible = () => {
      if (document.visibilityState === 'visible') refresh();
    };
    window.addEventListener('focus', refresh);
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      window.removeEventListener('focus', refresh);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, []);

  return now;
}
