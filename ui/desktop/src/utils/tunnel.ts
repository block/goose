import { App } from 'electron';
import log from './logger';
import { loadSettings, saveSettings } from './settings';

export type TunnelMode = 'lapstone' | 'tailscale';

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

let currentTunnelInfo: TunnelInfo | null = null;
let currentState: TunnelState = 'idle';

export async function startTunnel(goosedPort: number, serverSecret: string): Promise<TunnelInfo> {
  if (currentState === 'running' || currentState === 'starting') {
    throw new Error('Tunnel is already running or starting');
  }

  currentState = 'starting';

  try {
    log.info(`Starting Rust tunnel via API on port ${goosedPort}`);

    // Call the Rust tunnel API to start
    const response = await fetch(`http://127.0.0.1:${goosedPort}/api/tunnel/start`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': serverSecret,
      },
      body: JSON.stringify({ port: goosedPort }),
    });

    if (!response.ok) {
      throw new Error(`Failed to start tunnel: ${response.statusText}`);
    }

    const data = await response.json();
    currentTunnelInfo = data.info;
    currentState = 'running';

    log.info('Rust tunnel started successfully:', currentTunnelInfo);

    const settings = loadSettings();
    settings.tunnelAutoStart = true;
    saveSettings(settings);

    if (!currentTunnelInfo) {
      throw new Error('Tunnel started but no info returned');
    }

    return currentTunnelInfo;
  } catch (error) {
    currentState = 'error';
    log.error('Failed to start tunnel:', error);
    throw error;
  }
}

export async function stopTunnel(
  port: number,
  secret: string,
  clearAutoStart: boolean = true
): Promise<void> {
  try {
    log.info('Stopping Rust tunnel via API');

    // Call the Rust tunnel API to stop
    const response = await fetch(`http://127.0.0.1:${port}/api/tunnel/stop`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': secret,
      },
    });

    if (!response.ok) {
      log.error(`Failed to stop tunnel: ${response.statusText}`);
    }
  } catch (error) {
    log.error('Error stopping tunnel:', error);
  }

  currentState = 'idle';
  currentTunnelInfo = null;

  // Only clear auto-start setting when manually stopping (not on app quit)
  if (clearAutoStart) {
    const settings = loadSettings();
    settings.tunnelAutoStart = false;
    saveSettings(settings);
  }
}

export function getTunnelStatus(): { state: TunnelState; info: TunnelInfo | null } {
  return {
    state: currentState,
    info: currentTunnelInfo,
  };
}

export async function syncTunnelStatus(
  port: number,
  secret: string
): Promise<{ state: TunnelState; info: TunnelInfo | null }> {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/api/tunnel/status`, {
      headers: {
        'X-Secret-Key': secret,
      },
    });

    if (response.ok) {
      const status = await response.json();

      // Update local state to match server
      if (status.state === 'running' && status.info) {
        currentState = 'running';
        currentTunnelInfo = status.info;
      } else if (status.state === 'idle') {
        currentState = 'idle';
        currentTunnelInfo = null;
      }

      return {
        state: currentState,
        info: currentTunnelInfo,
      };
    }
  } catch (error) {
    log.error('Failed to sync tunnel status:', error);
  }

  // If we can't reach the server, return current state
  return {
    state: currentState,
    info: currentTunnelInfo,
  };
}

export function setupTunnelCleanup(electronApp: App, port: number, secret: string): void {
  electronApp.on('will-quit', () => {
    log.info('App quitting, stopping tunnel if running');
    stopTunnel(port, secret, false); // Don't clear auto-start flag on quit
  });
}

export async function autoStartTunnel(port: number, secret: string): Promise<void> {
  const settings = loadSettings();

  // First, check the actual server status
  try {
    const response = await fetch(`http://127.0.0.1:${port}/api/tunnel/status`, {
      headers: {
        'X-Secret-Key': secret,
      },
    });

    if (response.ok) {
      const status = await response.json();

      // Sync our local state with server state
      if (status.state === 'running' && status.info) {
        currentState = 'running';
        currentTunnelInfo = status.info;
        log.info('Tunnel already running on server, synced state');
        return;
      }
    }
  } catch (error) {
    log.error('Failed to check tunnel status:', error);
  }

  // If tunnelAutoStart is enabled and tunnel isn't running, start it
  if (settings.tunnelAutoStart && currentState === 'idle') {
    log.info('Auto-starting tunnel from previous session');
    try {
      await startTunnel(port, secret);
    } catch (error) {
      log.error('Failed to auto-start tunnel:', error);
    }
  }
}
