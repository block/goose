import { spawn, ChildProcess } from 'child_process';
import { app, App } from 'electron';
import fs from 'fs';
import path from 'path';
import { Buffer } from 'node:buffer';
import log from './logger';
import { loadSettings, saveSettings } from './settings';
import { getBinaryPath } from './pathUtils';
import { findAvailablePort } from '../goosed';

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
let currentTunnelInfo: TunnelInfo | null = null;
let currentState: TunnelState = 'idle';
let outputFilePath: string | null = null;

// Generate a random secret for tunnel authentication
function generateSecret(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let secret = '';
  for (let i = 0; i < 32; i++) {
    secret += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return secret;
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
    // Get the script path - it's in src/bin (same as uvx, jbang, etc.)
    let scriptPath: string;
    if (app.isPackaged) {
      scriptPath = path.join(process.resourcesPath, 'src', 'bin', 'tailscale-tunnel.sh');
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

    // Create temp output file path
    const timestamp = Date.now();
    outputFilePath = path.join(app.getPath('temp'), `goose-tunnel-${timestamp}.json`);

    log.info(`Starting tunnel: ${scriptPath} ${goosedPath} ${port} [secret] ${outputFilePath}`);

    // Spawn the tunnel script
    tunnelProcess = spawn(scriptPath, [goosedPath, String(port), secret, outputFilePath], {
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
      }
    });

    tunnelProcess.on('error', (err: Error) => {
      log.error('Tunnel process error:', err);
      currentState = 'error';
      throw err;
    });

    // Wait for the output file to be written
    currentTunnelInfo = await waitForOutputFile(outputFilePath);
    currentState = 'running';

    log.info('Tunnel started successfully:', currentTunnelInfo);
    return currentTunnelInfo;
  } catch (error) {
    currentState = 'error';
    log.error('Failed to start tunnel:', error);
    throw error;
  }
}

export function stopTunnel(): void {
  if (tunnelProcess) {
    log.info('Stopping tunnel process');
    tunnelProcess.kill('SIGTERM');
    tunnelProcess = null;
  }

  currentState = 'idle';
  currentTunnelInfo = null;

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
    stopTunnel();
  });
}
