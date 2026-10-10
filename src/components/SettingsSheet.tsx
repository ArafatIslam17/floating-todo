import { Power, Volume2, X } from 'lucide-react';
import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { LEAD_OPTIONS, SNOOZE_OPTIONS, type Settings, type SettingsPatch, type SoundName } from '../domain/types';

interface Props {
  settings: Settings;
  version: string;
  dbPath: string;
  isPreview: boolean;
  onPatch(patch: SettingsPatch): Promise<unknown>;
  onTestSound(sound: SoundName, volume: number): Promise<unknown>;
  onQuit(): void;
  onClose(): void;
}

const errMsg = (e: unknown) => (e as { message?: string }).message ?? String(e);

/** Turn a keydown into an accelerator string accepted by the global-shortcut plugin. */
export function acceleratorFromEvent(e: Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey' | 'code'>): string | null {
  const code = e.code;
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
  else if (/^F([1-9]|1[0-2])$/.test(code)) key = code;
  else if (['Space', 'Enter', 'Backspace', 'Home', 'End', 'PageUp', 'PageDown', 'Insert'].includes(code)) key = code;
  else if (code.startsWith('Arrow')) key = code.replace('Arrow', '');
  if (!key) return null;
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
  // Require Ctrl, Alt or Super so typing a plain letter can't become a global shortcut.
  if (!e.ctrlKey && !e.altKey && !e.metaKey) return null;
  return [...mods, key].join('+');
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-0.5 border-b border-[var(--hairline)] px-3.5 py-2.5 last:border-b-0">
      <h3 className="m-0 mb-0.5 text-meta font-bold text-ink-muted">{title}</h3>
      {children}
    </section>
  );
}

