import { AlarmClock, BellRing, ChevronDown, TriangleAlert } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { formatClock, formatCountdown } from '../domain/time';
import { SNOOZE_OPTIONS, type ActiveAlert } from '../domain/types';
import { alertArrive } from '../lib/motion';

interface Props {
  alerts: ActiveAlert[];
  now: number;
  defaultSnooze: number;
  alertSeq: number;
  onComplete(a: ActiveAlert): Promise<unknown>;
  onSnooze(a: ActiveAlert, minutes: number): Promise<unknown>;
  onDismiss(a: ActiveAlert): Promise<unknown>;
  onError(message: string): void;
}

function describe(a: ActiveAlert, now: number): string {
  if (a.dueAtUtc == null) return '';
  if (a.reminderType === 'lead') return `Due ${formatCountdown(a.dueAtUtc, now)}, at ${formatClock(a.dueAtUtc)}`;
  const c = formatCountdown(a.dueAtUtc, now);
  return c === 'Due now' ? `Due now, ${formatClock(a.dueAtUtc)}` : `${c}, was due ${formatClock(a.dueAtUtc)}`;
}

function SnoozeButton({ alert, defaultSnooze, onSnooze }: { alert: ActiveAlert; defaultSnooze: number; onSnooze(m: number): void }) {
  const [open, setOpen] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!wrap.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [open]);
  return (
    <div ref={wrap} className="split relative flex">
      <button
        type="button"
        className="text-btn"
        title="Snooze (Alt+S)"
        onClick={() => onSnooze(defaultSnooze)}
      >
        Snooze {defaultSnooze} min
      </button>
      <button
        type="button"
        className="text-btn" data-caret
        aria-label={`Choose snooze length for ${alert.title}`}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <ChevronDown size={13} strokeWidth={2.4} />
      </button>
      {open && (
        <div
          className="menu right-0 top-7"
          role="menu"
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              e.stopPropagation();
              setOpen(false);
            }
          }}
        >
          {SNOOZE_OPTIONS.map((m) => (
            <button
              key={m}
              type="button"
              role="menuitem"
              autoFocus={m === SNOOZE_OPTIONS[0]}
              onClick={() => {
                setOpen(false);
                onSnooze(m);
              }}
            >
              {m} minutes
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function AlertBanner({ alerts, now, defaultSnooze, alertSeq, onComplete, onSnooze, onDismiss, onError }: Props) {
  const ref = useRef<HTMLElement>(null);
  const [expanded, setExpanded] = useState(false);
  const top = alerts[0];

  useEffect(() => {
    if (alertSeq > 0 && ref.current) alertArrive(ref.current);
  }, [alertSeq]);

  if (!top) return null;
  const run = (p: Promise<unknown>) => p.catch((e) => onError((e as { message?: string }).message ?? String(e)));
  const Icon = top.reminderType === 'lead' ? AlarmClock : top.dueAtUtc != null && top.dueAtUtc < now - 60_000 ? TriangleAlert : BellRing;
  const rest = alerts.slice(1);

  return (
    <section ref={ref} className="alert" data-kind={top.reminderType} role="alert" aria-label="Reminder">
      <div className="flex items-start gap-2">
        <Icon size={16} strokeWidth={2.2} aria-hidden className={top.reminderType === 'lead' ? 'mt-0.5 shrink-0 text-soon' : 'mt-0.5 shrink-0 text-overdue'} />
        <div className="min-w-0 flex-1">
          <p className="m-0 text-body font-bold [overflow-wrap:anywhere]">{top.title}</p>
          <p className="tnum m-0 mt-0.5 text-meta font-semibold text-ink-muted">{describe(top, now)}</p>
        </div>
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-1.5">
        <button type="button" className="text-btn" data-primary onClick={() => void run(onComplete(top))}>
          Complete
        </button>
        <SnoozeButton alert={top} defaultSnooze={defaultSnooze} onSnooze={(m) => void run(onSnooze(top, m))} />
        <button type="button" className="text-btn" title="Dismiss (Alt+D)" onClick={() => void run(onDismiss(top))}>
          Dismiss
        </button>
      </div>
      {rest.length > 0 && (
        <div className="mt-2 border-t border-[var(--hairline)] pt-1.5">
          <button
            type="button"
            className="border-0 bg-transparent p-0 text-meta font-bold text-accent"
            aria-expanded={expanded}
            onClick={() => setExpanded((x) => !x)}
          >
            {rest.length === 1 ? '1 more needs attention' : `${rest.length} more need attention`}
          </button>
          {expanded && (
            <ul className="m-0 mt-1 grid list-none gap-1 p-0">
              {rest.map((a) => (
                <li key={a.reminderId} className="flex items-center gap-2 text-meta">
                  <span className="min-w-0 flex-1 truncate font-semibold" title={a.title}>
                    {a.title}
                  </span>
                  <button type="button" className="text-btn" data-size="sm" onClick={() => void run(onComplete(a))}>
                    Complete
                  </button>
                  <button type="button" className="text-btn" data-size="sm" onClick={() => void run(onDismiss(a))}>
                    Dismiss
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
