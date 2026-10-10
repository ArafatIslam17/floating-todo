import { ChevronsDownUp, ChevronsUpDown, Settings2, TriangleAlert, X } from 'lucide-react';

interface Props {
  productName: string;
  openCount: number;
  overdueCount: number;
  compact: boolean;
  onToggleCompact(): void;
  onOpenSettings(): void;
  onHide(): void;
}

function Mark() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" data-tauri-drag-region>
      <circle cx="8" cy="8" r="6" fill="none" stroke="var(--accent)" strokeOpacity="0.3" strokeWidth="1.8" />
      <path d="M8 2a6 6 0 0 1 6 6" fill="none" stroke="var(--accent)" strokeWidth="1.8" strokeLinecap="round" />
      <path d="M8 4.8V8l2.2 1.5" fill="none" stroke="var(--ink)" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function Header(p: Props) {
  return (
    <header data-tauri-drag-region data-overdue={p.overdueCount > 0 || undefined} className="app-header flex h-10 shrink-0 items-center gap-2 pr-1.5 pl-3">
      <Mark />
      <h1 data-tauri-drag-region className="app-name m-0 min-w-0 truncate text-head font-bold tracking-[-0.01em]">
        {p.productName}
      </h1>
      <p data-tauri-drag-region className="tnum m-0 flex shrink-0 items-center gap-1.5 text-meta font-semibold whitespace-nowrap text-ink-muted" aria-live="polite">
        <span data-tauri-drag-region>{p.openCount === 0 ? 'Nothing open' : `${p.openCount} open`}</span>
        {p.overdueCount > 0 && (
          <span data-tauri-drag-region className="inline-flex items-center gap-0.5 text-overdue">
            <TriangleAlert size={12} strokeWidth={2.4} aria-hidden />
            {p.overdueCount} overdue
          </span>
        )}
      </p>
      <div data-tauri-drag-region className="h-full flex-1" />
      <button
        type="button"
        className="icon-btn"
        onClick={p.onToggleCompact}
        aria-label={p.compact ? 'Expand widget' : 'Compact widget'}
        title={p.compact ? 'Expand' : 'Compact: show only the next task'}
      >
        {p.compact ? <ChevronsUpDown size={15} strokeWidth={2} /> : <ChevronsDownUp size={15} strokeWidth={2} />}
      </button>
      <button type="button" className="icon-btn" onClick={p.onOpenSettings} aria-label="Settings" title="Settings">
        <Settings2 size={15} strokeWidth={2} />
      </button>
      <button type="button" className="icon-btn" onClick={p.onHide} aria-label="Hide to tray" title="Hide to tray (reminders keep running)">
        <X size={16} strokeWidth={2} />
      </button>
    </header>
  );
}
