import { createContext, useContext, useEffect, useMemo, useReducer, useState, type ReactNode } from 'react';
import type { Backend } from '../lib/backend';
import { toBackendError } from '../lib/backend';
import type { AlertFired, Issue, Snapshot } from '../domain/types';

export interface AppState {
  status: 'loading' | 'ready' | 'error';
  snapshot: Snapshot | null;
  loadError: string | null;
  issues: Issue[];
  /** Increments whenever an alert fires (drives the banner arrival animation). */
  alertSeq: number;
  /** Increments when the backend asks to focus quick-add (global shortcut, tray "New task"). */
  focusQuickAddSeq: number;
}

type Action =
  | { type: 'snapshot'; snapshot: Snapshot }
  | { type: 'loadError'; message: string }
  | { type: 'issue'; issue: Issue }
  | { type: 'dismissIssue'; code: string }
  | { type: 'alert'; alert: AlertFired }
  | { type: 'focusQuickAdd' };

export const initialState: AppState = {
  status: 'loading',
  snapshot: null,
  loadError: null,
  issues: [],
  alertSeq: 0,
  focusQuickAddSeq: 0,
};

function mergeIssues(current: Issue[], incoming: Issue[]): Issue[] {
  const out = [...current];
  for (const i of incoming) {
    const idx = out.findIndex((x) => x.code === i.code);
    if (idx >= 0) out[idx] = i;
    else out.push(i);
  }
  return out;
}

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case 'snapshot': {
      const first = state.snapshot == null;
      return {
        ...state,
        status: 'ready',
        snapshot: action.snapshot,
        loadError: null,
        // Backend issues present before the UI loaded are shown once.
        issues: first ? mergeIssues(state.issues, action.snapshot.issues) : state.issues,
      };
    }
    case 'loadError':
      return { ...state, status: state.snapshot ? state.status : 'error', loadError: action.message };
    case 'issue':
      return { ...state, issues: mergeIssues(state.issues, [action.issue]) };
    case 'dismissIssue':
      return { ...state, issues: state.issues.filter((i) => i.code !== action.code) };
    case 'alert':
      return { ...state, alertSeq: state.alertSeq + 1 };
    case 'focusQuickAdd':
      return { ...state, focusQuickAddSeq: state.focusQuickAddSeq + 1 };
  }
}

interface Ctx {
  state: AppState;
  backend: Backend | null;
  dispatch: React.Dispatch<Action>;
  reportIssue(code: string, message: string): void;
}

const AppCtx = createContext<Ctx | null>(null);

export function AppProvider({ backendPromise, children }: { backendPromise: Promise<Backend>; children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [backend, setBackend] = useState<Backend | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unsubscribe: (() => void) | null = null;
    backendPromise
      .then(async (backend) => {
        if (cancelled) return;
        setBackend(backend);
        const un = await backend.subscribe({
          onSnapshot: (snapshot) => dispatch({ type: 'snapshot', snapshot }),
          onAlert: (alert) => dispatch({ type: 'alert', alert }),
          onIssue: (issue) => dispatch({ type: 'issue', issue }),
          onFocusQuickAdd: () => dispatch({ type: 'focusQuickAdd' }),
        });
        if (cancelled) {
          un();
          return;
        }
        unsubscribe = un;
        const snapshot = await backend.getSnapshot();
        if (!cancelled) dispatch({ type: 'snapshot', snapshot });
      })
      .catch((e) => {
        if (!cancelled) dispatch({ type: 'loadError', message: toBackendError(e).message });
      });
    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, [backendPromise]);

  const value = useMemo<Ctx>(
    () => ({
      state,
      backend,
      dispatch,
      reportIssue: (code, message) => dispatch({ type: 'issue', issue: { code, message } }),
    }),
    [state, backend],
  );
  return <AppCtx.Provider value={value}>{children}</AppCtx.Provider>;
}

export function useApp(): Ctx {
  const ctx = useContext(AppCtx);
  if (!ctx) throw new Error('useApp must be used inside AppProvider');
  return ctx;
}
