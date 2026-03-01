import { test as base, Page, _electron as electron } from '@playwright/test';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'path';
import path from 'node:path';
import dotenv from 'dotenv';


// Allow a developer/CI to provide a dotenv file (e.g. Azure/OpenAI creds) without
// exporting a large number of env vars manually.
//
// This runs in the Playwright test process, so `test.skip(process.env...)` gates
// in spec files can rely on it.
const dotenvPath = process.env.DOTENV_CONFIG_PATH;
if (dotenvPath) {
  const resolvedPath = dotenvPath.startsWith('/') ? dotenvPath : join(process.cwd(), dotenvPath);
  if (fs.existsSync(resolvedPath)) {
    dotenv.config({ path: resolvedPath, override: false });
  }
}

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

let buildOnce: Promise<void> | null = null;
let buildOnceMode: 'debug' | 'prod' | null = null;
let buildOnceStampMs: number | null = null;

function latestMtimeMs(path: string): number {
  if (!fs.existsSync(path)) return 0;

  const stat = fs.statSync(path);
  if (stat.isFile()) return stat.mtimeMs;

  if (!stat.isDirectory()) return stat.mtimeMs;

  let max = stat.mtimeMs;
  for (const entry of fs.readdirSync(path)) {
    // Avoid accidentally walking build output.
    if (entry === 'node_modules' || entry === '.vite' || entry === 'dist' || entry === 'out') {
      continue;
    }
    max = Math.max(max, latestMtimeMs(join(path, entry)));
  }

  return max;
}

function computeBuildStampMs(projectRoot: string): number {
  return Math.max(
    latestMtimeMs(join(projectRoot, 'src')),
    latestMtimeMs(join(projectRoot, 'package.json')),
    latestMtimeMs(join(projectRoot, 'package-lock.json')),
    latestMtimeMs(join(projectRoot, 'tsconfig.json')),
    latestMtimeMs(join(projectRoot, 'vite.renderer.config.mts')),
    latestMtimeMs(join(projectRoot, 'vite.main.config.mts')),
    latestMtimeMs(join(projectRoot, 'vite.preload.config.mts'))
  );
}

