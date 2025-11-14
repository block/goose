import log from './logger';

export type TunnelInfo =
  | { state: 'idle' }
  | { state: 'starting' }
  | { state: 'running'; url: string; hostname: string; secret: string }
  | { state: 'error'; error: string };

/**
 * Start the tunnel via Rust API and remember it
 */
export async function startTunnel(baseUrl: string, serverSecret: string): Promise<TunnelInfo> {
  log.info(`Starting tunnel via Rust API at ${baseUrl}`);

  const response = await fetch(`${baseUrl}/api/tunnel/start`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': serverSecret,
    },
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Failed to start tunnel: ${response.statusText} - ${errorText}`);
  }

  const info: TunnelInfo = await response.json();

  log.info('Tunnel started successfully:', info);
  return info;
}

/**
 * Stop the tunnel via Rust API, and remember it
 */
export async function stopTunnel(baseUrl: string, secret: string): Promise<void> {
  log.info('Stopping tunnel via Rust API');

  try {
    const response = await fetch(`${baseUrl}/api/tunnel/stop`, {
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
 * Get tunnel info from Rust API
 * Note: Rust backend is the source of truth
 */
export async function syncTunnelStatus(baseUrl: string, secret: string): Promise<TunnelInfo> {
  const response = await fetch(`${baseUrl}/api/tunnel/status`, {
    headers: {
      'X-Secret-Key': secret,
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to get tunnel status: ${response.statusText}`);
  }

  return await response.json();
}
