import { test as base, expect, Page, _electron as electron } from '@playwright/test';
import { promisify } from 'util';
import { exec } from 'child_process';
import { join } from 'path';
import * as fs from 'fs';
import * as os from 'os';

const execAsync = promisify(exec);

type GooseTestFixtures = {
  goosePage: Page;
};

const isDebug = () => process.env.DEBUG_TESTS === '1' || process.env.DEBUG_TESTS === 'true';
const debugLog = (message: string) => {
  if (isDebug()) {
    console.log(message);
  }
};

export const test = base.extend<GooseTestFixtures>({
  goosePage: async ({}, use, testInfo) => {
    let electronApp: Awaited<ReturnType<typeof electron.launch>> | null = null;
    let page: Page | null = null;
    let tracingStarted = false;
    const appRoot = join(__dirname, '../..');
    const executablePath = resolvePackagedExecutable(appRoot);

    if (!fs.existsSync(executablePath)) {
      throw new Error(
        `Packaged app executable not found at ${executablePath}. Build it first (e.g. "cd ui/desktop && npm run package"), or set GOOSE_PACKAGED_EXECUTABLE.`
      );
    }

    const tempDir = createIsolatedGoosePathRoot();

    try {
      const videoDir = process.env.PW_ELECTRON_VIDEO === '1' ? testInfo.outputPath('videos') : undefined;
      const launchOptions = buildLaunchOptions(executablePath, tempDir, videoDir);
      debugLog(`Launching packaged Electron for test: ${testInfo.title}`);
      debugLog(`Using packaged executable: ${executablePath}`);

      electronApp = await electron.launch(launchOptions);
      attachAppDebugLogs(electronApp);

      await electronApp.firstWindow({ timeout: 30000 });
      page = await waitForReadyAppWindow(electronApp, 60000);
      await page.waitForLoadState('domcontentloaded', { timeout: 10000 }).catch(() => {});
      debugLog(`Selected app window URL: ${page.url()}`);
      attachPageDebugLogs(page);

      await page.context().tracing.start({
        screenshots: true,
        snapshots: true,
        sources: true,
      });
      tracingStarted = true;

      await use(page);
    } finally {
      if (page && tracingStarted) {
        try {
          await page.context().tracing.stop({ path: testInfo.outputPath('trace.zip') });
        } catch {
          // Tracing stop can fail if context has already been torn down.
        }
      }

      if (electronApp) {
        await closeWithFallback(electronApp);
      }
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  },
});

async function waitForReadyAppWindow(
  electronApp: Awaited<ReturnType<typeof electron.launch>>,
  timeoutMs: number
): Promise<Page> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const windows = electronApp.windows();
    for (const page of windows) {
      try {
        const isReady = await page.evaluate(() => {
          const root = document.getElementById('root');
          if (!root) {
            return false;
          }
          const chatInput = document.querySelector('[data-testid="chat-input"]');
          return !!chatInput;
        });
        if (isReady) {
          return page;
        }
      } catch {
        // Window may not be ready for evaluation yet.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }

  const urls = electronApp.windows().map((w) => w.url() || '<empty>');
  throw new Error(`No app-ready window found within ${timeoutMs}ms. Window URLs: ${urls.join(', ')}`);
}

function resolvePackagedExecutable(appRoot: string): string {
  if (process.env.GOOSE_PACKAGED_EXECUTABLE) {
    return process.env.GOOSE_PACKAGED_EXECUTABLE;
  }

  if (process.platform === 'darwin') {
    return join(appRoot, 'out', 'Goose-darwin-arm64', 'Goose.app', 'Contents', 'MacOS', 'Goose');
  }

  if (process.platform === 'win32') {
    return join(appRoot, 'out', 'Goose-win32-x64', 'Goose.exe');
  }

  return join(appRoot, 'out', 'goose-linux-x64', 'goose');
}

function createIsolatedGoosePathRoot(): string {
  const tempDir = fs.mkdtempSync(join(os.tmpdir(), 'goose-test-'));
  const configDir = join(tempDir, 'config');
  fs.mkdirSync(configDir, { recursive: true });
  fs.mkdirSync(join(tempDir, 'data'), { recursive: true });
  fs.mkdirSync(join(tempDir, 'state'), { recursive: true });
  fs.writeFileSync(
    join(configDir, 'config.yaml'),
    'GOOSE_PROVIDER: databricks\nGOOSE_MODEL: databricks-claude-haiku-4-5\nGOOSE_TELEMETRY_ENABLED: false\nDATABRICKS_HOST: https://block-lakehouse-production.cloud.databricks.com/\n'
  );
  return tempDir;
}

function buildLaunchOptions(
  executablePath: string,
  tempDir: string,
  videoDir?: string
): Parameters<typeof electron.launch>[0] {
  const launchOptions: Parameters<typeof electron.launch>[0] = {
    executablePath,
    args: [],
    timeout: 60000,
    env: {
      ...process.env,
      GOOSE_ALLOWLIST_BYPASS: 'true',
      GOOSE_PATH_ROOT: tempDir,
      RUST_LOG: 'info',
    },
  };

  if (videoDir) {
    fs.mkdirSync(videoDir, { recursive: true });
    launchOptions.recordVideo = {
      dir: videoDir,
      size: { width: 1280, height: 720 },
    };
  }

  return launchOptions;
}

function attachAppDebugLogs(electronApp: Awaited<ReturnType<typeof electron.launch>>) {
  const appProcess = electronApp.process();
  appProcess?.stdout?.on('data', (data) => {
    debugLog(`Electron stdout: ${data.toString()}`);
  });
  appProcess?.stderr?.on('data', (data) => {
    debugLog(`Electron stderr: ${data.toString()}`);
  });
}

function attachPageDebugLogs(page: Page) {
  page.on('console', (msg) => {
    debugLog(`Renderer console [${msg.type()}]: ${msg.text()}`);
  });
  page.on('pageerror', (err) => {
    debugLog(`Renderer pageerror: ${err.message}`);
  });
  page.on('crash', () => {
    debugLog('Renderer crash event');
  });
  page.on('close', () => {
    debugLog('Renderer page close event');
  });
}

async function closeWithFallback(electronApp: Awaited<ReturnType<typeof electron.launch>>) {
  const appPid = electronApp.process()?.pid;
  debugLog(`Shutting down Electron app${appPid ? ` (pid=${appPid})` : ''}`);

  const closeError = await electronApp.close().then(() => null).catch((error) => error);
  if (!closeError || !appPid) {
    return;
  }

  debugLog(`electronApp.close() failed: ${String(closeError)}`);
  try {
    if (process.platform === 'win32') {
      await execAsync(`taskkill /F /T /PID ${appPid}`);
    } else {
      try {
        process.kill(appPid, 'SIGTERM');
        await new Promise((resolve) => setTimeout(resolve, 1000));
      } catch {
        // Process may already be gone.
      }
      try {
        process.kill(appPid, 'SIGKILL');
      } catch {
        // Process may already be gone.
      }
    }
    debugLog(`Applied hard-kill fallback for Electron pid=${appPid}`);
  } catch (killError) {
    debugLog(`Hard-kill fallback failed: ${String(killError)}`);
  }
}

export { expect } from '@playwright/test';
