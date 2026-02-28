import { test as base, Page, Browser, chromium } from '@playwright/test';
import { spawn, ChildProcess } from 'child_process';
import { join } from 'path';
import { promisify } from 'util';
import * as fs from 'fs';
import * as os from 'os';

const execAsync = promisify(require('child_process').exec);

type GooseTestFixtures = {
  goosePage: Page;
};

/**
 * Test-scoped fixture that launches a fresh Electron app for EACH test.
 *
 * Isolation: ✅ Full - each test gets a fresh app instance with an isolated config directory
 *   via GOOSE_PATH_ROOT, so tests never touch ~/.config/goose/
 * Speed: ⚠️ Slow - ~3s startup overhead per test
 *
 * Usage:
 *   import { test, expect } from './fixtures';
 *
 *   test('my test', async ({ goosePage }) => {
 *     await goosePage.waitForSelector('[data-testid="chat-input"]');
 *     // ... test code
 *   });
 */
export const test = base.extend<GooseTestFixtures>({
  // Test-scoped fixture: launches a fresh Electron app for each test
  goosePage: async ({}, use, testInfo) => {
    console.log(`Launching fresh Electron app for test: ${testInfo.title}`);

    let appProcess: ChildProcess | null = null;
    let browser: Browser | null = null;

    // Create an isolated config directory so tests never touch ~/.config/goose/
    const tempDir = fs.mkdtempSync(join(os.tmpdir(), 'goose-test-'));
    const configDir = join(tempDir, 'config');
    fs.mkdirSync(configDir, { recursive: true });
    fs.mkdirSync(join(tempDir, 'data'), { recursive: true });
    fs.mkdirSync(join(tempDir, 'state'), { recursive: true });
    fs.writeFileSync(join(configDir, 'config.yaml'), 'GOOSE_PROVIDER: databricks\nGOOSE_MODEL: databricks-claude-haiku-4-5\nGOOSE_TELEMETRY_ENABLED: false\nDATABRICKS_HOST: https://block-lakehouse-production.cloud.databricks.com/\n');
    console.log(`Using isolated config directory: ${tempDir}`);

    try {
      // Assign a unique debug port for this test to enable parallel execution
      // Base port 9222, offset by worker index * 100 + parallel slot
      const debugPort = 9222 + (testInfo.parallelIndex * 10);
      console.log(`Using debug port ${debugPort} for parallel test execution`);

      // Start the electron-forge process with Playwright remote debugging enabled
      // Use detached mode on Unix to create a process group we can kill together
      appProcess = spawn('npm', ['run', 'start-gui'], {
        cwd: join(__dirname, '../..'),
        stdio: 'pipe',
        detached: process.platform !== 'win32',
        env: {
          ...process.env,
          ELECTRON_IS_DEV: '1',
          NODE_ENV: 'development',
          GOOSE_ALLOWLIST_BYPASS: 'true',
          ENABLE_PLAYWRIGHT: 'true',
          PLAYWRIGHT_DEBUG_PORT: debugPort.toString(), // Unique port per test for parallel execution
          RUST_LOG: 'info', // Enable info-level logging for goosed backend
          GOOSE_PATH_ROOT: tempDir,
        }
      });

      // Log process output for debugging
      if (process.env.DEBUG_TESTS) {
        appProcess.stdout?.on('data', (data) => {
          console.log('App stdout:', data.toString());
        });

        appProcess.stderr?.on('data', (data) => {
          console.log('App stderr:', data.toString());
        });
      }

      // Wait for the app to start and remote debugging to be available
      // Retry connection until it succeeds (app is ready) or timeout
      console.log(`Waiting for Electron app to start on port ${debugPort}...`);
      const maxRetries = 100; // 100 retries * 100ms = 10 seconds max
      const retryDelay = 100; // 100ms between retries

      for (let attempt = 1; attempt <= maxRetries; attempt++) {
        try {
          browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
          console.log(`Connected to Electron app on attempt ${attempt} (~${(attempt * retryDelay) / 1000}s)`);
          break;
        } catch (error) {
          if (attempt === maxRetries) {
            throw new Error(`Failed to connect to Electron app after ${maxRetries} attempts (${(maxRetries * retryDelay) / 1000}s). Last error: ${error.message}`);
          }
          // Wait before next retry
          await new Promise(resolve => setTimeout(resolve, retryDelay));
        }
      }

      if (!browser) {
        throw new Error('Browser connection failed unexpectedly');
      }

      // Get the electron app context and first page
      const contexts = browser.contexts();
      if (contexts.length === 0) {
        throw new Error('No browser contexts found');
      }

      const pages = contexts[0].pages();
      if (pages.length === 0) {
        throw new Error('No windows/pages found');
      }

      const page = pages[0];

      // Wait for page to be ready
      await page.waitForLoadState('domcontentloaded');

      // Try to wait for networkidle
      try {
        await page.waitForLoadState('networkidle', { timeout: 10000 });
      } catch (error) {
        console.log('NetworkIdle timeout (likely due to MCP activity), continuing...');
      }

      // Wait for React app to be ready
      await page.waitForFunction(() => {
        const root = document.getElementById('root');
        return root && root.children.length > 0;
      }, { timeout: 30000 });

      console.log('App ready, starting test...');

      // Provide the page to the test
      await use(page);

    } finally {
      console.log('Cleaning up Electron app for this test...');

      // Close the CDP connection
      if (browser) {
        await browser.close().catch(console.error);
      }

      // Kill the npm process tree
      if (appProcess && appProcess.pid) {
        try {
          if (process.platform === 'win32') {
            // On Windows, kill the entire process tree
            await execAsync(`taskkill /F /T /PID ${appProcess.pid}`);
          } else {
            // On Unix, kill the entire process group
            try {
              // First try SIGTERM for graceful shutdown
              process.kill(-appProcess.pid, 'SIGTERM');
              await new Promise(resolve => setTimeout(resolve, 2000));
            } catch (e) {
              // Process might already be dead
            }
            // Then SIGKILL if still running
            try {
              process.kill(-appProcess.pid, 'SIGKILL');
            } catch (e) {
              // Process already exited
            }
          }
          console.log('Cleaned up app process');
        } catch (error) {
          if (error.code !== 'ESRCH' && !error.message?.includes('No such process')) {
            console.error('Error killing app process:', error);
          }
        }
      }

      // Clean up the isolated config directory
      try {
        fs.rmSync(tempDir, { recursive: true, force: true });
        console.log('Cleaned up isolated config directory');
      } catch (error) {
        console.error('Error cleaning up temp directory:', error);
      }
    }
  },
});

export { expect } from '@playwright/test';
