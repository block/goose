import log from './logger';

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

export interface TunnelStatus {
  state: TunnelState;
  info: TunnelInfo | null;
  auto_start: boolean;
}

/**
 * Start the tunnel via Rust API
 * Note: Rust backend is the source of truth for all tunnel state
 */
export async function startTunnel(goosedPort: number, serverSecret: string): Promise<TunnelInfo> {
  log.info(`Starting tunnel via Rust API on port ${goosedPort}`);

  const response = await fetch(`http://127.0.0.1:${goosedPort}/api/tunnel/start`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': serverSecret,
    },
    body: JSON.stringify({ port: goosedPort }),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Failed to start tunnel: ${response.statusText} - ${errorText}`);
  }

  const data: TunnelStatus = await response.json();

  if (!data.info) {
    throw new Error('Tunnel started but no info returned');
  }

  log.info('Tunnel started successfully:', data.info);
  return data.info;
}

/**
 * Stop the tunnel via Rust API
 * Note: This will also set auto_start = false in the Rust config
 */
export async function stopTunnel(port: number, secret: string): Promise<void> {
  log.info('Stopping tunnel via Rust API');

  try {
    const response = await fetch(`http://127.0.0.1:${port}/api/tunnel/stop`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': secret,
      },
    });

    if (!response.ok) {
      log.error(`Failed to stop tunnel: ${response.statusText}`);
    } else {
      log.info('Tunnel stopped successfully');
    }
  } catch (error) {
    log.error('Error stopping tunnel:', error);
  }
}

/**
 * Get tunnel status from Rust API
 * Note: Rust backend is the source of truth
 */
export async function syncTunnelStatus(port: number, secret: string): Promise<TunnelStatus> {
  const response = await fetch(`http://127.0.0.1:${port}/api/tunnel/status`, {
    headers: {
      'X-Secret-Key': secret,
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to get tunnel status: ${response.statusText}`);
  }

  return await response.json();
}
