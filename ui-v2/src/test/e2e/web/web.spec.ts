import { spawn } from 'child_process';
import http from 'http';
import { Buffer } from 'node:buffer';

import { test, expect } from '@playwright/test';

let webProcess: ReturnType<typeof spawn> | undefined;

// Helper function to check if server is ready
async function waitForServer(url: string, timeout: number): Promise<boolean> {
  const start = Date.now();

  while (Date.now() - start < timeout) {
    try {
      await new Promise<void>((resolve, reject) => {
        const req = http.get(url, (res) => {
          if (res.statusCode === 200) {
            resolve();
          } else {
            reject(new Error(`Status code: ${res.statusCode}`));
          }
          res.resume(); // Consume response data to free up memory
        });

        req.on('error', reject);
        req.setTimeout(1000, () => reject(new Error('Request timeout')));
      });

      console.log('Server is ready');
      return true;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      console.log('Waiting for server...', message);
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }

  console.log('Server failed to respond in time');
  return false;
}

// Helper function to safely kill a process and its children
async function killProcess(pid: number): Promise<void> {
  try {
    // Try to kill the process group first
    try {
      process.kill(-pid, 'SIGTERM');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      console.log('Failed to kill process group:', message);
    }

    // Wait a bit and then try to kill the process directly
    await new Promise((resolve) => setTimeout(resolve, 1000));

    try {
      process.kill(pid, 'SIGTERM');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      console.log('Failed to kill process:', message);
    }

    // Final cleanup with SIGKILL if needed
    await new Promise((resolve) => setTimeout(resolve, 1000));
    try {
      process.kill(pid, 'SIGKILL');
    } catch (error: unknown) {
      // Process is probably already dead
      const message = error instanceof Error ? error.message : String(error);
      console.log('Process already terminated:', message);
    }
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.log('Error during process cleanup:', message);
  }
}

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
      // Set detached to false and create a new process group
      detached: false,
    });

    // Store the PID for cleanup
    const pid = webProcess.pid;
    console.log('Started Vite server with PID:', pid);

    // Capture stdout and stderr for debugging
    webProcess.stdout?.on('data', (data: Buffer) => {
      console.log(`Vite stdout: ${data.toString()}`);
    });

    webProcess.stderr?.on('data', (data: Buffer) => {
      console.log(`Vite stderr: ${data.toString()}`);
    });

    // Wait for server to be ready
    console.log('Waiting for server to be ready...');
    const serverReady = await waitForServer('http://localhost:3000', 30000);
    if (!serverReady) {
      throw new Error('Server failed to start');
    }
  });

  test.afterAll(async () => {
    console.log('Cleaning up processes...');

    if (webProcess?.pid) {
      console.log('Killing Vite server process:', webProcess.pid);
      await killProcess(webProcess.pid);
    }

    // Give processes time to fully terminate
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  test('shows correct runtime', async ({ page }) => {
    console.log('Navigating to http://localhost:3000');
    const response = await page.goto('http://localhost:3000');
    console.log('Navigation status:', response?.status());

    // Wait for and check the text with more detailed logging
    console.log('Looking for runtime text...');
    const runtimeText = page.locator('text=Running in: Web Browser');

    // Wait for the text to be visible
    await expect(runtimeText).toBeVisible({ timeout: 10000 });
  });
});
