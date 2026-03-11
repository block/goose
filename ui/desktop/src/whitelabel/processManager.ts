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

/**
 * Resolves when all whitelabel processes (daemons + auth flows) are ready.
 * Callers that need whitelabel env vars (e.g. startGoosed) should await this.
 */
let processesReady: Promise<void> = Promise.resolve();
let resolveProcessesReady: () => void;

export function waitForWhiteLabelProcesses(): Promise<void> {
  return processesReady;
}

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
 * Run a short-lived command, collect its stdout, parse as JSON, and map
 * fields to process.env according to the provided mapping.
 */
function runAndCaptureEnv(config: WhiteLabelProcess, resourcesPath: string): Promise<void> {
  const mapping = config.envFromOutput!;
  const timeoutMs = config.envFromOutputTimeoutMs || 120000;
  const cwd = config.cwd ? path.resolve(resourcesPath, config.cwd) : resourcesPath;

  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`[${config.name}] timed out after ${timeoutMs}ms`));
    }, timeoutMs);

    const child = spawn(config.command, config.args || [], {
      cwd,
      env: { ...process.env, ...(config.env || {}) },
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';

    child.stdout?.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr?.on('data', (data) => {
      log.info(`[${config.name}] ${data.toString().trim()}`);
    });

    child.on('error', (err) => {
      clearTimeout(timer);
      reject(new Error(`[${config.name}] failed to spawn: ${err.message}`));
    });

    child.on('exit', (code) => {
      clearTimeout(timer);

      if (code !== 0) {
        reject(new Error(`[${config.name}] exited with code ${code}`));
        return;
      }

      try {
        const json = JSON.parse(stdout.trim());
        for (const [jsonField, envVar] of Object.entries(mapping)) {
          const value = json[jsonField];
          if (value !== undefined && value !== null) {
            process.env[envVar] = String(value);
            log.info(`[WhiteLabel] Set env: ${envVar}=${String(value).substring(0, 10)}...`);
          }
        }
        resolve();
      } catch (err) {
        reject(new Error(`[${config.name}] failed to parse stdout as JSON: ${err}`));
      }
    });
  });
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
  // Create a gate that other code (e.g. startGoosed) can await.
  // This resolves only after ALL processes (daemons + auth flows) are ready.
  processesReady = new Promise<void>((resolve) => {
    resolveProcessesReady = resolve;
  });

  // Add the bin directory to PATH so bundled tools (square, managerbot-server, etc.)
  // are available to sidecar processes and the agent's shell commands.
  const binDir = path.join(resourcesPath, 'src', 'bin');
  const pathKey = process.platform === 'win32' ? 'Path' : 'PATH';
  const currentPath = process.env[pathKey] || '';
  if (!currentPath.includes(binDir)) {
    process.env[pathKey] = `${binDir}${path.delimiter}${currentPath}`;
    log.info(`[WhiteLabel] Added ${binDir} to PATH`);
  }

  // Collect deferred (envFromOutput) processes to run after daemons are up.
  const deferred: WhiteLabelProcess[] = [];

  for (const config of processConfigs) {
    if (config.envFromOutput) {
      deferred.push(config);
      continue;
    }

    // Long-running daemon process — start and wait for port before continuing.
    const managed: ManagedProcess = { config, child: null, stopped: false };
    managedProcesses.push(managed);

    startProcess(managed, resourcesPath);

    if (config.waitForPort) {
      const timeout = config.waitTimeoutMs || 10000;
      try {
        await waitForPort(config.waitForPort, timeout);
        log.info(`[WhiteLabel] Process ${config.name} is ready on port ${config.waitForPort}`);
      } catch (err) {
        log.error(`[WhiteLabel] Process ${config.name} failed to become ready: ${err}`);
      }
    }
  }

  // Kick off auth/env-capture processes in the background.
  // They don't block appMain (so the window can open), but they DO block
  // goosed from spawning via waitForWhiteLabelProcesses().
  if (deferred.length > 0) {
    const runDeferred = async () => {
      for (const config of deferred) {
        try {
          log.info(`[WhiteLabel] Running ${config.name} to capture env...`);
          await runAndCaptureEnv(config, resourcesPath);
          log.info(`[WhiteLabel] ${config.name} complete`);
        } catch (err) {
          log.error(`[WhiteLabel] ${config.name} failed: ${err}`);
        }
      }
      resolveProcessesReady();
    };
    runDeferred();
  } else {
    resolveProcessesReady();
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
