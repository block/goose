import { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
  testDir: './tests/e2e',
  // E2E boots a full Electron + embedded goosed; allow a longer budget.
  timeout: 180000,
  expect: {
    timeout: 30000
  },
  fullyParallel: false,
  workers: 1,
  reporter: [
    // Never start a server automatically (prevents CI/dev runs from hanging).
    // Use `npm run test-e2e:report` to open the report manually.
    ['html', { open: 'never' }],
    ['list'],
  ],
  use: {
    actionTimeout: 30000,
    navigationTimeout: 30000,
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure'
  },
  outputDir: 'test-results',
  preserveOutput: 'always'
};

export default config;
