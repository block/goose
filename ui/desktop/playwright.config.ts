import { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
  testDir: './tests/e2e',
  timeout: 60000,
  expect: {
    timeout: 10000
  },
  fullyParallel: true,
  workers: 3,
  reporter: [
    ['html', { open: 'never' }],
    ['list']
  ],
  use: {
    actionTimeout: 10000,
    navigationTimeout: 30000,
    video: 'on',
    screenshot: 'only-on-failure'
  },
  outputDir: 'test-results',
  preserveOutput: 'always'
};

export default config;
