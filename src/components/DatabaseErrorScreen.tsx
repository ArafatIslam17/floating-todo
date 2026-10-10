import { RotateCcw } from 'lucide-react';
import { useState } from 'react';

export function DatabaseErrorScreen({
  message,
  path,
  onRetry,
  onQuit,
}: {
  message: string;
  path: string;
  onRetry(): Promise<unknown>;
  onQuit(): void;
}) {
  const [busy, setBusy] = useState(false);
  return (
    <div className="grid flex-1 content-center gap-3 px-5 pb-6" role="alert">
      <h2 className="m-0 text-title font-bold">Can't open your task database.</h2>
      <p className="m-0 text-body text-ink-muted [overflow-wrap:anywhere]">{message}</p>
      <p className="m-0 text-meta text-ink-muted">
        Your file was not changed. It is at:
        <br />
        <code className="text-ink [overflow-wrap:anywhere] select-text">{path}</code>
      </p>
      <p className="m-0 text-meta text-ink-muted">
        Close other copies of the app, check the file isn't read-only, then retry. To start fresh, move the file
        somewhere else first.
      </p>
      <div className="flex gap-2">
        <button
          type="button"
          className="text-btn inline-flex items-center gap-1.5"
          data-primary
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            await onRetry().finally(() => setBusy(false));
          }}
        >
          <RotateCcw size={13} strokeWidth={2.4} aria-hidden /> Retry
        </button>
        <button type="button" className="text-btn" onClick={onQuit}>
          Quit
        </button>
      </div>
    </div>
  );
}
