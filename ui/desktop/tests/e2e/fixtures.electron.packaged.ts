import { test as base, expect, Locator, Page, _electron as electron } from '@playwright/test';
import { promisify } from 'util';
import { exec, execFile } from 'child_process';
import { join } from 'path';
import * as fs from 'fs';
import * as os from 'os';

const execAsync = promisify(exec);
const execFileAsync = promisify(execFile);

type GooseTestFixtures = {
  goosePage: Page;
};

const isDebug = () => process.env.DEBUG_TESTS === '1' || process.env.DEBUG_TESTS === 'true';
const debugLog = (message: string) => {
  if (isDebug()) {
    console.log(message);
  }
};

export const isVideoRecording = () => process.env.PW_ELECTRON_VIDEO === '1';
const VISUAL_DELAY_MS = Number(process.env.E2E_VISUAL_DELAY_MS ?? (isVideoRecording() ? '500' : '0'));
const POST_LOAD_HOLD_MS = Number(process.env.E2E_POST_LOAD_HOLD_MS ?? (isVideoRecording() ? '1000' : '0'));
const VIDEO_TRIM_READY_BUFFER_MS = Number(process.env.E2E_VIDEO_TRIM_READY_BUFFER_MS ?? (isVideoRecording() ? '300' : '0'));

const PAGE_ACTION_METHODS = new Set([
  'click',
  'dblclick',
  'tap',
  'fill',
  'press',
  'check',
  'uncheck',
  'setChecked',
  'selectOption',
  'dragAndDrop',
  'goto',
  'reload'
]);

const PAGE_LOCATOR_METHODS = new Set([
  'locator',
  'getByAltText',
  'getByLabel',
  'getByPlaceholder',
  'getByRole',
  'getByTestId',
  'getByText',
  'getByTitle'
]);

const LOCATOR_ACTION_METHODS = new Set([
  'click',
  'dblclick',
  'tap',
  'fill',
  'press',
  'check',
  'uncheck',
  'setChecked',
  'selectOption',
  'hover',
  'focus',
  'blur'
]);

const LOCATOR_CHAIN_METHODS = new Set([
  'locator',
  'getByAltText',
  'getByLabel',
  'getByPlaceholder',
  'getByRole',
  'getByTestId',
  'getByText',
  'getByTitle'
]);
const VISUAL_DELAY_DECORATED = Symbol('visual-delay-decorated');

async function applyVisualDelay(page: Page): Promise<void> {
  if (VISUAL_DELAY_MS > 0) {
    await page.waitForTimeout(VISUAL_DELAY_MS);
  }
}

function withVisualDelayLocator(locator: Locator, page: Page): Locator {
  const existingMark = (locator as unknown as Record<PropertyKey, unknown>)[VISUAL_DELAY_DECORATED];
  if (existingMark) {
    return locator;
  }

  for (const methodName of LOCATOR_ACTION_METHODS) {
    const original = (locator as unknown as Record<string, unknown>)[methodName];
    if (typeof original === 'function') {
      (locator as unknown as Record<string, unknown>)[methodName] = async (...args: unknown[]) => {
        const resolved = await (original as (...params: unknown[]) => unknown).apply(locator, args);
        await applyVisualDelay(page);
        return resolved;
      };
    }
  }

  for (const methodName of LOCATOR_CHAIN_METHODS) {
    const original = (locator as unknown as Record<string, unknown>)[methodName];
    if (typeof original === 'function') {
      (locator as unknown as Record<string, unknown>)[methodName] = (...args: unknown[]) => {
        const next = (original as (...params: unknown[]) => unknown).apply(locator, args);
        if (next && typeof next === 'object') {
          return withVisualDelayLocator(next as Locator, page);
        }
        return next;
      };
    }
  }

  Object.defineProperty(locator, VISUAL_DELAY_DECORATED, {
    value: true,
    enumerable: false,
    configurable: false,
    writable: false
  });

  return locator;
}

function withVisualDelayPage(page: Page): Page {
  if (VISUAL_DELAY_MS <= 0) {
    return page;
  }

  return new Proxy(page, {
    get(target, prop, receiver) {
      const propName = String(prop);
      const value = Reflect.get(target, prop, receiver);

      if (typeof value !== 'function') {
        return value;
      }

      return (...args: unknown[]) => {
        const result = value.apply(target, args);
        if (PAGE_LOCATOR_METHODS.has(propName) && result && typeof result === 'object') {
          return withVisualDelayLocator(result as Locator, target);
        }
        if (PAGE_ACTION_METHODS.has(propName)) {
          return Promise.resolve(result).then(async (resolved) => {
            await applyVisualDelay(target);
            return resolved;
          });
        }
        return result;
      };
    }
  }) as Page;
}


