// Typed access to the native backend. When the page is not running inside Tauri (plain
// `npm run dev` in a browser, or Playwright), a clearly labelled in-memory preview backend is
// used instead so the UI can be developed and tested; it is never used by the desktop app.
import type {
  AlertFired,
  BackendError,
  Issue,
  Settings,
  SettingsPatch,
  Snapshot,
  SoundName,
  Task,
  TaskInput,
} from '../domain/types';

export interface BackendEvents {
  onSnapshot(s: Snapshot): void;
  onAlert(a: AlertFired): void;
  onIssue(i: Issue): void;
  onFocusQuickAdd(): void;
}

export interface Backend {
  readonly kind: 'tauri' | 'preview';
  getSnapshot(): Promise<Snapshot>;
  createTask(input: TaskInput): Promise<Task>;
  updateTask(id: string, input: TaskInput): Promise<Task>;
  setTaskCompleted(id: string, completed: boolean): Promise<Task>;
  deleteTask(id: string): Promise<void>;
  restoreTask(id: string): Promise<Task>;
  snoozeAlert(reminderId: number, minutes: number): Promise<number>;
  dismissAlert(reminderId: number): Promise<void>;
  updateSettings(patch: SettingsPatch): Promise<Settings>;
  testSound(sound?: SoundName, volume?: number): Promise<void>;
  setCompact(compact: boolean, height?: number): Promise<void>;
  hideWidget(): Promise<void>;
  quitApp(): Promise<void>;
  retryStartup(): Promise<Snapshot>;
  startResizeDragging(): Promise<void>;
  subscribe(events: BackendEvents): Promise<() => void>;
}

export function toBackendError(e: unknown): BackendError {
  if (e && typeof e === 'object' && 'message' in e) {
    const o = e as { code?: unknown; message: unknown };
    return { code: typeof o.code === 'string' ? o.code : 'internal', message: String(o.message) };
  }
  return { code: 'internal', message: String(e) };
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function createTauriBackend(): Promise<Backend> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const call = async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
    try {
      return await invoke<T>(cmd, args);
    } catch (e) {
      throw toBackendError(e);
    }
  };
  return {
    kind: 'tauri',
    getSnapshot: () => call('get_snapshot'),
    createTask: (input) => call('create_task', { input }),
    updateTask: (id, input) => call('update_task', { id, input }),
    setTaskCompleted: (id, completed) => call('set_task_completed', { id, completed }),
    deleteTask: (id) => call('delete_task', { id }),
    restoreTask: (id) => call('restore_task', { id }),
    snoozeAlert: (reminderId, minutes) => call('snooze_alert', { reminderId, minutes }),
    dismissAlert: (reminderId) => call('dismiss_alert', { reminderId }),
    updateSettings: (patch) => call('update_settings', { patch }),
    testSound: (sound, volume) => call('test_sound', { sound: sound ?? null, volume: volume ?? null }),
    setCompact: (compact, height) => call('set_compact', { compact, height: height ?? null }),
    hideWidget: () => call('hide_widget'),
    quitApp: () => call('quit_app'),
    retryStartup: () => call('retry_startup'),
    startResizeDragging: () => getCurrentWindow().startResizeDragging('SouthEast'),
    subscribe: async (ev) => {
      const un = await Promise.all([
        listen<Snapshot>('snapshot-changed', (e) => ev.onSnapshot(e.payload)),
        listen<AlertFired>('alert-fired', (e) => ev.onAlert(e.payload)),
        listen<Issue>('app-issue', (e) => ev.onIssue(e.payload)),
        listen('focus-quick-add', () => ev.onFocusQuickAdd()),
      ]);
      return () => un.forEach((u) => u());
    },
  };
}

let backendPromise: Promise<Backend> | null = null;

export function getBackend(): Promise<Backend> {
  backendPromise ??= isTauri()
    ? createTauriBackend()
    : import('./previewBackend').then((m) => m.createPreviewBackend());
  return backendPromise;
}
