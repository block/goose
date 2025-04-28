import { spawn, exec } from 'child_process';
import { promisify } from 'util';

import { test, expect } from '@playwright/test';

const execAsync = promisify(exec);
let webProcess: any;

test.describe('web app', () => {
  test.beforeAll(async () => {
    console.log('Starting web app...');

    // Start the vite dev server
    webProcess = spawn('npm', ['run', 'start:web'], {
      stdio: 'pipe',
      shell: true,
      env: {
        ...process.env,
        NODE_ENV: 'development',
      },
    });

    // Wait for server to start
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  test.afterAll(async () => {
    console.log('Stopping web app...');
    await execAsync('pkill -9 -f vite || true');

    if (webProcess) {
      try {
        process.kill(-webProcess.pid);
      } catch {
        // Process might already be dead
      }
    }
  });

  test('shows correct runtime', async ({ page }) => {
    await page.goto('http://localhost:3000');
    await expect(page.locator('text=Running in: Web Browser')).toBeVisible();
  });
});
