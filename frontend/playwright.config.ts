import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: 'tests/e2e',
  fullyParallel: true,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : undefined,
  use: {
    baseURL: 'http://127.0.0.1:8080',
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    ignoreHTTPSErrors: true,
    viewport: { width: 1280, height: 800 },
  },
  // Backend server should already be running on port 8080
  // webServer: {
  //   command: 'cargo run',
  //   port: 8080,
  //   reuseExistingServer: true,
  //   timeout: 120000,
  // },
});
