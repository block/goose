import { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
  testDir: './tests/e2e-release',
  timeout: 120000,
  expect: {
    timeout: 45000
  },
  fullyParallel: false,
  workers: 1,
  reporter: process.env.CI ? [
    ['html'],
    ['list']
  ] : [
    ['list'] // Only use list reporter for local runs
  ],
  use: {
    actionTimeout: 45000,
    navigationTimeout: 45000,
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure'
  },
  outputDir: 'test-results-release',
  preserveOutput: 'failures-only',
  // Use headed mode for local development if specified
  headed: process.env.HEADED === 'true'
};

export default config;