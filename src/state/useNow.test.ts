import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useNow } from './useNow';

describe('useNow', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(Date.UTC(2027, 2, 3, 10, 0, 20)));
  });
  afterEach(() => vi.useRealTimers());

  it('updates on the next minute boundary, not before', () => {
    const tasks: never[] = [];
    const { result } = renderHook(() => useNow(tasks));
    const start = result.current;
    act(() => vi.advanceTimersByTime(39_000));
    expect(result.current).toBe(start);
    act(() => vi.advanceTimersByTime(2_000));
    expect(result.current).toBeGreaterThan(start);
  });

  it('keeps exactly one timer and cleans up on unmount', () => {
    const tasks: never[] = [];
    const { unmount } = renderHook(() => useNow(tasks));
    expect(vi.getTimerCount()).toBe(1);
    act(() => vi.advanceTimersByTime(60_000 * 3));
    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('refreshes when the window regains focus', () => {
    const tasks: never[] = [];
    const { result } = renderHook(() => useNow(tasks));
    const start = result.current;
    vi.setSystemTime(start + 5 * 60_000); // computer slept: timers did not run
    act(() => {
      window.dispatchEvent(new Event('focus'));
    });
    expect(result.current).toBe(start + 5 * 60_000);
  });
});
