// Deterministic timezone with DST for date tests. Node honours TZ changes at runtime.
declare const process: { env: Record<string, string | undefined> };
process.env.TZ = 'Europe/London';
import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(() => cleanup());