export function SettingsSheet({ settings: s, version, dbPath, isPreview, onPatch, onTestSound, onQuit, onClose }: Props) {
  const id = useId();
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [volume, setVolume] = useState(s.soundVolume);
  const [opacity, setOpacity] = useState(s.opacity);
  const [shortcutDraft, setShortcutDraft] = useState(s.globalShortcut);
  const [capturing, setCapturing] = useState(false);
  const [testing, setTesting] = useState(false);
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => closeRef.current?.focus(), []);
  useEffect(() => setVolume(s.soundVolume), [s.soundVolume]);
  useEffect(() => setOpacity(s.opacity), [s.opacity]);

  const patch = async (key: string, p: SettingsPatch) => {
    try {
      await onPatch(p);
      setErrors(({ [key]: _gone, ...rest }) => rest);
    } catch (e) {
      setErrors((x) => ({ ...x, [key]: errMsg(e) }));
    }
  };
  const err = (key: string) =>
    errors[key] ? (
      <p className="field-error" role="alert">
        {errors[key]}
      </p>
    ) : null;

  return (
    <div
      className="sheet"
      role="dialog"
      aria-modal="true"
      aria-labelledby={`${id}-title`}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && !capturing) {
          e.stopPropagation();
          onClose();
        }
      }}
    >
      <div className="flex h-10 shrink-0 items-center pr-1.5 pl-3.5" data-tauri-drag-region>
        <h2 id={`${id}-title`} className="m-0 flex-1 text-head font-bold" data-tauri-drag-region>
          Settings
        </h2>
        <button ref={closeRef} type="button" className="icon-btn" aria-label="Close settings" title="Close (Esc)" onClick={onClose}>
          <X size={16} strokeWidth={2} />
        </button>
      </div>
      <div className="scroll flex-1">
        <Section title="Alerts">
          <label className="setting">
            Play a sound when tasks are due
            <input
              type="checkbox"
              role="switch"
              className="switch"
              checked={s.soundEnabled}
              onChange={(e) => void patch('sound', { soundEnabled: e.target.checked })}
            />
          </label>
          <div className="setting">
            <label htmlFor={`${id}-tone`}>Tone</label>
            <div className="flex items-center gap-1.5">
              <select
                id={`${id}-tone`}
                className="field"
                value={s.soundName}
                disabled={!s.soundEnabled}
                onChange={(e) => void patch('sound', { soundName: e.target.value as SoundName })}
              >
                <option value="chime">Chime</option>
                <option value="bell">Bell</option>
                <option value="soft">Soft</option>
              </select>
              <button
                type="button"
                className="text-btn inline-flex items-center gap-1"
                disabled={testing}
                onClick={async () => {
                  setTesting(true);
                  try {
                    await onTestSound(s.soundName, volume);
                    setErrors(({ test: _t, ...rest }) => rest);
                  } catch (e) {
                    setErrors((x) => ({ ...x, test: errMsg(e) }));
                  } finally {
                    setTesting(false);
                  }
                }}
              >
                <Volume2 size={13} strokeWidth={2.2} aria-hidden /> Test
              </button>
            </div>
          </div>
          {err('test')}
          <div className="setting">
            <label htmlFor={`${id}-vol`}>Volume</label>
            <input
              id={`${id}-vol`}
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={volume}
              disabled={!s.soundEnabled}
              aria-valuetext={`${Math.round(volume * 100)} percent`}
              onChange={(e) => setVolume(Number(e.target.value))}
              onPointerUp={() => void patch('sound', { soundVolume: volume })}
              onKeyUp={() => void patch('sound', { soundVolume: volume })}
            />
          </div>
          {err('sound')}
          <div className="setting">
            <label htmlFor={`${id}-repeat`}>If not answered, ring again</label>
            <select
              id={`${id}-repeat`}
              className="field"
              value={s.repeatCount}
              onChange={(e) => void patch('repeat', { repeatCount: Number(e.target.value) })}
            >
              <option value={0}>Never</option>
              <option value={1}>Once more</option>
              <option value={2}>2 more times</option>
              <option value={3}>3 more times</option>
            </select>
          </div>
          {s.repeatCount > 0 && (
            <div className="setting">
              <label htmlFor={`${id}-interval`}>Time between rings</label>
              <select
                id={`${id}-interval`}
                className="field"
                value={s.repeatIntervalMin}
                onChange={(e) => void patch('repeat', { repeatIntervalMin: Number(e.target.value) })}
              >
                {[2, 5, 10, 15, 30].map((m) => (
                  <option key={m} value={m}>
                    {m} min
                  </option>
                ))}
              </select>
            </div>
          )}
          {err('repeat')}
          <div className="setting">
            <label htmlFor={`${id}-snooze`}>Default snooze</label>
            <select
              id={`${id}-snooze`}
              className="field"
              value={s.defaultSnoozeMin}
              onChange={(e) => void patch('snooze', { defaultSnoozeMin: Number(e.target.value) })}
            >
              {SNOOZE_OPTIONS.map((m) => (
                <option key={m} value={m}>
                  {m} min
                </option>
              ))}
            </select>
          </div>
          <div className="setting">
            <label htmlFor={`${id}-lead`}>Default early reminder</label>
            <select
              id={`${id}-lead`}
              className="field"
              value={s.defaultLeadMin ?? ''}
              onChange={(e) => void patch('lead', { defaultLeadMin: e.target.value ? Number(e.target.value) : null })}
            >
              <option value="">None</option>
              {LEAD_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
          </div>
          {err('snooze')}
          {err('lead')}
        </Section>

        <Section title="Window">
          <label className="setting">
            Keep above other windows
            <input
              type="checkbox"
              role="switch"
              className="switch"
              checked={s.alwaysOnTop}
              onChange={(e) => void patch('window', { alwaysOnTop: e.target.checked })}
            />
          </label>
          <label className="setting">
            Bring the widget forward when a task is due
            <input
              type="checkbox"
              role="switch"
              className="switch"
              checked={s.bringForwardOnAlert}
              onChange={(e) => void patch('window', { bringForwardOnAlert: e.target.checked })}
            />
          </label>
          <div className="setting">
            <label htmlFor={`${id}-opacity`}>Background opacity</label>
            <input
              id={`${id}-opacity`}
              type="range"
              min={0.85}
              max={1}
              step={0.01}
              value={opacity}
              aria-valuetext={`${Math.round(opacity * 100)} percent`}
              onChange={(e) => {
                const v = Number(e.target.value);
                setOpacity(v);
                document.documentElement.style.setProperty('--surface-alpha', String(v));
              }}
              onPointerUp={() => void patch('window', { opacity })}
              onKeyUp={() => void patch('window', { opacity })}
            />
          </div>
          <label className="setting">
            <span>
              Blur the desktop behind (Windows)
              <span className="block text-small font-medium text-ink-muted">May lag while dragging on Windows 10.</span>
            </span>
            <input
              type="checkbox"
              role="switch"
              className="switch"
              checked={s.acrylic}
              onChange={(e) => void patch('window', { acrylic: e.target.checked })}
            />
          </label>
          <label className="setting">
            Open when I sign in
            <input
              type="checkbox"
              role="switch"
              className="switch"
              checked={s.launchAtLogin}
              onChange={(e) => void patch('window', { launchAtLogin: e.target.checked })}
            />
          </label>
          {err('window')}
        </Section>

        <Section title="Shortcut to show the widget">
          <div className="flex items-center gap-1.5">
            <label htmlFor={`${id}-shortcut`} className="sr-only">
              Global shortcut
            </label>
            <input
              id={`${id}-shortcut`}
              className="field tnum"
              readOnly
              value={capturing ? 'Press keys…' : shortcutDraft}
              aria-describedby={`${id}-shortcut-help`}
              onFocus={() => setCapturing(true)}
              onBlur={() => setCapturing(false)}
              onKeyDown={(e) => {
                if (e.key === 'Tab') return;
                e.preventDefault();
                if (e.key === 'Escape') {
                  setShortcutDraft(s.globalShortcut);
                  (e.target as HTMLInputElement).blur();
                  return;
                }
                const acc = acceleratorFromEvent(e);
                if (acc) {
                  setShortcutDraft(acc);
                  setCapturing(false);
                }
              }}
            />
            <button
              type="button"
              className="text-btn"
              data-primary
              disabled={shortcutDraft === s.globalShortcut}
              onClick={() => void patch('shortcut', { globalShortcut: shortcutDraft })}
            >
              Save
            </button>
          </div>
          <p id={`${id}-shortcut-help`} className="m-0 mt-1 text-small font-medium text-ink-muted">
            Click the box and press a combination with Ctrl or Alt.
          </p>
          {err('shortcut')}
        </Section>

        <Section title="About">
          <p className="m-0 text-meta text-ink-muted">
            Version {version}. Tasks are stored only on this computer:
            <br />
            <code className="text-ink [overflow-wrap:anywhere] select-text">{dbPath}</code>
          </p>
          <p className="m-0 mt-1.5 text-meta text-ink-muted">
            Reminders only ring while the app is running (closing the widget keeps it in the tray).
          </p>
          {!isPreview && (
            <button type="button" className="text-btn mt-2 inline-flex w-fit items-center gap-1.5" onClick={onQuit}>
              <Power size={13} strokeWidth={2.2} aria-hidden /> Quit app
            </button>
          )}
        </Section>
      </div>
    </div>
  );
}
