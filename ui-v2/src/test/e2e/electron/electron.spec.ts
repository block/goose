import { spawn, exec } from 'child_process';
import { promisify } from 'util';

import { test, expect } from '@playwright/test';

const execAsync = promisify(exec);
let appProcess: any;

test.describe('electron app', () => {
  test.beforeAll(async () => {
    console.log('Starting Electron app...');

    // Always use electron-forge, but pass HEADLESS env var
    appProcess = spawn('npm', ['run', 'start:electron'], {
      stdio: 'pipe',
      shell: true,
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
        HEADLESS: process.env.HEADLESS || 'false',
        ELECTRON_START_URL: 'http://localhost:3001',
      },
    });

    // Wait for app to start
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  test.afterAll(async () => {
    console.log('Stopping Electron app...');
    await execAsync('pkill -9 -f electron || true');
    await new Promise((resolve) => setTimeout(resolve, 500));
    await execAsync('pkill -9 -f "npm run start:electron" || true');

    if (appProcess) {
      try {
        process.kill(-appProcess.pid);
      } catch {
        // Process might already be dead
      }
    }
  });

  test('shows correct runtime', async ({ page }) => {
    await page.goto('http://localhost:3001');
    await expect(page.locator('text=Running in: Electron')).toBeVisible();
  });
});