async function ensureViteBuild(projectRoot: string) {
  const isDebug = process.env.E2E_DEBUG === 'true';
  const mode: 'debug' | 'prod' = isDebug ? 'debug' : 'prod';

  const stampMs = computeBuildStampMs(projectRoot);

  // Playwright keeps one Node process per worker, so we cache builds for speed.
  // But we also need to rebuild when sources change, otherwise E2E runs can report
  // stale crashes/stacks even after a fix.
  if (buildOnceStampMs && stampMs > buildOnceStampMs) {
    buildOnce = null;
    buildOnceMode = null;
    buildOnceStampMs = null;
  }

  if (buildOnce && buildOnceMode && buildOnceMode !== mode) {
    buildOnce = null;
    buildOnceMode = null;
    buildOnceStampMs = null;
  }

  buildOnceMode = mode;

  buildOnce ??= (async () => {
    const vite = await import('vite');

    const buildMode = isDebug ? 'development' : 'production';

    // Build renderer/main/preload once per worker. Running Vite builds per-test is
    // expensive and can cause Node to OOM when a journey suite launches many apps.
    await vite.build({
      root: projectRoot,
      configFile: 'vite.renderer.config.mts',
      mode: buildMode,
      // Debug mode should keep React errors readable. In some bundling paths, React will
      // still emit minified error codes unless NODE_ENV is explicitly set at build time.
      define: isDebug ? { 'process.env.NODE_ENV': JSON.stringify('development') } : undefined,
      // For file:// loading (used in this E2E setup), Vite must emit relative asset
      // URLs; otherwise the renderer will request file:///assets/... and fail.
      base: './',
      build: {
        outDir: join(projectRoot, '.vite/renderer/main_window'),
        emptyOutDir: true,
        // Debug runs need actionable stack traces (React dev errors + sourcemaps).
        // CI runs should stay as close to production as possible.
        minify: isDebug ? false : true,
        sourcemap: isDebug ? true : false,
      },
    });

    await vite.build({ root: projectRoot, configFile: 'vite.main.config.mts', mode: buildMode });
    await vite.build({ root: projectRoot, configFile: 'vite.preload.config.mts', mode: buildMode });

    buildOnceStampMs = stampMs;
  })();

  await buildOnce;
}

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
    testInfo.setTimeout(180_000);
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

    let mainLog: string[] = [];
    let rendererLog: string[] = [];
    let page: Page | null = null;
    let rendererDebugAttached = false;
    let mainDebugAttached = false;

    const attachRendererDebug = async (reason: string) => {
      if (rendererDebugAttached || !page) return;
      rendererDebugAttached = true;

      const url = page.url();
      const title = await page.title().catch(() => '(failed to read page title)');
      const html = await page.content().catch(() => '(failed to read page content)');

      const body = [
        `reason: ${reason}`,
        `url: ${url}`,
        `title: ${title}`,
        '',
        '--- console/page errors ---',
        rendererLog.join('\n') || '(no console output captured)',
        '',
        '--- html (truncated) ---',
        html.slice(0, 20_000),
      ].join('\n');

      // Ensure the debug output exists on disk (Playwright attachments are not always
      // preserved as plain files depending on reporter/output settings).
      const outPath = testInfo.outputPath('renderer-debug.txt');
      await mkdir(dirname(outPath), { recursive: true });
      await writeFile(outPath, body, 'utf8');

      await testInfo.attach('renderer-debug.txt', {
        body,
        contentType: 'text/plain',
      });
    };

    const attachMainDebug = async () => {
      if (mainDebugAttached || mainLog.length === 0) return;
      mainDebugAttached = true;

      const body = mainLog.join('');
      const outPath = testInfo.outputPath('main-debug.txt');
      await mkdir(dirname(outPath), { recursive: true });
      await writeFile(outPath, body, 'utf8');

      await testInfo.attach('main-debug.txt', {
        body,
        contentType: 'text/plain',
      });
    };

    try {
      const projectRoot = join(__dirname, '../..');
      const repoRoot = join(projectRoot, '..', '..');

      let earlyDebugTimer: ReturnType<typeof setTimeout> | undefined;

      // The Electron main process starts goosed. In many dev environments it's
      // not present unless you've built the Rust workspace.
      ensureGoosedBinary(repoRoot);

      // Build renderer/main/preload once per worker. Using a file-based renderer avoids
      // Vite dev-server dep-scan flakiness and CJS/ESM interop issues.
      await ensureViteBuild(projectRoot);

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

      const electronArgs = [projectRoot];
      const electronEnv = {
        ...process.env,
        // If callers provide only defaults, map to GOOSE_PROVIDER; if callers provide only
        // GOOSE_PROVIDER, map to defaults so ProviderGuard sees a configured provider.
        GOOSE_PROVIDER: process.env.GOOSE_PROVIDER ?? process.env.GOOSE_DEFAULT_PROVIDER,
        GOOSE_DEFAULT_PROVIDER: process.env.GOOSE_DEFAULT_PROVIDER ?? process.env.GOOSE_PROVIDER,

        // Azure OpenAI typically needs the deployment name to be used as the model identifier.
        // If the caller didn't provide GOOSE_MODEL/GOOSE_DEFAULT_MODEL, infer them.
        GOOSE_MODEL:
          process.env.GOOSE_MODEL ??
          (process.env.GOOSE_PROVIDER === 'azure_openai' ||
          process.env.GOOSE_DEFAULT_PROVIDER === 'azure_openai'
            ? process.env.AZURE_OPENAI_DEPLOYMENT_NAME
            : undefined),
        GOOSE_DEFAULT_MODEL:
          process.env.GOOSE_DEFAULT_MODEL ??
          process.env.GOOSE_MODEL ??
          (process.env.GOOSE_PROVIDER === 'azure_openai' ||
          process.env.GOOSE_DEFAULT_PROVIDER === 'azure_openai'
            ? process.env.AZURE_OPENAI_DEPLOYMENT_NAME
            : undefined),
      };

      // Azure OpenAI typically needs the deployment name to be used as the model identifier.
      // We already infer model identifiers into GOOSE_MODEL/GOOSE_DEFAULT_MODEL above when the
      // provider is azure_openai, so no additional model inference is needed here.

		if (process.platform === 'linux') {
			// Under Playwright we run Electron inside Xvfb on Linux.
			// Keep the launch config minimal to avoid hangs.
			electronArgs.push('--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu');
		}
		const launchTimeoutMs = 45_000;
		console.log(`[e2e] launching electron (timeout=${launchTimeoutMs}ms)`);
		const launchPromise = electron.launch({
			executablePath: electronPath,
			// Must point to the Electron *main process* entry.
			args: [builtMainPath, ...electronArgs.slice(1)],
			env: {
				...electronEnv,
				VITE_START_EMBEDDED_SERVER: 'yes',
				GOOSE_ALLOWLIST_BYPASS: 'true',
				ENABLE_PLAYWRIGHT: 'true',
				ELECTRON_ENABLE_LOGGING: 'true',
				ELECTRON_ENABLE_STACK_DUMPING: 'true',
				ELECTRON_CRASH_REPORTER_DISABLE: 'true',
				ELECTRON_DISABLE_SECURITY_WARNINGS: 'true',
			},
		});
		app = await Promise.race([
			launchPromise,
			new Promise<never>((_, reject) =>
				setTimeout(
					() => reject(new Error(`electron.launch timed out after ${launchTimeoutMs}ms`)),
					launchTimeoutMs
				)
			),
		]);
		console.log('[e2e] electron launched');

      const child = app.process();
      child?.stdout?.on('data', (buf) => {
        const line = buf.toString();
        mainLog.push(`[main:stdout] ${line}`);
        process.stdout.write(`[e2e][main:stdout] ${line}`);
      });
      child?.stderr?.on('data', (buf) => {
        const line = buf.toString();
        mainLog.push(`[main:stderr] ${line}`);
        process.stderr.write(`[e2e][main:stderr] ${line}`);
      });

      // Electron can create multiple windows; pick the main window deterministically.
      const mainWindowDeadline = Date.now() + 60_000;
      while (true) {
        const windows = app.windows();
        const mainWindow = windows.find((w) =>
          w.url().includes('/.vite/renderer/main_window/index.html')
        );

        if (mainWindow) {
          page = mainWindow;
          break;
        }

        if (Date.now() > mainWindowDeadline) {
          const urls = windows.map((w) => w.url()).join(', ');
          throw new Error(`Timed out waiting for main window. Window URLs: ${urls}`);
        }

        await new Promise((resolve) => setTimeout(resolve, 200));
      }

      // Watchdog: Playwright can hang inside electron.launch / waitForLoadState without letting
      // Node timers flush logs. Polling keeps the process responsive and produces actionable
      // diagnostics when setup stalls.
      let isReady = false;
      const watchdog = (async () => {
        const start = Date.now();
        while (!isReady && Date.now() - start < 30_000) {
          process.stdout.write(`[e2e] watchdog: url=${page?.url() ?? 'undefined'}\n`);
          await new Promise((resolve) => setTimeout(resolve, 2_000));
        }

        if (isReady) return;

        process.stdout.write(
          `[e2e] watchdog timeout: url=${page?.url() ?? 'undefined'} windows=${app
            ?.windows()
            .map((w) => w.url())
            .join(', ')} mainLogBytes=${mainLog.join('').length} rendererLogLines=${rendererLog.length}\n`
        );

        await attachRendererDebug('watchdog timeout waiting for renderer ready');
        await attachMainDebug();
      })();

      page.on('console', (msg) => {
        void (async () => {
          const location = msg.location();
          const locSuffix = location?.url
            ? ` @ ${location.url}:${location.lineNumber ?? 0}:${location.columnNumber ?? 0}`
            : '';

          const args = msg.args();
          const argValues = await Promise.all(
            args.map(async (arg) => {
              try {
                return await arg.jsonValue();
              } catch {
                return '[unserializable]';
              }
            })
          );

          const serializedArgs = argValues.length ? ` ${JSON.stringify(argValues)}` : '';
          rendererLog.push(`[console:${msg.type()}] ${msg.text()}${serializedArgs}${locSuffix}`);
        })();
      });
      page.on('pageerror', (err) => {
        rendererLog.push(`[pageerror] ${err.message}\n${err.stack ?? ''}`);
      });
      page.on('requestfailed', (req) => {
        rendererLog.push(
          `[requestfailed] ${req.method()} ${req.url()} :: ${req.failure()?.errorText ?? 'unknown error'}`
        );
      });

      // The first window can be created before the main process finishes bootstrapping.
      // Waiting on a URL change is not a useful signal here because the Electron renderer
      // usually stays on file://.../#/... and may never "navigate away".
      await Promise.race([
        page.waitForLoadState('domcontentloaded'),
        new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error('Timed out waiting for domcontentloaded')), 45_000)
        ),
      ]);

      // Capture useful stacks for otherwise stack-less React console errors.
      // React 19 can emit "Maximum update depth exceeded" without a component stack;
      // patching console.error lets us log a stack once from the callsite.
      await Promise.race([
        page.evaluate(() => {
          const marker = '__gooseE2EConsoleErrorPatched';
          if ((window as unknown as Record<string, unknown>)[marker]) return;
          (window as unknown as Record<string, unknown>)[marker] = true;

          const original = console.error.bind(console);
          console.error = (...args: unknown[]) => {
            try {
              const first = args[0];
              if (typeof first === 'string' && first.includes('Maximum update depth exceeded')) {
                original('[goose:e2e] Maximum update depth stack', new Error().stack);
              }
            } catch {
              // ignore
            }
            original(...args);
          };
        }),
        new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error('Timed out patching console.error')), 45_000)
        ),
      ]);


      // Try to wait for networkidle
      try {
        await page.waitForLoadState('networkidle', { timeout: 10000 });
      } catch (error) {
        console.log('NetworkIdle timeout (likely due to MCP activity), continuing...');
      }

      // Wait for the app chrome to be ready.
      // We key off the global app shell because it is always present once React renders.
      try {
        await page.waitForSelector('[data-testid="app-shell"]', { timeout: 150_000 });
      } catch (error) {
        console.log('[e2e] wait for app-shell failed', {
          url: page?.url(),
          windowUrls: app?.windows().map((w) => w.url()),
          mainLogBytes: mainLog.join('').length,
          rendererLogLines: rendererLog.length,
        });
        await attachRendererDebug('timeout waiting for app shell');
        await attachMainDebug();
        throw error;
      }

      isReady = true;
      await watchdog;

      if (earlyDebugTimer) {
        clearTimeout(earlyDebugTimer);
      }
      // Fail-fast if we landed on the app ErrorBoundary ("Honk!"). This is a runtime crash
      // and subsequent selector assertions will be misleading.
      const honk = page.getByRole('heading', { name: /^honk!$/i });
      if (await honk.isVisible().catch(() => false)) {
        await attachRendererDebug('error boundary visible (honk)');
        await attachMainDebug();
        throw new Error('App crashed (ErrorBoundary "Honk!" visible). See renderer-debug.txt');
      }

      console.log('App ready, starting test...');

      if (earlyDebugTimer) {
        clearTimeout(earlyDebugTimer);
      }

      // Provide the page to the test
      await use(page);

    } finally {
      console.log('Cleaning up Electron app for this test...');

      if (testInfo.status !== testInfo.expectedStatus) {
        await attachRendererDebug('test failed');
        await attachMainDebug();
      }

      await app?.close().catch(console.error);
      console.log('Cleaned up app');
    }
  },
});

export { expect } from '@playwright/test';