export const test = base.extend<GooseTestFixtures>({
  goosePage: async ({}, use, testInfo) => {
    let electronApp: Awaited<ReturnType<typeof electron.launch>> | null = null;
    let page: Page | null = null;
    let videoDir: string | undefined;
    let videoTrimStartMs = 0;
    const appRoot = join(__dirname, '../..');
    const executablePath = resolvePackagedExecutable(appRoot);

    if (!fs.existsSync(executablePath)) {
      throw new Error(
        `Packaged app executable not found at ${executablePath}. Build it first (e.g. "cd ui/desktop && npm run package"), or set GOOSE_PACKAGED_EXECUTABLE.`
      );
    }

    const tempDir = createIsolatedGoosePathRoot();

    try {
      videoDir = isVideoRecording() ? testInfo.outputPath('videos') : undefined;
      const launchOptions = buildLaunchOptions(executablePath, tempDir, videoDir);
      debugLog(`Launching packaged Electron for test: ${testInfo.title}`);
      debugLog(`Using packaged executable: ${executablePath}`);

      electronApp = await electron.launch(launchOptions);
      attachAppDebugLogs(electronApp);

      await electronApp.firstWindow({ timeout: 30000 });
      const recordingStartMs = Date.now();
      await waitForRootWindow(electronApp, 60000);
      const rootReadyElapsedMs = Date.now() - recordingStartMs;
      page = await waitForReadyAppWindow(electronApp, 60000);
      await page.waitForLoadState('domcontentloaded', { timeout: 10000 }).catch(() => {});
      debugLog(`Selected app window URL: ${page.url()}`);
      attachPageDebugLogs(page);
      if (isVideoRecording()) {
        await enableCursorHighlight(page);
        page.on('domcontentloaded', () => {
          void enableCursorHighlight(page);
        });
      }
      const overrideTrimStartMs = process.env.E2E_VIDEO_TRIM_START_MS;
      if (overrideTrimStartMs !== undefined) {
        const parsedOverride = Number(overrideTrimStartMs);
        videoTrimStartMs = Number.isFinite(parsedOverride) && parsedOverride > 0 ? parsedOverride : 0;
      } else {
        videoTrimStartMs = Math.max(0, rootReadyElapsedMs - VIDEO_TRIM_READY_BUFFER_MS);
      }

      await use(withVisualDelayPage(page));
    } finally {
      if (electronApp) {
        await closeWithFallback(electronApp);
      }

      if (videoDir && videoTrimStartMs > 0) {
        await trimVideosInDirectory(videoDir, videoTrimStartMs);
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

async function waitForRootWindow(
  electronApp: Awaited<ReturnType<typeof electron.launch>>,
  timeoutMs: number
): Promise<Page> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const windows = electronApp.windows();
    for (const page of windows) {
      try {
        const hasRoot = await page.evaluate(() => !!document.getElementById('root'));
        if (hasRoot) {
          return page;
        }
      } catch {
        // Window may not be ready for evaluation yet.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  const urls = electronApp.windows().map((w) => w.url() || '<empty>');
  throw new Error(`No root-ready window found within ${timeoutMs}ms. Window URLs: ${urls.join(', ')}`);
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

  // Copy OAuth token cache so the app can silently refresh without opening a browser.
  // If the refresh token is expired, the app falls back to browser OAuth as usual.
  const realOAuthDir = join(os.homedir(), '.config', 'goose', 'databricks', 'oauth');
  if (fs.existsSync(realOAuthDir)) {
    const testOAuthDir = join(configDir, 'databricks', 'oauth');
    fs.mkdirSync(testOAuthDir, { recursive: true });
    for (const file of fs.readdirSync(realOAuthDir)) {
      fs.copyFileSync(join(realOAuthDir, file), join(testOAuthDir, file));
    }
  }

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
      GOOSE_DISABLE_KEYRING: '1',
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

async function enableCursorHighlight(page: Page): Promise<void> {
  try {
    await page.evaluate(() => {
      const markerAttr = 'data-test-cursor-highlight';
      if (document.querySelector(`[${markerAttr}]`)) {
        return;
      }

      const style = document.createElement('style');
      style.setAttribute(markerAttr, 'style');
      style.textContent = `
        .test-cursor-overlay {
          position: fixed;
          inset: 0;
          pointer-events: none;
          z-index: 2147483647;
          overflow: hidden;
        }
        .test-cursor-highlight {
          position: absolute;
          width: 22px;
          height: 22px;
          border-radius: 9999px;
          border: 2px solid rgba(255, 255, 255, 0.95);
          background: rgba(239, 68, 68, 0.45);
          box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.2), 0 4px 14px rgba(0, 0, 0, 0.25);
          transform: none;
          transition: width 100ms ease, height 100ms ease, background 100ms ease;
          mix-blend-mode: normal;
          opacity: 0;
        }
        .test-cursor-highlight.clicking {
          width: 28px;
          height: 28px;
          background: rgba(239, 68, 68, 0.65);
        }
        .test-cursor-click-ring {
          position: absolute;
          width: 12px;
          height: 12px;
          border-radius: 9999px;
          border: 2px solid rgba(239, 68, 68, 0.8);
          transform: none;
          animation: test-cursor-ring 420ms ease-out forwards;
        }
        @keyframes test-cursor-ring {
          0% { opacity: 0.95; width: 12px; height: 12px; }
          100% { opacity: 0; width: 54px; height: 54px; }
        }
      `;
      document.head.appendChild(style);

      const overlay = document.createElement('div');
      overlay.className = 'test-cursor-overlay';
      overlay.setAttribute(markerAttr, 'overlay');
      document.body.appendChild(overlay);

      const cursor = document.createElement('div');
      cursor.className = 'test-cursor-highlight';
      cursor.setAttribute(markerAttr, 'cursor');
      overlay.appendChild(cursor);

      const setElementCenter = (element: HTMLElement, x: number, y: number) => {
        const width = element.offsetWidth || Number.parseInt(getComputedStyle(element).width, 10) || 0;
        const height = element.offsetHeight || Number.parseInt(getComputedStyle(element).height, 10) || 0;
        element.style.left = `${x - width / 2}px`;
        element.style.top = `${y - height / 2}px`;
      };

      const moveCursor = (x: number, y: number) => {
        setElementCenter(cursor, x, y);
        cursor.style.opacity = '1';
      };

      const spawnClickRing = (x: number, y: number) => {
        const ring = document.createElement('div');
        ring.className = 'test-cursor-click-ring';
        setElementCenter(ring, x, y);
        ring.setAttribute(markerAttr, 'ring');
        overlay.appendChild(ring);
        window.setTimeout(() => ring.remove(), 500);
      };

      document.addEventListener(
        'mousemove',
        (event) => {
          moveCursor(event.clientX, event.clientY);
        },
        { passive: true }
      );

      document.addEventListener(
        'mousedown',
        (event) => {
          moveCursor(event.clientX, event.clientY);
          cursor.classList.add('clicking');
          spawnClickRing(event.clientX, event.clientY);
        },
        { passive: true }
      );

      document.addEventListener(
        'mouseup',
        () => {
          cursor.classList.remove('clicking');
        },
        { passive: true }
      );
    });
  } catch (error) {
    debugLog(`Failed to enable cursor highlight: ${String(error)}`);
  }
}

async function trimVideosInDirectory(videoDir: string, trimStartMs: number): Promise<void> {
  const requestedTrimSeconds = Math.max(0, trimStartMs) / 1000;
  if (requestedTrimSeconds <= 0 || !fs.existsSync(videoDir)) {
    return;
  }

  const files = fs.readdirSync(videoDir).filter((name) => name.endsWith('.webm'));
  if (files.length === 0) {
    return;
  }

  for (const fileName of files) {
    const sourcePath = join(videoDir, fileName);
    const trimmedPath = join(videoDir, `${fileName}.trimmed.webm`);
    try {
      const durationSeconds = await getVideoDurationSeconds(sourcePath);
      const maxTrimSeconds = Math.max(0, durationSeconds - 0.25);
      const trimSeconds = Math.min(requestedTrimSeconds, maxTrimSeconds);
      if (trimSeconds <= 0) {
        continue;
      }

      await execFileAsync('ffmpeg', [
        '-y',
        '-ss',
        trimSeconds.toFixed(3),
        '-i',
        sourcePath,
        '-c:v',
        'libvpx-vp9',
        '-b:v',
        '0',
        '-crf',
        '32',
        '-an',
        trimmedPath
      ]);
      fs.renameSync(trimmedPath, sourcePath);
    } catch (error) {
      debugLog(`Failed to trim video ${sourcePath}: ${String(error)}`);
      if (fs.existsSync(trimmedPath)) {
        fs.rmSync(trimmedPath, { force: true });
      }
    }
  }
}

async function getVideoDurationSeconds(videoPath: string): Promise<number> {
  try {
    const { stdout } = await execFileAsync('ffprobe', [
      '-v',
      'error',
      '-show_entries',
      'format=duration',
      '-of',
      'default=noprint_wrappers=1:nokey=1',
      videoPath
    ]);
    const duration = Number(stdout.trim());
    return Number.isFinite(duration) && duration > 0 ? duration : 0;
  } catch (error) {
    debugLog(`Failed to probe video duration ${videoPath}: ${String(error)}`);
    return 0;
  }
}

export async function waitForLoadingDone(page: Page, timeout: number): Promise<void> {
  await expect(page.getByTestId('loading-indicator')).toHaveCount(0, { timeout });
  if (POST_LOAD_HOLD_MS > 0) {
    await page.waitForTimeout(POST_LOAD_HOLD_MS);
  }
}

export { expect } from '@playwright/test';
