import { Info, X } from 'lucide-react';
import type { Issue } from '../domain/types';

export function IssueBanner({ issues, onDismiss }: { issues: Issue[]; onDismiss(code: string): void }) {
  if (issues.length === 0) return null;
  return (
    <div className="mx-2 mb-1.5 grid gap-1">
      {issues.map((i) => (
        <div
          key={i.code}
          role="status"
          className="flex items-start gap-2 rounded-[10px] border border-[var(--hairline-strong)] bg-[var(--surface-raised)] py-1.5 pr-1 pl-2.5 text-meta font-semibold text-ink"
        >
          <Info size={13} strokeWidth={2.2} className="mt-px shrink-0 text-ink-muted" aria-hidden />
          <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">{i.message}</span>
          <button type="button" className="icon-btn shrink-0" data-size="xs" aria-label="Dismiss message" onClick={() => onDismiss(i.code)}>
            <X size={12} strokeWidth={2.4} />
          </button>
        </div>
      ))}
    </div>
  );
}
