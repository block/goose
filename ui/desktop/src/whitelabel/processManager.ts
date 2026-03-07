/**
 * Manages sidecar processes defined in whitelabel.yaml.
 * Runs in the Electron main process.
 */
import { spawn, ChildProcess } from 'child_process';
import * as net from 'node:net';
import * as path from 'node:path';
import log from '../utils/logger';
import type { WhiteLabelProcess } from './types';

interface ManagedProcess {
  config: WhiteLabelProcess;
  child: ChildProcess | null;
  stopped: boolean;
}

const managedProcesses: ManagedProcess[] = [];

function waitForPort(port: number, timeoutMs: number): Promise<void> {
  return new Promise((resolve, reject) => {
    const start = Date.now();

    const tryConnect = () => {
      if (Date.now() - start > timeoutMs) {
        reject(new Error(`Timed out waiting for port ${port} after ${timeoutMs}ms`));
        return;
      }

      const socket = net.createConnection({ port, host: '127.0.0.1' }, () => {
        socket.destroy();
        resolve();
      });

      socket.on('error', () => {
        socket.destroy();
        setTimeout(tryConnect, 200);
      });
    };

    tryConnect();
  });
}

/**
 * Call a URL and set the JSON response (string→string map) as process.env vars.
 * Used to inject credentials from a sidecar's auth flow into the agent's environment.
 */
async function fetchAndSetEnv(url: string, timeoutMs: number): Promise<void> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const resp = await fetch(url, {
      method: 'POST',
      signal: controller.signal,
    });

    if (!resp.ok) {
      const body = await resp.text();
      throw new Error(`HTTP ${resp.status}: ${body}`);
    }

    const envVars: Record<string, string> = await resp.json();

    for (const [key, value] of Object.entries(envVars)) {
      if (typeof value === 'string') {
        process.env[key] = value;
        log.info(`[WhiteLabel] Set env: ${key}=${value.substring(0, 10)}...`);
      }
    }
  } finally {
    clearTimeout(timer);
  }
}

function startProcess(managed: ManagedProcess, resourcesPath: string): void {
  const { config } = managed;
  const cwd = config.cwd ? path.resolve(resourcesPath, config.cwd) : resourcesPath;

  log.info(`[WhiteLabel] Starting process: ${config.name} (${config.command})`);

  const child = spawn(config.command, config.args || [], {
    cwd,
    env: { ...process.env, ...(config.env || {}) },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  child.stdout?.on('data', (data) => {
    log.info(`[${config.name}] ${data.toString().trim()}`);
  });

  child.stderr?.on('data', (data) => {
    log.error(`[${config.name}] ${data.toString().trim()}`);
  });

  child.on('exit', (code, signal) => {
    log.info(`[WhiteLabel] Process ${config.name} exited (code=${code}, signal=${signal})`);
    managed.child = null;

    if (!managed.stopped && config.restartOnCrash) {
      log.info(`[WhiteLabel] Restarting ${config.name} in 2s...`);
      setTimeout(() => {
        if (!managed.stopped) {
          startProcess(managed, resourcesPath);
        }
      }, 2000);
    }
  });

  managed.child = child;
}

export async function startWhiteLabelProcesses(
  processConfigs: WhiteLabelProcess[],
  resourcesPath: string
): Promise<void> {
  for (const config of processConfigs) {
    const managed: ManagedProcess = { config, child: null, stopped: false };
    managedProcesses.push(managed);

    startProcess(managed, resourcesPath);

    // Wait for the process to be ready on its port
    if (config.waitForPort) {
      const timeout = config.waitTimeoutMs || 10000;
      try {
        await waitForPort(config.waitForPort, timeout);
        log.info(`[WhiteLabel] Process ${config.name} is ready on port ${config.waitForPort}`);
      } catch (err) {
        log.error(`[WhiteLabel] Process ${config.name} failed to become ready: ${err}`);
        continue; // skip envFromUrl if port never came up
      }
    }

    // Fetch env vars from the process's bootstrap endpoint
    if (config.envFromUrl) {
      const envTimeout = config.envFromUrlTimeoutMs || 120000;
      try {
        log.info(`[WhiteLabel] Fetching env from ${config.envFromUrl}...`);
        await fetchAndSetEnv(config.envFromUrl, envTimeout);
        log.info(`[WhiteLabel] Process ${config.name} bootstrap complete`);
      } catch (err) {
        log.error(`[WhiteLabel] Process ${config.name} envFromUrl failed: ${err}`);
      }
    }
  }
}

export function stopAllWhiteLabelProcesses(): void {
  for (const managed of managedProcesses) {
    managed.stopped = true;
    if (managed.child) {
      log.info(`[WhiteLabel] Stopping process: ${managed.config.name}`);
      managed.child.kill('SIGTERM');
      managed.child = null;
    }
  }
  managedProcesses.length = 0;
}
