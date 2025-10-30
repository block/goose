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

// Get tunnel mode from settings (default: lapstone)
function getTunnelMode(): TunnelMode {
  const settings = loadSettings();
  return settings.tunnelMode || 'lapstone';
}

// Set tunnel mode in settings
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

// Generate a random secret for tunnel authentication (same as main app)
function generateSecret(): string {
  return crypto.randomBytes(32).toString('hex');
}

// Get or create tunnel secret
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

// Poll for output file to be created
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
          // Check if file has content and is not empty
          if (!data || data.trim().length === 0) {
            setTimeout(checkFile, pollInterval);
            return;
          }
          const tunnelInfo: TunnelInfo = JSON.parse(data);
          // Verify we got a valid tunnel info object with required fields
          if (tunnelInfo.url && tunnelInfo.secret && tunnelInfo.port) {
            resolve(tunnelInfo);
          } else {
            // File exists but content is incomplete, retry
            setTimeout(checkFile, pollInterval);
          }
        } catch (error) {
          // If JSON parse fails, file might still be being written, retry
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
    // Get the script path - it's in src/bin (dev) or bin (packaged)
    let scriptPath: string;
    if (app.isPackaged) {
      scriptPath = path.join(process.resourcesPath, 'bin', 'tailscale-tunnel.sh');
    } else {
      scriptPath = path.resolve(app.getAppPath(), 'src', 'bin', 'tailscale-tunnel.sh');
    }

    if (!fs.existsSync(scriptPath)) {
      throw new Error(`Tunnel script not found at: ${scriptPath}`);
    }

    // Get goosed binary path
    const goosedPath = getBinaryPath(app, 'goosed');
    if (!fs.existsSync(goosedPath)) {
      throw new Error(`Goosed binary not found at: ${goosedPath}`);
    }

    // Find available port
    const port = await findAvailablePort();

    // Get or create secret
    const secret = getTunnelSecret();

    // Get user's home directory
    const homeDir = os.homedir();
    const isWindows = process.platform === 'win32';

    // Start goosed first (similar to goosed.ts)
    log.info(`Starting goosed on port ${port} in home directory ${homeDir}`);

    // Set up environment for goosed (consistent with goosed.ts)
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

    // Spawn goosed process
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

    // Wait for goosed to be ready
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

    // Get tunnel mode from settings
    const tunnelMode = getTunnelMode();
    log.info(`Goosed is ready, starting tunnel (mode: ${tunnelMode})...`);

    // Choose tunnel implementation based on mode
    if (tunnelMode === 'lapstone') {
      // Use Lapstone tunnel (default)
      currentTunnelInfo = startLapstoneTunnel(port, secret, goosedProcess.pid || 0);
      currentState = 'running';
      log.info('Lapstone tunnel started successfully:', currentTunnelInfo);

      // Save auto-start setting when tunnel starts
      const settings = loadSettings();
      settings.tunnelAutoStart = true;
      saveSettings(settings);

      return currentTunnelInfo;
    }

    // Use Tailscale tunnel
    log.info('Starting Tailscale tunnel...');

    // Create temp output file path
    const timestamp = Date.now();
    outputFilePath = path.join(app.getPath('temp'), `goose-tunnel-${timestamp}.json`);

    // Now spawn the tailscale script with just the port and output file
    // The script no longer needs to manage goosed
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
        // Also stop goosed when tunnel stops
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

    // Wait for the output file to be written
    currentTunnelInfo = await waitForOutputFile(outputFilePath);
    // Update tunnel info with the actual goosed PID
    if (goosedProcess.pid) {
      currentTunnelInfo.pids.goosed = goosedProcess.pid;
    }
    currentState = 'running';

    log.info('Tunnel started successfully:', currentTunnelInfo);

    // Save auto-start setting when tunnel starts
    const settings = loadSettings();
    settings.tunnelAutoStart = true;
    saveSettings(settings);

    return currentTunnelInfo;
  } catch (error) {
    currentState = 'error';
    // Clean up goosed if we started it but tunnel failed
    if (goosedProcess) {
      goosedProcess.kill();
      goosedProcess = null;
    }
    log.error('Failed to start tunnel:', error);
    throw error;
  }
}

export function stopTunnel(clearAutoStart: boolean = true): void {
  // Stop Lapstone tunnel if active
  stopLapstoneTunnel();

  // Stop the tailscale tunnel process
  if (tunnelProcess) {
    log.info('Stopping tunnel process');
    tunnelProcess.kill('SIGTERM');
    tunnelProcess = null;
  }

  // Stop the goosed process
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

  // Clean up output file
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

// Clean up tunnel on app quit
export function setupTunnelCleanup(electronApp: App): void {
  electronApp.on('will-quit', () => {
    log.info('App quitting, stopping tunnel if running');
    stopTunnel(false); // Don't clear auto-start flag on quit
  });
}

// Auto-start tunnel if it was running when app closed
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
