import { join } from 'path';
import * as fs from 'fs';
import * as os from 'os';
import { Page, _electron as electron } from '@playwright/test';
import { debugLog } from './debug-log';

function resolveDirectElectronExecutable(appRoot: string): string {
  return join(appRoot, 'node_modules', 'electron', 'dist', 'Electron.app', 'Contents', 'MacOS', 'Electron');
}

export function createIsolatedGoosePathRoot(): string {
  const tempDir = fs.mkdtempSync(join(os.tmpdir(), 'goose-test-'));
  const configDir = join(tempDir, 'config');
  fs.mkdirSync(configDir, { recursive: true });
  fs.mkdirSync(join(tempDir, 'data'), { recursive: true });
  fs.mkdirSync(join(tempDir, 'state'), { recursive: true });
  const provider = process.env.GOOSE_PROVIDER || 'anthropic';
  const model = process.env.GOOSE_MODEL || 'claude-haiku-4-5-20251001';
  fs.writeFileSync(
    join(configDir, 'config.yaml'),
    `GOOSE_PROVIDER: ${provider}\nGOOSE_MODEL: ${model}\nGOOSE_TELEMETRY_ENABLED: false\n`
  );

  return tempDir;
}

function validateBuildPrerequisites(appRoot: string, executablePath: string): void {
  if (!fs.existsSync(executablePath)) {
    throw new Error(
      `Electron executable not found at ${executablePath}. Install dependencies in ui/desktop to ensure node_modules/electron exists.`
    );
  }

  const viteMainPath = join(appRoot, '.vite', 'build', 'main.js');
  if (!fs.existsSync(viteMainPath)) {
    throw new Error(
      `Direct Electron mode requires Vite build output at ${viteMainPath}. Run "cd ui/desktop && npm run package" (or another build step that generates .vite/build) first.`
    );
  }
}

function resolveBundledGoosedPath(appPath: string): string {
  return join(appPath, 'Contents', 'Resources', 'bin', 'goosed');
}

function resolveBundledAppAsar(appPath: string): string {
  return join(appPath, 'Contents', 'Resources', 'app.asar');
}

export function isBundledAppMode(): boolean {
  return !!process.env.GOOSE_E2E_APP_PATH;
}

export function buildLaunchOptions(
  tempDir: string,
  videoDir?: string
): Parameters<typeof electron.launch>[0] {
  const appRoot = join(__dirname, '../../..');
  const executablePath = resolveDirectElectronExecutable(appRoot);

  const bundledAppPath = process.env.GOOSE_E2E_APP_PATH;
  let appPath: string;
  const extraEnv: Record<string, string> = {};

  if (bundledAppPath) {
    // Bundled app mode: use dev Electron binary but load bundled app.asar
    const appAsar = resolveBundledAppAsar(bundledAppPath);
    if (!fs.existsSync(appAsar)) {
      throw new Error(`Bundled app.asar not found at ${appAsar}`);
    }
    const goosedPath = resolveBundledGoosedPath(bundledAppPath);
    if (!fs.existsSync(goosedPath)) {
      throw new Error(`Bundled goosed binary not found at ${goosedPath}`);
    }
    appPath = appAsar;
    extraEnv.GOOSED_BINARY = goosedPath;
    debugLog(`Using bundled app.asar: ${appAsar}`);
    debugLog(`Using bundled goosed: ${goosedPath}`);
  } else {
    // Dev mode: use local Vite build output
    appPath = appRoot;
    validateBuildPrerequisites(appRoot, executablePath);
  }

  debugLog(`Using Electron: ${executablePath} (mode=${bundledAppPath ? 'bundled' : 'dev'})`);

  const launchOptions: Parameters<typeof electron.launch>[0] = {
    executablePath,
    args: [appPath],
    timeout: 30000,
    // WARNING: env contains API keys (e.g. ANTHROPIC_API_KEY). Do not log launchOptions.
    env: {
      ...process.env,
      GOOSE_ALLOWLIST_BYPASS: 'true',
      GOOSE_DISABLE_KEYRING: '1',
      GOOSE_PATH_ROOT: tempDir,
      GOOSE_WORKING_DIR: tempDir,
      RUST_LOG: 'info',
      ...extraEnv,
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

export async function waitForRootWindow(
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

export async function closeElectronApp(electronApp: Awaited<ReturnType<typeof electron.launch>>) {
  const appPid = electronApp.process()?.pid;
  debugLog(`Shutting down Electron app${appPid ? ` (pid=${appPid})` : ''}`);

  const closeError = await electronApp.close().then(() => null).catch((error) => error);
  if (!closeError || !appPid) {
    return;
  }

  debugLog(`electronApp.close() failed: ${String(closeError)}`);
  try {
    process.kill(appPid, 'SIGKILL');
    debugLog(`Applied SIGKILL fallback for Electron pid=${appPid}`);
  } catch {
    // Process may already be gone.
  }
}
