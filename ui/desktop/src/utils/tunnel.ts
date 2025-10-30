import { spawn, ChildProcess } from 'child_process';
import { app, App } from 'electron';
import fs from 'fs';
import path from 'path';
import os from 'os';
import * as crypto from 'crypto';
import { Buffer } from 'node:buffer';
import log from './logger';
import { loadSettings, saveSettings } from './settings';
import { getBinaryPath } from './pathUtils';
import { findAvailablePort, checkServerStatus } from '../goosed';
import { createClient, createConfig } from '../api/client';
import { startLapstoneTunnel, stopLapstoneTunnel } from './lapstone-tunnel';

export type TunnelMode = 'lapstone' | 'tailscale';

function getTunnelMode(): TunnelMode {
  const settings = loadSettings();
  return settings.tunnelMode || 'lapstone';
}

export function setTunnelMode(mode: TunnelMode): void {
  const settings = loadSettings();
  settings.tunnelMode = mode;
  saveSettings(settings);
}

export interface TunnelInfo {
  url: string;
  ipv4: string;
  ipv6: string;
  hostname: string;
  secret: string;
  port: number;
  pids: {
    goosed: number;
    tailscale_serve: number;
  };
}

export type TunnelState = 'idle' | 'starting' | 'running' | 'error';

let tunnelProcess: ChildProcess | null = null;
let goosedProcess: ChildProcess | null = null;
let currentTunnelInfo: TunnelInfo | null = null;
let currentState: TunnelState = 'idle';
let outputFilePath: string | null = null;

function generateSecret(): string {
  return crypto.randomBytes(32).toString('hex');
}

function getTunnelSecret(): string {
  const settings = loadSettings();
  if (settings.tunnelSecret) {
    return settings.tunnelSecret;
  }

  const secret = generateSecret();
  settings.tunnelSecret = secret;
  saveSettings(settings);
  return secret;
}

async function waitForOutputFile(
  filePath: string,
  timeoutMs: number = 120000
): Promise<TunnelInfo> {
  const startTime = Date.now();
  const pollInterval = 500;

  return new Promise((resolve, reject) => {
    const checkFile = () => {
      if (Date.now() - startTime > timeoutMs) {
        reject(new Error('Timeout waiting for tunnel to start'));
        return;
      }

      if (fs.existsSync(filePath)) {
        try {
          const data = fs.readFileSync(filePath, 'utf8');
          if (!data || data.trim().length === 0) {
            setTimeout(checkFile, pollInterval);
            return;
          }
          const tunnelInfo: TunnelInfo = JSON.parse(data);
          if (tunnelInfo.url && tunnelInfo.secret && tunnelInfo.port) {
            resolve(tunnelInfo);
          } else {
            setTimeout(checkFile, pollInterval);
          }
        } catch (error) {
          // File might still be being written
          if (error instanceof SyntaxError) {
            setTimeout(checkFile, pollInterval);
          } else {
            log.error('Error parsing tunnel output file:', error);
            reject(error);
          }
        }
      } else {
        setTimeout(checkFile, pollInterval);
      }
    };

    checkFile();
  });
}

