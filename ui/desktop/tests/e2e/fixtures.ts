import { test as base, Page, _electron as electron } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import { join } from 'path';

function ensureGoosedBinary(repoRoot: string) {
  const executableName = process.platform === 'win32' ? 'goosed.exe' : 'goosed';
  const goosedPath = join(repoRoot, 'target', 'debug', executableName);

  if (fs.existsSync(goosedPath)) return;

  console.log(`[e2e] goosed not found at ${goosedPath}; building...`);

  const result = spawnSync('cargo', ['build', '-p', 'goose-server', '--bin', 'goosed'], {
    cwd: repoRoot,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    throw new Error(`Failed to build goosed (exit code ${result.status ?? 'unknown'})`);
  }

  if (!fs.existsSync(goosedPath)) {
    throw new Error(`Built goosed but binary still not found at ${goosedPath}`);
  }
}

type GooseTestFixtures = {
  goosePage: Page;
};

/**
 * Test-scoped fixture that launches a fresh Electron app for EACH test.
 *
 * Isolation: ⚠️ Partial - each test gets a fresh app instance, but uses ambient user config
 * Speed: ⚠️ Slow - ~3s startup overhead per test
 *
 * This ensures each test starts with a fresh app instance, but the app uses the
 * user's existing Goose configuration (providers, models, etc.).
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

    if (process.platform === 'linux' && !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY) {
      throw new Error(
        [
          'Playwright E2E requires a display server on Linux.',
          'No DISPLAY/WAYLAND_DISPLAY detected.',
          '',
          'Fix options:',
          '  1) Install Xvfb and re-run (recommended for CI):',
          '       sudo apt-get update && sudo apt-get install -y xvfb',
          '  2) Run in a desktop session (set DISPLAY or WAYLAND_DISPLAY).',
        ].join('\n')
      );
    }

    const electronPath = require('electron') as string;
    let app: import('@playwright/test').ElectronApplication | null = null;

    try {
      const projectRoot = join(__dirname, '../..');
      const repoRoot = join(projectRoot, '..', '..');

      // The Electron main process starts goosed. In many dev environments it's
      // not present unless you've built the Rust workspace.
      ensureGoosedBinary(repoRoot);

      const vite = await import('vite');

      // Build renderer/main/preload once per test run.
      // Using a file-based renderer avoids Vite dev-server dep-scan flakiness and
      // CJS/ESM interop issues (named exports) when optimizeDeps is constrained.
      await vite.build({
        root: projectRoot,
        configFile: 'vite.renderer.config.mts',
        // For file:// loading (used in this E2E setup), Vite must emit relative asset
        // URLs; otherwise the renderer will request file:///assets/... and fail.
        base: './',
        build: {
          outDir: join(projectRoot, '.vite/renderer/main_window'),
          emptyOutDir: true,
        },
      });
      await vite.build({ root: projectRoot, configFile: 'vite.main.config.mts' });
      await vite.build({ root: projectRoot, configFile: 'vite.preload.config.mts' });

      const builtMainPath = join(projectRoot, '.vite/build/main.js');
      if (!fs.existsSync(builtMainPath)) {
        const viteDir = join(projectRoot, '.vite');
        const viteListing = fs.existsSync(viteDir) ? fs.readdirSync(viteDir) : [];
        throw new Error(
          `Expected Electron main bundle at ${builtMainPath}, but it does not exist. `.concat(
            `Contents of ${viteDir}: ${JSON.stringify(viteListing)}`
          )
        );
      }

      app = await electron.launch({
        executablePath: electronPath,
        // Launch the Electron *app* (directory with package.json). This matches Playwright's
        // expected model and avoids flaky behavior when passing a raw JS entry.
        args: [projectRoot],
        env: {
          ...process.env,
          NODE_ENV: 'development',
          ELECTRON_IS_DEV: '1',
          GOOSE_ALLOWLIST_BYPASS: 'true',
          ENABLE_PLAYWRIGHT: 'true',
          MAIN_WINDOW_VITE_NAME: 'main_window',
        },
      });

      const page = await app.firstWindow();

      const mainLog: string[] = [];
      const child = app.process();
      child?.stdout?.on('data', (buf) => mainLog.push(`[main:stdout] ${buf.toString()}`));
      child?.stderr?.on('data', (buf) => mainLog.push(`[main:stderr] ${buf.toString()}`));

      const rendererLog: string[] = [];
      page.on('console', (msg) => {
        rendererLog.push(`[console:${msg.type()}] ${msg.text()}`);
      });
      page.on('pageerror', (err) => {
        rendererLog.push(`[pageerror] ${err.message}\n${err.stack ?? ''}`);
      });
      page.on('requestfailed', (req) => {
        rendererLog.push(
          `[requestfailed] ${req.method()} ${req.url()} :: ${req.failure()?.errorText ?? 'unknown error'}`
        );
      });

      // The first window can be created before the main process finishes bootstrapping
      // (e.g. while waiting on the backend to become ready). Wait until it actually
      // navigates away from about:blank.
      await page.waitForURL(/^(file|http):/i, { timeout: 60_000 });

      // Wait for page to be ready
      await page.waitForLoadState('domcontentloaded');

      // Try to wait for networkidle
      try {
        await page.waitForLoadState('networkidle', { timeout: 10000 });
      } catch (error) {
        console.log('NetworkIdle timeout (likely due to MCP activity), continuing...');
      }

      // Wait for React app to be ready.
      try {
        await page.waitForFunction(
          () => {
            const root = document.getElementById('root');
            return root && root.children.length > 0;
          },
          undefined,
          { timeout: 60_000 }
        );
      } catch (error) {
        const url = page.url();
        const html = await page.content().catch(() => '(failed to read page content)');
        const title = await page.title().catch(() => '(failed to read page title)');

        await testInfo.attach('renderer-debug.txt', {
          body: [
            `url: ${url}`,
            `title: ${title}`,
            '',
            '--- console/page errors ---',
            rendererLog.join('\n') || '(no console output captured)',
            '',
            '--- html (truncated) ---',
            html.slice(0, 20_000),
          ].join('\n'),
          contentType: 'text/plain',
        });

        if (mainLog.length > 0) {
          await testInfo.attach('main-debug.txt', {
            body: mainLog.join(''),
            contentType: 'text/plain',
          });
        }

        throw error;
      }

      console.log('App ready, starting test...');

      // Provide the page to the test
      await use(page);

    } finally {
      console.log('Cleaning up Electron app for this test...');

      await app?.close().catch(console.error);
      console.log('Cleaned up app');
    }
  },
});

export { expect } from '@playwright/test';
