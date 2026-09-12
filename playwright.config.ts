import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  retries: 0,
  use: {
    baseURL: 'http://localhost:5174',
    viewport: { width: 1280, height: 800 },
  },
  webServer: {
    command: 'pnpm dev:web',
    port: 5174,
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
