import { spawn } from 'child_process';
import { Buffer } from 'node:buffer';

import { test, expect } from '@playwright/test';

let electronProcess: ReturnType<typeof spawn> | undefined;

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

test.describe('electron app', () => {
  test.beforeAll(async () => {
    console.log('Starting Electron app...');
    console.log('Environment:', process.env.NODE_ENV);
    console.log('HEADLESS:', process.env.HEADLESS);

    // Start electron with minimal memory settings
    electronProcess = spawn('npm', ['run', 'start:electron'], {
      stdio: 'pipe',
      shell: true,
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
        HEADLESS: process.env.HEADLESS || 'false',
        ELECTRON_START_URL: 'http://localhost:3001',
        // Add memory limits for Electron
        ELECTRON_EXTRA_LAUNCH_ARGS: '--js-flags="--max-old-space-size=512" --disable-gpu',
      },
      // Set detached to false and create a new process group
      detached: false,
    });

    // Store the PID for cleanup
    const pid = electronProcess.pid;
    console.log('Started Electron app with PID:', pid);

    // Capture stdout and stderr for debugging
    electronProcess.stdout?.on('data', (data: Buffer) => {
      console.log(`Electron stdout: ${data.toString()}`);
    });

    electronProcess.stderr?.on('data', (data: Buffer) => {
      console.log(`Electron stderr: ${data.toString()}`);
    });

    // Wait for the app to be ready
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  test.afterAll(async () => {
    console.log('Stopping Electron app...');

    if (electronProcess?.pid) {
      console.log('Killing Electron process:', electronProcess.pid);
      await killProcess(electronProcess.pid);
    }

    // Give processes time to fully terminate
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  test('shows correct runtime', async ({ page }) => {
    console.log('Navigating to http://localhost:3001');
    const response = await page.goto('http://localhost:3001');
    console.log('Navigation status:', response?.status());

    // Wait for and check the text with more detailed logging
    console.log('Looking for runtime text...');
    const runtimeText = page.locator('text=Running in: Electron');

    // Wait for the text to be visible
    await expect(runtimeText).toBeVisible({ timeout: 10000 });
  });
});