export async function startTunnel(): Promise<TunnelInfo> {
  if (currentState === 'running' || currentState === 'starting') {
    throw new Error('Tunnel is already running or starting');
  }

  currentState = 'starting';

  try {
    let scriptPath: string;
    if (app.isPackaged) {
      scriptPath = path.join(process.resourcesPath, 'bin', 'tailscale-tunnel.sh');
    } else {
      scriptPath = path.resolve(app.getAppPath(), 'src', 'bin', 'tailscale-tunnel.sh');
    }

    if (!fs.existsSync(scriptPath)) {
      throw new Error(`Tunnel script not found at: ${scriptPath}`);
    }

    const goosedPath = getBinaryPath(app, 'goosed');
    if (!fs.existsSync(goosedPath)) {
      throw new Error(`Goosed binary not found at: ${goosedPath}`);
    }

    const port = await findAvailablePort();
    const secret = getTunnelSecret();
    const homeDir = os.homedir();
    const isWindows = process.platform === 'win32';

    log.info(`Starting goosed on port ${port} in home directory ${homeDir}`);

    const goosedEnv = {
      ...process.env,
      HOME: homeDir,
      USERPROFILE: homeDir,
      APPDATA: process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming'),
      LOCALAPPDATA: process.env.LOCALAPPDATA || path.join(homeDir, 'AppData', 'Local'),
      PATH: `${path.dirname(goosedPath)}${path.delimiter}${process.env.PATH || ''}`,
      GOOSE_PORT: String(port),
      GOOSE_SERVER__SECRET_KEY: secret,
    };

    goosedProcess = spawn(goosedPath, ['agent'], {
      cwd: homeDir,
      env: goosedEnv,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      detached: isWindows,
      shell: false,
    });

    goosedProcess.stdout?.on('data', (data: Buffer) => {
      log.info(`goosed stdout: ${data.toString()}`);
    });

    goosedProcess.stderr?.on('data', (data: Buffer) => {
      log.error(`goosed stderr: ${data.toString()}`);
    });

    goosedProcess.on('close', (code: number | null) => {
      log.info(`goosed process exited with code ${code}`);
      if (currentState === 'running') {
        // If goosed dies while tunnel is running, stop everything
        stopTunnel();
      }
    });

    goosedProcess.on('error', (err: Error) => {
      log.error('Failed to start goosed:', err);
      currentState = 'error';
      throw err;
    });

    log.info('Waiting for goosed to be ready...');
    const client = createClient(
      createConfig({
        baseUrl: `http://127.0.0.1:${port}`,
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': secret,
        },
      })
    );

    const serverReady = await checkServerStatus(client);
    if (!serverReady) {
      throw new Error('Goosed server failed to start in time');
    }

    const tunnelMode = getTunnelMode();
    log.info(`Goosed is ready, starting tunnel (mode: ${tunnelMode})...`);

    if (tunnelMode === 'lapstone') {
      currentTunnelInfo = startLapstoneTunnel(port, secret, goosedProcess.pid || 0);
      currentState = 'running';
      log.info('Lapstone tunnel started successfully:', currentTunnelInfo);

      const settings = loadSettings();
      settings.tunnelAutoStart = true;
      saveSettings(settings);

      return currentTunnelInfo;
    }

    log.info('Starting Tailscale tunnel...');

    const timestamp = Date.now();
    outputFilePath = path.join(app.getPath('temp'), `goose-tunnel-${timestamp}.json`);

    log.info(`Starting Tailscale tunnel: ${scriptPath} ${port} ${secret} ${outputFilePath}`);

    tunnelProcess = spawn(scriptPath, [String(port), secret, outputFilePath], {
      detached: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    tunnelProcess.stdout?.on('data', (data: Buffer) => {
      log.info(`Tunnel stdout: ${data.toString()}`);
    });

    tunnelProcess.stderr?.on('data', (data: Buffer) => {
      log.error(`Tunnel stderr: ${data.toString()}`);
    });

    tunnelProcess.on('close', (code: number | null) => {
      log.info(`Tunnel process exited with code ${code}`);
      if (currentState === 'running') {
        currentState = 'idle';
        currentTunnelInfo = null;
        if (goosedProcess) {
          goosedProcess.kill();
          goosedProcess = null;
        }
      }
    });

    tunnelProcess.on('error', (err: Error) => {
      log.error('Tunnel process error:', err);
      currentState = 'error';
      throw err;
    });

    currentTunnelInfo = await waitForOutputFile(outputFilePath);
    if (goosedProcess.pid) {
      currentTunnelInfo.pids.goosed = goosedProcess.pid;
    }
    currentState = 'running';

    log.info('Tunnel started successfully:', currentTunnelInfo);

    const settings = loadSettings();
    settings.tunnelAutoStart = true;
    saveSettings(settings);

    return currentTunnelInfo;
  } catch (error) {
    currentState = 'error';
    if (goosedProcess) {
      goosedProcess.kill();
      goosedProcess = null;
    }
    log.error('Failed to start tunnel:', error);
    throw error;
  }
}

export function stopTunnel(clearAutoStart: boolean = true): void {
  stopLapstoneTunnel();

  if (tunnelProcess) {
    log.info('Stopping tunnel process');
    tunnelProcess.kill('SIGTERM');
    tunnelProcess = null;
  }

  if (goosedProcess) {
    log.info('Stopping goosed process');
    const isWindows = process.platform === 'win32';

    try {
      if (isWindows && goosedProcess.pid) {
        // On Windows, use taskkill for cleaner shutdown
        spawn('taskkill', ['/pid', goosedProcess.pid.toString(), '/T', '/F'], { shell: false });
      } else {
        goosedProcess.kill();
      }
    } catch (error) {
      log.error('Error while terminating goosed process:', error);
    }

    goosedProcess = null;
  }

  currentState = 'idle';
  currentTunnelInfo = null;

  // Only clear auto-start setting when manually stopping (not on app quit)
  if (clearAutoStart) {
    const settings = loadSettings();
    settings.tunnelAutoStart = false;
    saveSettings(settings);
  }

  if (outputFilePath && fs.existsSync(outputFilePath)) {
    try {
      fs.unlinkSync(outputFilePath);
    } catch (error) {
      log.error('Error cleaning up output file:', error);
    }
  }
  outputFilePath = null;
}

export function getTunnelStatus(): { state: TunnelState; info: TunnelInfo | null } {
  return {
    state: currentState,
    info: currentTunnelInfo,
  };
}

export function setupTunnelCleanup(electronApp: App): void {
  electronApp.on('will-quit', () => {
    log.info('App quitting, stopping tunnel if running');
    stopTunnel(false); // Don't clear auto-start flag on quit
  });
}

export async function autoStartTunnel(): Promise<void> {
  const settings = loadSettings();
  if (settings.tunnelAutoStart) {
    log.info('Auto-starting tunnel from previous session');
    try {
      await startTunnel();
    } catch (error) {
      log.error('Failed to auto-start tunnel:', error);
    }
  }
}
