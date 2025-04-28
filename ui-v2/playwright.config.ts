import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './src/test/e2e',
  workers: 1,
  use: {
    trace: 'on-first-retry',
    headless: false,
  },
  projects: [
    {
      name: 'web',
      testMatch: ['**/web/*.spec.ts'],
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'electron',
      testMatch: ['**/electron/*.spec.ts'],
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
  timeout: 30000,
  expect: {
    timeout: 10000,
  },
  reporter: [['html'], ['list']],
});
