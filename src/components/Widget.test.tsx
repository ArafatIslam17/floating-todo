import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { App } from '../App';
import { DEFAULT_SETTINGS, type ActiveAlert, type Snapshot, type Task } from '../domain/types';
import type { Backend, BackendEvents } from '../lib/backend';

const NOW = Date.UTC(2027, 2, 3, 10, 0);

function task(o: Partial<Task>): Task {
  return {
    id: crypto.randomUUID(),
    title: 'Task',
    notes: null,
    dueAtUtc: null,
    timezoneId: null,
    leadMinutes: null,
    status: 'pending',
    createdAtUtc: NOW - 60_000,
    updatedAtUtc: NOW - 60_000,
    completedAtUtc: null,
    snoozedUntilUtc: null,
    ...o,
  };
}

function snapshot(o: Partial<Snapshot> = {}): Snapshot {
  return {
    productName: 'Floating Todo',
    version: '0.1.0',
    tasks: [],
    alerts: [],
    settings: { ...DEFAULT_SETTINGS },
    issues: [],
    dbPath: 'C:\\Users\\me\\AppData\\Roaming\\app.floatingtodo.desktop\\floating-todo.db',
    startupError: null,
    nowUtc: NOW,
    ...o,
  };
}

/** A scripted backend: every method is a spy; `emit` pushes events like the Rust side does. */
function fakeBackend(initial: Snapshot) {
  let listeners: BackendEvents | null = null;
  const b = {
    kind: 'tauri' as const,
    getSnapshot: vi.fn(async () => initial),
    createTask: vi.fn(async (input) => task({ title: input.title })),
    updateTask: vi.fn(async (_id, input) => task({ title: input.title })),
    setTaskCompleted: vi.fn(async (id) => task({ id, status: 'completed' })),
    deleteTask: vi.fn(async () => {}),
    restoreTask: vi.fn(async (id) => task({ id })),
    snoozeAlert: vi.fn(async () => NOW + 600_000),
    dismissAlert: vi.fn(async () => {}),
    updateSettings: vi.fn(async (p) => ({ ...DEFAULT_SETTINGS, ...p })),
    testSound: vi.fn(async () => {}),
    setCompact: vi.fn(async () => {}),
    hideWidget: vi.fn(async () => {}),
    quitApp: vi.fn(async () => {}),
    retryStartup: vi.fn(async () => initial),
    startResizeDragging: vi.fn(async () => {}),
    subscribe: vi.fn(async (ev: BackendEvents) => {
      listeners = ev;
      return () => {
        listeners = null;
      };
    }),
  } satisfies Backend;
  return {
    backend: b,
    emit: (s: Snapshot) => act(() => listeners?.onSnapshot(s)),
    issue: (code: string, message: string) => act(() => listeners?.onIssue({ code, message })),
    isSubscribed: () => listeners != null,
  };
}

async function renderApp(initial: Snapshot) {
  const fb = fakeBackend(initial);
  const utils = render(<App backend={Promise.resolve(fb.backend)} />);
  await screen.findByRole('banner');
  return { ...fb, ...utils, user: userEvent.setup() };
}

