import { useMemo } from 'react';
import { Widget } from './components/Widget';
import { getBackend, type Backend } from './lib/backend';
import { AppProvider } from './state/AppContext';

export function App({ backend }: { backend?: Promise<Backend> }) {
  const promise = useMemo(() => backend ?? getBackend(), [backend]);
  return (
    <AppProvider backendPromise={promise}>
      <Widget />
    </AppProvider>
  );
}
