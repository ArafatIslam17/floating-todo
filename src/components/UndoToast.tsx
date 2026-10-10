import { useEffect } from 'react';

export const UNDO_MS = 6000;

export function UndoToast({ title, onUndo, onExpire }: { title: string; onUndo(): void; onExpire(): void }) {
  useEffect(() => {
    const id = window.setTimeout(onExpire, UNDO_MS);
    return () => window.clearTimeout(id);
  }, [title, onExpire]);
  return (
    <div className="toast" role="status" aria-live="polite">
      <span className="min-w-0 flex-1 truncate">Deleted “{title}”</span>
      <button type="button" className="text-btn" onClick={onUndo} title="Undo (Ctrl+Z)">
        Undo
      </button>
    </div>
  );
}