describe('Widget', () => {
  it('shows the empty state and focuses quick-add with Ctrl+N', async () => {
    const { user } = await renderApp(snapshot());
    expect(screen.getByText(/Nothing on your list/)).toBeInTheDocument();
    await user.keyboard('{Control>}n{/Control}');
    expect(screen.getByLabelText('New task title')).toHaveFocus();
  });

  it('rejects an empty title without calling the backend', async () => {
    const { user, backend } = await renderApp(snapshot());
    await user.click(screen.getByLabelText('New task title'));
    await user.keyboard('   {Enter}');
    expect(await screen.findByText('Enter a task title.')).toBeInTheDocument();
    expect(backend.createTask).not.toHaveBeenCalled();
  });

  it('creates a task with Enter and clears the input', async () => {
    const { user, backend } = await renderApp(snapshot());
    const input = screen.getByLabelText('New task title');
    await user.type(input, 'Reply to supervisor{Enter}');
    expect(backend.createTask).toHaveBeenCalledWith({
      title: 'Reply to supervisor',
      notes: null,
      dueAtUtc: null,
      timezoneId: null,
      leadMinutes: null,
    });
    await waitFor(() => expect(input).toHaveValue(''));
    expect(input).toHaveFocus();
  });

  it('keeps the text and shows the reason when saving fails', async () => {
    const { user, backend } = await renderApp(snapshot());
    backend.createTask.mockRejectedValueOnce({ code: 'database', message: 'disk is full' });
    const input = screen.getByLabelText('New task title');
    await user.type(input, 'Important{Enter}');
    expect(await screen.findByText("Couldn't save: disk is full")).toBeInTheDocument();
    expect(input).toHaveValue('Important');
  });

  it('renders time, countdown and overdue state with text, not colour alone', async () => {
    await renderApp(
      snapshot({
        tasks: [
          task({ title: 'Soon', dueAtUtc: NOW + 25 * 60_000 }),
          task({ title: 'Late', dueAtUtc: NOW - 12 * 60_000 }),
          task({ title: 'Someday' }),
        ],
      }),
    );
    const rows = screen.getAllByRole('listitem');
    expect(rows.map((r) => r.getAttribute('aria-label'))).toEqual([
      'Late, Overdue, Overdue 12 min',
      'Soon, Due soon, in 25 min',
      'Someday',
    ]);
    expect(screen.getByText('1 overdue')).toBeInTheDocument();
  });

  it('alert banner actions and Alt+S / Alt+D reach the backend', async () => {
    const alert: ActiveAlert = { reminderId: 7, taskId: 't1', title: 'Submit report', reminderType: 'due', dueAtUtc: NOW, firedAtUtc: NOW };
    const { user, backend } = await renderApp(
      snapshot({ tasks: [task({ id: 't1', title: 'Submit report', dueAtUtc: NOW })], alerts: [alert] }),
    );
    const banner = screen.getByRole('alert', { name: 'Reminder' });
    expect(within(banner).getByText(/^Due now, 10:00/)).toBeInTheDocument();
    await user.click(within(banner).getByRole('button', { name: 'Snooze 10 min' }));
    expect(backend.snoozeAlert).toHaveBeenCalledWith(7, 10);
    await user.click(within(banner).getByRole('button', { name: /Choose snooze length/ }));
    await user.click(screen.getByRole('menuitem', { name: '30 minutes' }));
    expect(backend.snoozeAlert).toHaveBeenLastCalledWith(7, 30);
    await user.click(within(banner).getByRole('button', { name: 'Complete' }));
    expect(backend.setTaskCompleted).toHaveBeenCalledWith('t1', true);
    await user.keyboard('{Alt>}d{/Alt}');
    expect(backend.dismissAlert).toHaveBeenCalledWith(7);
    await user.keyboard('{Alt>}s{/Alt}');
    expect(backend.snoozeAlert).toHaveBeenLastCalledWith(7, 10);
  });

  it('keyboard: arrows move between rows, Space completes, E edits, Ctrl+Enter saves', async () => {
    const { user, backend } = await renderApp(
      snapshot({ tasks: [task({ id: 'a', title: 'First', dueAtUtc: NOW + 3_600_000 }), task({ id: 'b', title: 'Second' })] }),
    );
    const [first, second] = screen.getAllByRole('listitem');
    first!.focus();
    await user.keyboard('{ArrowDown}');
    await waitFor(() => expect(second).toHaveFocus());
    await user.keyboard('e');
    const title = await screen.findByLabelText('Title');
    await user.clear(title);
    await user.type(title, 'Second, renamed');
    await user.keyboard('{Control>}{Enter}{/Control}');
    expect(backend.updateTask).toHaveBeenCalledWith('b', expect.objectContaining({ title: 'Second, renamed' }));
    await waitFor(() => expect(screen.queryByLabelText('Title')).not.toBeInTheDocument());
    screen.getAllByRole('listitem')[0]!.focus();
    await user.keyboard(' ');
    await waitFor(() => expect(backend.setTaskCompleted).toHaveBeenCalledWith('a', true));
  });

  it('delete offers undo which restores the task', async () => {
    const { user, backend } = await renderApp(snapshot({ tasks: [task({ id: 'x', title: 'Throwaway' })] }));
    await user.click(screen.getByRole('button', { name: 'Delete Throwaway' }));
    await waitFor(() => expect(backend.deleteTask).toHaveBeenCalledWith('x'));
    const toast = await screen.findByText(/Deleted “Throwaway”/);
    await user.click(within(toast.parentElement!).getByRole('button', { name: 'Undo' }));
    expect(backend.restoreTask).toHaveBeenCalledWith('x');
  });

  it('shows the database error screen with the file path and retries', async () => {
    const { user, backend } = await renderApp(snapshot({ startupError: 'database error: file is not a database' }));
    expect(screen.getByText("Can't open your task database.")).toBeInTheDocument();
    expect(screen.getByText(/floating-todo\.db/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /Retry/ }));
    expect(backend.retryStartup).toHaveBeenCalled();
  });

  it('surfaces audio and notification failures and lets the user dismiss them', async () => {
    const { user, issue } = await renderApp(snapshot());
    issue('audio', "Couldn't play the alert sound: no audio output device available. Check your output device.");
    const msg = await screen.findByText(/Couldn't play the alert sound/);
    await user.click(within(msg.parentElement!).getByRole('button', { name: 'Dismiss message' }));
    expect(screen.queryByText(/Couldn't play the alert sound/)).not.toBeInTheDocument();
  });

  it('shows a settings error inline when the OS refuses a shortcut', async () => {
    const { user, backend } = await renderApp(snapshot());
    backend.updateSettings.mockRejectedValueOnce({ code: 'shortcut', message: 'Ctrl+Alt+K is in use by another app.' });
    await user.click(screen.getByRole('button', { name: 'Settings' }));
    const box = screen.getByLabelText('Global shortcut');
    await user.click(box);
    await user.keyboard('{Control>}{Alt>}k{/Alt}{/Control}');
    expect(box).toHaveValue('Ctrl+Alt+K');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText('Ctrl+Alt+K is in use by another app.')).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('unsubscribes from backend events on unmount', async () => {
    const fb = await renderApp(snapshot());
    expect(fb.isSubscribed()).toBe(true);
    fb.unmount();
    expect(fb.isSubscribed()).toBe(false);
  });
});
