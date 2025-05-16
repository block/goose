import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './src/test/e2e',
  workers: 1,
  use: {
    trace: 'on-first-retry',
    // Use headless mode in CI, non-headless locally unless specified
    headless: process.env.CI === 'true' || process.env.HEADLESS === 'true',
    // Add longer timeouts for CI
    navigationTimeout: 30000,
    actionTimeout: 15000,
  },
  projects: [
    {
      name: 'electron',
      testMatch: ['**/electron/*.spec.ts'],
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
  timeout: 60000, // Increase overall timeout
  expect: {
    timeout: 15000, // Increase expect timeout
  },
  reporter: [['html'], ['list']],
});