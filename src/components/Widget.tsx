import { ChevronRight } from 'lucide-react';
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { countOverdue, sortAlerts, sortCompleted, sortPending } from '../domain/sort';
import type { ActiveAlert, Task, TaskInput } from '../domain/types';
import { useApp } from '../state/AppContext';
import { useNow } from '../state/useNow';
import { AlertBanner } from './AlertBanner';
import { DatabaseErrorScreen } from './DatabaseErrorScreen';
import { EmptyState } from './EmptyState';
import { Header } from './Header';
import { IssueBanner } from './IssueBanner';
import { QuickAdd, type QuickAddHandle } from './QuickAdd';
import { SettingsSheet } from './SettingsSheet';
import { TaskRow } from './TaskRow';
import { UndoToast } from './UndoToast';

// After deleting with the keyboard, focus may land in the (empty) quick-add box; Ctrl+Z there
// still means "undo delete" because there is no text to undo.
const isEmptyQuickAdd = (t: EventTarget | null) => t instanceof HTMLInputElement && t.id === 'quick-add' && t.value === '';

const errMsg = (e: unknown) => (e as { message?: string }).message ?? String(e);

export function Widget() {
  const { state, backend, dispatch, reportIssue } = useApp();
  const snap = state.snapshot;
  const tasks = useMemo(() => snap?.tasks ?? [], [snap]);
  // The backend stamps each snapshot, so an alert that just fired is never shown as "in 1 min".
  const now = Math.max(useNow(tasks), snap?.nowUtc ?? 0);
  const settings = snap?.settings;

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [undo, setUndo] = useState<{ id: string; title: string } | null>(null);
  const [compact, setCompact] = useState(false);
  const [showCompleted, setShowCompleted] = useState(false);
  const quickAdd = useRef<QuickAddHandle>(null);
  const measureRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const lastCompactHeight = useRef(0);

  // Local, optimistic view state that follows persisted settings when they change
  // (React's "adjust state when a prop changes" pattern — no effect round-trip).
  const [seenCompact, setSeenCompact] = useState<boolean | undefined>(undefined);
  if (settings && settings.compact !== seenCompact) {
    setSeenCompact(settings.compact);
    setCompact(settings.compact);
  }
  const [seenShowCompleted, setSeenShowCompleted] = useState<boolean | undefined>(undefined);
  if (settings && settings.showCompleted !== seenShowCompleted) {
    setSeenShowCompleted(settings.showCompleted);
    setShowCompleted(settings.showCompleted);
  }
  // Global shortcut / tray "New task": leave settings and compact mode, then focus quick-add.
  const [seenFocusSeq, setSeenFocusSeq] = useState(0);
  if (state.focusQuickAddSeq !== seenFocusSeq) {
    setSeenFocusSeq(state.focusQuickAddSeq);
    setSettingsOpen(false);
    setCompact(false);
  }
  const handledFocusSeq = useRef(0);
  useEffect(() => {
    if (state.focusQuickAddSeq === handledFocusSeq.current) return;
    handledFocusSeq.current = state.focusQuickAddSeq;
    if (settings?.compact) void backend?.setCompact(false);
    requestAnimationFrame(() => quickAdd.current?.focus());
  }, [state.focusQuickAddSeq, settings, backend]);

  useEffect(() => {
    if (settings) document.documentElement.style.setProperty('--surface-alpha', String(settings.opacity));
  }, [settings]);

  const pending = useMemo(() => sortPending(tasks), [tasks]);
  const completed = useMemo(() => sortCompleted(tasks), [tasks]);
  const alerts = useMemo(() => sortAlerts(snap?.alerts ?? []), [snap]);
  const overdue = countOverdue(tasks, now);

  // Compact mode: size the window to the header, alert and the most urgent task.
  useLayoutEffect(() => {
    if (!compact || !backend || !measureRef.current) return;
    const h = Math.ceil(measureRef.current.getBoundingClientRect().height) + 2;
    if (Math.abs(h - lastCompactHeight.current) > 2) {
      lastCompactHeight.current = h;
      backend.setCompact(true, h).catch((e) => reportIssue('window', `Couldn't resize the widget: ${errMsg(e)}`));
    }
  });

  const toggleCompact = () => {
    if (!backend) return;
    const next = !compact;
    setCompact(next);
    lastCompactHeight.current = 0;
    if (!next) {
      backend.setCompact(false).catch((e) => reportIssue('window', `Couldn't resize the widget: ${errMsg(e)}`));
    }
  };

  const focusRow = useCallback((id: string) => {
    listRef.current?.querySelector<HTMLElement>(`[data-task-id="${CSS.escape(id)}"]`)?.focus();
  }, []);

  const moveFocus = useCallback(
    (id: string, dir: 1 | -1) => {
      const order = [...pending, ...(showCompleted ? completed : [])].map((t) => t.id);
      const i = order.indexOf(id);
      const next = order[i + dir] ?? order[i - dir];
      if (next && next !== id) {
        setFocusedId(next);
        requestAnimationFrame(() => focusRow(next));
      } else if (dir === 1) {
        quickAdd.current?.focus();
      }
    },
    [pending, completed, showCompleted, focusRow],
  );

  const doRestore = useCallback(async () => {
    if (!backend || !undo) return;
    const { id } = undo;
    setUndo(null);
    try {
      await backend.restoreTask(id);
    } catch (e) {
      reportIssue('undo', `Couldn't restore the task: ${errMsg(e)}`);
    }
  }, [backend, undo, reportIssue]);
  const expireUndo = useCallback(() => setUndo(null), []);

  // Widget-wide shortcuts (PRD FR-10).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && e.target.matches('input, textarea, select');
      const top = alerts[0];
      if ((e.ctrlKey && e.key.toLowerCase() === 'n') || (!typing && e.key === '/')) {
        e.preventDefault();
        setSettingsOpen(false);
        quickAdd.current?.focus();
      } else if (e.altKey && e.code === 'KeyS' && top && backend && settings) {
        e.preventDefault();
        backend.snoozeAlert(top.reminderId, settings.defaultSnoozeMin).catch((x) => reportIssue('alert', errMsg(x)));
      } else if (e.altKey && e.code === 'KeyD' && top && backend) {
        e.preventDefault();
        backend.dismissAlert(top.reminderId).catch((x) => reportIssue('alert', errMsg(x)));
      } else if (e.ctrlKey && e.key.toLowerCase() === 'z' && undo && (!typing || isEmptyQuickAdd(e.target))) {
        e.preventDefault();
        void doRestore();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [alerts, backend, settings, undo, doRestore, reportIssue]);

  if (!snap || !settings || !backend) {
    return (
      <div className="panel" aria-busy="true">
        <div className="h-10" />
        {state.status === 'error' ? (
          <DatabaseErrorScreen message={state.loadError ?? 'Unknown error'} path="" onRetry={async () => location.reload()} onQuit={() => {}} />
        ) : (
          <div className="grid gap-2 px-3 py-2" aria-label="Loading tasks">
            {[0, 1, 2].map((i) => (
              <div key={i} className="h-11 rounded-[10px] border border-[var(--hairline)]" />
            ))}
          </div>
        )}
      </div>
    );
  }

  const isPreview = backend.kind === 'preview';
  const rowActions = {
    onToggle: (t: Task, done: boolean) => backend.setTaskCompleted(t.id, done),
    onSave: (t: Task, input: TaskInput) => backend.updateTask(t.id, input),
    onDelete: async (t: Task) => {
      await backend.deleteTask(t.id);
      setUndo({ id: t.id, title: t.title });
    },
  };
  const tabbableId = focusedId && tasks.some((t) => t.id === focusedId) ? focusedId : pending[0]?.id ?? completed[0]?.id;
  const renderRow = (t: Task) => (
    <TaskRow
      key={t.id}
      task={t}
      now={now}
      defaultLeadMinutes={settings.defaultLeadMin}
      tabbable={t.id === tabbableId}
      onFocusRow={setFocusedId}
      onMove={moveFocus}
      editing={editingId === t.id}
      setEditing={setEditingId}
      onError={(m) => reportIssue('task', m)}
      {...rowActions}
    />
  );

  const alertBanner = (
    <AlertBanner
      alerts={alerts}
      now={now}
      defaultSnooze={settings.defaultSnoozeMin}
      alertSeq={state.alertSeq}
      onComplete={(a: ActiveAlert) => backend.setTaskCompleted(a.taskId, true)}
      onSnooze={(a, m) => backend.snoozeAlert(a.reminderId, m)}
      onDismiss={(a) => backend.dismissAlert(a.reminderId)}
      onError={(m) => reportIssue('alert', m)}
    />
  );

  const header = (
    <Header
      productName={snap.productName}
      openCount={pending.length}
      overdueCount={overdue}
      compact={compact}
      onToggleCompact={toggleCompact}
      onOpenSettings={() => setSettingsOpen(true)}
      onHide={() => void backend.hideWidget()}
    />
  );

  if (snap.startupError) {
    return (
      <div className="panel">
        {header}
        <DatabaseErrorScreen
          message={snap.startupError}
          path={snap.dbPath}
          onRetry={() => backend.retryStartup()}
          onQuit={() => void backend.quitApp()}
        />
      </div>
    );
  }

  if (compact) {
    const next = pending[0];
    return (
      <div className="panel">
        <div ref={measureRef}>
          {header}
          {alertBanner}
          {next ? (
            <ul className="task-list" ref={listRef} aria-label="Next task">
              {renderRow(next)}
            </ul>
          ) : (
            <p className="m-0 px-4 pb-3 text-meta font-semibold text-ink-muted">Nothing open.</p>
          )}
        </div>
      </div>
    );
  }

  const issues = isPreview
    ? [{ code: 'preview', message: 'Browser preview: nothing is saved and there are no sounds. Run the desktop app for real reminders.' }, ...state.issues]
    : state.issues;

  return (
    <div className="panel">
      {header}
      <IssueBanner issues={issues} onDismiss={(code) => dispatch({ type: 'dismissIssue', code })} />
      {alertBanner}
      <div className="scroll min-h-0 flex-1">
        {pending.length === 0 && <EmptyState kind={completed.length ? 'all-done' : 'empty'} />}
        {pending.length > 0 && (
          <ul className="task-list" ref={listRef} aria-label="Open tasks">
            {pending.map(renderRow)}
          </ul>
        )}
        {completed.length > 0 && (
          <div className="px-1.5 pb-2">
            <button
              type="button"
              className="flex w-full items-center gap-1 rounded-lg border-0 bg-transparent px-2 py-1.5 text-meta font-bold text-ink-muted hover:text-ink"
              aria-expanded={showCompleted}
              onClick={() => {
                const v = !showCompleted;
                setShowCompleted(v);
                backend.updateSettings({ showCompleted: v }).catch(() => {});
              }}
            >
              <ChevronRight
                size={13}
                strokeWidth={2.4}
                aria-hidden
                style={{ transform: showCompleted ? 'rotate(90deg)' : undefined, transition: 'transform var(--dur-fast) var(--ease)' }}
              />
              Completed ({completed.length})
            </button>
            {showCompleted && (
              <ul className="task-list" data-flush aria-label="Completed tasks">
                {completed.map(renderRow)}
              </ul>
            )}
          </div>
        )}
      </div>
      <QuickAdd ref={quickAdd} onCreate={(input) => backend.createTask(input)} defaultLeadMinutes={settings.defaultLeadMin} now={now} />
      {!isPreview && (
        <div
          className="grip"
          aria-hidden="true"
          onMouseDown={(e) => {
            e.preventDefault();
            void backend.startResizeDragging();
          }}
        />
      )}
      {undo && <UndoToast title={undo.title} onUndo={() => void doRestore()} onExpire={expireUndo} />}
      {settingsOpen && (
        <SettingsSheet
          settings={settings}
          version={snap.version}
          dbPath={snap.dbPath}
          isPreview={isPreview}
          onPatch={(p) => backend.updateSettings(p)}
          onTestSound={(sound, volume) => backend.testSound(sound, volume)}
          onQuit={() => void backend.quitApp()}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </div>
  );
}
