import { defineConfig } from '@playwright/test';

// Browser tests of the built page (`npm run build`, then `npm run e2e`): the main paths in a real browser. They
// run against `vite preview`, which serves web/dist. In CI the browser is Chromium from Playwright; on a machine
// that has Edge or Chrome, PW_CHANNEL=msedge (or chrome) uses that one and needs no download.
export default defineConfig({
  testDir: 'e2e',
  testMatch: '*.e2e.ts',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: 'http://localhost:4173',
    // The page picks its language from the browser's: the tests start from English.
    locale: 'en-US',
    channel: process.env.PW_CHANNEL || undefined,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'npx vite preview --port 4173 --strictPort',
    url: 'http://localhost:4173/',
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
