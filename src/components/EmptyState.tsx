export function EmptyState({ kind }: { kind: 'empty' | 'all-done' }) {
  return (
    <div className="grid h-full place-items-center px-6 py-6 text-center">
      <p className="m-0 max-w-[30ch] text-body font-semibold text-ink-muted">
        {kind === 'empty'
          ? 'Nothing on your list. Type a task below and press Enter.'
          : 'All done. Completed tasks are below.'}
      </p>
    </div>
  );
}
