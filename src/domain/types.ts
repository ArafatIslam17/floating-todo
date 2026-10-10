// Wire types — must match the Rust serde structs (camelCase).

export type TaskStatus = 'pending' | 'completed';
export type ReminderType = 'lead' | 'due';
export type SoundName = 'chime' | 'bell' | 'soft';

export interface Task {
  id: string;
  title: string;
  notes: string | null;
  dueAtUtc: number | null;
  timezoneId: string | null;
  leadMinutes: number | null;
  status: TaskStatus;
  createdAtUtc: number;
  updatedAtUtc: number;
  completedAtUtc: number | null;
  snoozedUntilUtc: number | null;
}

export interface TaskInput {
  title: string;
  notes?: string | null;
  dueAtUtc?: number | null;
  timezoneId?: string | null;
  leadMinutes?: number | null;
}

export interface ActiveAlert {
  reminderId: number;
  taskId: string;
  title: string;
  reminderType: ReminderType;
  dueAtUtc: number | null;
  firedAtUtc: number;
}

export interface Settings {
  soundEnabled: boolean;
  soundName: SoundName;
  soundVolume: number;
  repeatCount: number;
  repeatIntervalMin: number;
  defaultSnoozeMin: number;
  defaultLeadMin: number | null;
  alwaysOnTop: boolean;
  launchAtLogin: boolean;
  opacity: number;
  acrylic: boolean;
  compact: boolean;
  showCompleted: boolean;
  bringForwardOnAlert: boolean;
  globalShortcut: string;
}

export type SettingsPatch = Partial<Settings>;

export interface Issue {
  code: string;
  message: string;
}

export interface Snapshot {
  productName: string;
  version: string;
  tasks: Task[];
  alerts: ActiveAlert[];
  settings: Settings;
  issues: Issue[];
  dbPath: string;
  startupError: string | null;
  nowUtc: number;
}

export interface AlertFired {
  alerts: ActiveAlert[];
  playedSound: boolean;
  count: number;
}

export interface BackendError {
  code: string;
  message: string;
}

export const DEFAULT_SETTINGS: Settings = {
  soundEnabled: true,
  soundName: 'chime',
  soundVolume: 0.7,
  repeatCount: 2,
  repeatIntervalMin: 5,
  defaultSnoozeMin: 10,
  defaultLeadMin: null,
  alwaysOnTop: true,
  launchAtLogin: false,
  opacity: 0.94,
  acrylic: false,
  compact: false,
  showCompleted: false,
  bringForwardOnAlert: true,
  globalShortcut: 'Ctrl+Alt+Space',
};

export const LEAD_OPTIONS: { value: number; label: string }[] = [
  { value: 5, label: '5 min before' },
  { value: 10, label: '10 min before' },
  { value: 15, label: '15 min before' },
  { value: 30, label: '30 min before' },
  { value: 60, label: '1 hour before' },
  { value: 120, label: '2 hours before' },
  { value: 1440, label: '1 day before' },
];

export const SNOOZE_OPTIONS = [5, 10, 15, 30] as const;
