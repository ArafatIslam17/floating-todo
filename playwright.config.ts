import { defineConfig, devices } from '@playwright/test';

// UI flows run against the browser-preview backend (the desktop shell is covered by Rust tests
// and the desktop smoke run). Set PW_CHROMIUM_PATH to use an already-installed Chrome/Chromium.
export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:1420',
    ...devices['Desktop Chrome'],
    viewport: { width: 380, height: 560 },
    launchOptions: process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {},
  },
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
