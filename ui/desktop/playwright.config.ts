import { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
  testDir: './tests/e2e',
  timeout: 60000,
  expect: {
    timeout: 30000
  },
  fullyParallel: false,
  workers: 1,
  reporter: [
    ['html'],
    ['list']
  ],
  use: {
    actionTimeout: 30000,
    navigationTimeout: 30000,
    video: 'on',
    screenshot: 'only-on-failure'
  },
  outputDir: 'test-results',
  preserveOutput: 'always'
};

export default config;