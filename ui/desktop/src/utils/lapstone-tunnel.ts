import WebSocket from 'ws';
import * as http from 'http';
import * as crypto from 'crypto';
import * as os from 'os';
import { Buffer } from 'buffer';
import log from './logger';
import { loadSettings, saveSettings } from './settings';
import { TunnelInfo } from './tunnel';

const WORKER_URL =
  process.env.GOOSE_TUNNEL_WORKER_URL ||
  'https://cloudflare-tunnel-proxy.michael-neale.workers.dev';

interface TunnelMessage {
  requestId: string;
  method: string;
  path: string;
  headers?: Record<string, string>;
  body?: string;
}

let ws: WebSocket | null = null;
let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
let pingInterval: ReturnType<typeof setInterval> | null = null;
let healthCheckInterval: ReturnType<typeof setInterval> | null = null;
let networkMonitorInterval: ReturnType<typeof setInterval> | null = null;
let currentPort: number = 0;
let currentAgentId: string = '';
let isRunning: boolean = false;
let lastNetworkSnapshot: NetworkSnapshot | null = null;

// Monitoring configuration
const HEALTH_CHECK_INTERVAL = 30000; // 30 seconds - end-to-end check
const NETWORK_MONITOR_INTERVAL = 10000; // 10 seconds - interface check
const HEALTH_CHECK_TIMEOUT = 5000; // 5 seconds

interface NetworkSnapshot {
  interfaceNames: string[];
  ipAddresses: string[];
  vpnActive: boolean;
}

function generateAgentId(): string {
  return crypto.randomBytes(16).toString('hex');
}

function getAgentId(): string {
  const settings = loadSettings();
  if (settings.tunnelAgentId) {
    return settings.tunnelAgentId;
  }
  const agentId = generateAgentId();
  settings.tunnelAgentId = agentId;
  saveSettings(settings);
  return agentId;
}

async function handleRequest(message: TunnelMessage): Promise<void> {
  const { requestId, method, path, headers, body } = message;

  log.info(`→ ${method} ${path} [${requestId}]`);

  const targetUrl = new URL(path, `http://127.0.0.1:${currentPort}`);

  const options = {
    method,
    headers: headers || {},
    hostname: targetUrl.hostname,
    port: targetUrl.port,
    path: targetUrl.pathname + targetUrl.search,
  };

  const req = http.request(options, (res) => {
    const responseHeaders = res.headers;

    // Check if this is a streaming response (SSE, etc.)
    const isStreamingResponse = responseHeaders['content-type']?.includes('text/event-stream');

    if (isStreamingResponse) {
      // Real-time streaming: send each chunk as it arrives
      log.info(`← ${res.statusCode} ${path} [${requestId}] (streaming)`);
      let isFirstChunk = true;
      let chunkIndex = 0;

      res.on('data', (chunk) => {
        const chunkStr = chunk.toString();

        const response = {
          requestId,
          status: res.statusCode,
          headers: isFirstChunk ? responseHeaders : undefined,
          body: chunkStr,
          chunkIndex: chunkIndex++,
          isStreaming: true,
          isFirstChunk: isFirstChunk,
          isLastChunk: false,
        };

        isFirstChunk = false;
        ws?.send(JSON.stringify(response));
      });

      res.on('end', () => {
        // Send final chunk marker
        const response = {
          requestId,
          status: res.statusCode,
          body: '',
          chunkIndex: chunkIndex,
          isStreaming: true,
          isFirstChunk: false,
          isLastChunk: true,
        };

        ws?.send(JSON.stringify(response));
        log.info(`← ${res.statusCode} ${path} [${requestId}] (complete, ${chunkIndex} chunks)`);
      });
    } else {
      // Regular response: buffer and potentially split if too large
      const chunks: Buffer[] = [];

      res.on('data', (chunk) => chunks.push(chunk));

      res.on('end', () => {
        const responseBody = Buffer.concat(chunks).toString();

        // Check if response is too large for single WebSocket message (1MB limit)
        const MAX_WS_SIZE = 900000; // 900KB to be safe

        if (responseBody.length > MAX_WS_SIZE) {
          // Send in chunks
          const totalChunks = Math.ceil(responseBody.length / MAX_WS_SIZE);
          log.info(
            `← ${res.statusCode} ${path} [${requestId}] (${responseBody.length} bytes, ${totalChunks} chunks)`
          );

          for (let i = 0; i < totalChunks; i++) {
            const start = i * MAX_WS_SIZE;
            const end = Math.min(start + MAX_WS_SIZE, responseBody.length);
            const chunk = responseBody.substring(start, end);

            const response = {
              requestId,
              status: res.statusCode,
              headers: i === 0 ? responseHeaders : undefined, // Only send headers in first chunk
              body: chunk,
              chunkIndex: i,
              totalChunks: totalChunks,
              isChunked: true,
            };

            ws?.send(JSON.stringify(response));
          }
        } else {
          // Send as single message
          const response = {
            requestId,
            status: res.statusCode,
            headers: responseHeaders,
            body: responseBody,
          };

          ws?.send(JSON.stringify(response));
          log.info(`← ${res.statusCode} ${path} [${requestId}]`);
        }
      });
    }
  });

  req.on('error', (err) => {
    log.error(`✗ Request error [${requestId}]:`, err.message);
    const errorResponse = {
      requestId,
      status: 500,
      error: err.message,
    };
    ws?.send(JSON.stringify(errorResponse));
  });

  if (body && method !== 'GET' && method !== 'HEAD') {
    req.write(body);
  }

  req.end();
}

// eslint-disable-next-line no-undef
function detectVPN(interfaces: NodeJS.Dict<os.NetworkInterfaceInfo[]>): boolean {
  const names = Object.keys(interfaces);
  const vpnPatterns = [
    /^utun/, // macOS VPN
    /^tun/, // OpenVPN, WireGuard
    /^tap/, // OpenVPN
    /^wg/, // WireGuard
    /^ppp/, // PPTP, L2TP
    /^ipsec/, // IPSec
  ];

  return names.some((name) => vpnPatterns.some((pattern) => pattern.test(name)));
}

function getNetworkSnapshot(): NetworkSnapshot {
  const interfaces = os.networkInterfaces();

  const active = Object.entries(interfaces).filter(
    ([_, addrs]) => addrs !== undefined && addrs.some((a) => !a.internal)
  );

  return {
    interfaceNames: active.map(([name]) => name).sort(),
    ipAddresses: active
      .flatMap(([_, addrs]) => addrs!.filter((a) => !a.internal).map((a) => a.address))
      .sort(),
    vpnActive: detectVPN(interfaces),
  };
}

function networkChanged(prev: NetworkSnapshot, curr: NetworkSnapshot): boolean {
  // Check if interface list changed
  if (JSON.stringify(prev.interfaceNames) !== JSON.stringify(curr.interfaceNames)) {
    log.info('Network interfaces changed:', {
      before: prev.interfaceNames,
      after: curr.interfaceNames,
    });
    return true;
  }

  // Check if IPs changed
  if (JSON.stringify(prev.ipAddresses) !== JSON.stringify(curr.ipAddresses)) {
    log.info('IP addresses changed:', {
      before: prev.ipAddresses,
      after: curr.ipAddresses,
    });
    return true;
  }

  // Check if VPN status changed
  if (prev.vpnActive !== curr.vpnActive) {
    log.info(`VPN ${curr.vpnActive ? 'connected' : 'disconnected'}`);
    return true;
  }

  return false;
}

function startNetworkMonitoring(): void {
  if (networkMonitorInterval) clearInterval(networkMonitorInterval);

  lastNetworkSnapshot = getNetworkSnapshot();
  log.info('Network monitoring started:', lastNetworkSnapshot);

  networkMonitorInterval = setInterval(() => {
    if (!isRunning || !lastNetworkSnapshot) return;

    const currentSnapshot = getNetworkSnapshot();

    if (networkChanged(lastNetworkSnapshot, currentSnapshot)) {
      log.warn('Network change detected, restarting tunnel immediately');
      lastNetworkSnapshot = currentSnapshot;
      restartTunnel();
    }
  }, NETWORK_MONITOR_INTERVAL);
}

async function checkTunnelHealth(): Promise<boolean> {
  const publicUrl = `${WORKER_URL}/tunnel/${currentAgentId}`;

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), HEALTH_CHECK_TIMEOUT);

    const response = await fetch(publicUrl, {
      method: 'GET',
      signal: controller.signal,
    });

    clearTimeout(timeout);

    // Any response < 500 is considered healthy (200, 400, 401, 404, etc.)
    if (response.status >= 500) {
      log.warn(`Tunnel health check failed: HTTP ${response.status}, restarting immediately`);
      return false;
    }

    log.info(`Tunnel health check passed: HTTP ${response.status}`);
    return true;
  } catch (err) {
    // Network errors, timeouts, etc.
    const message = err instanceof Error ? err.message : 'Unknown error';
    log.warn(`Tunnel health check error: ${message}, restarting immediately`);
    return false;
  }
}

function startHealthChecks(): void {
  if (healthCheckInterval) clearInterval(healthCheckInterval);

  healthCheckInterval = setInterval(async () => {
    if (!isRunning) return;

    const healthy = await checkTunnelHealth();

    if (!healthy) {
      restartTunnel();
    }
  }, HEALTH_CHECK_INTERVAL);

  // Run initial health check after a short delay (give WS time to fully connect)
  setTimeout(async () => {
    if (isRunning) {
      await checkTunnelHealth();
    }
  }, 5000);
}

function restartTunnel(): void {
  log.info('Restarting tunnel connection...');

  // Close existing connection
  if (ws) {
    ws.close();
    ws = null;
  }

  // Clear intervals/timeouts (monitoring intervals will continue)
  if (pingInterval) clearInterval(pingInterval);
  if (reconnectTimeout) clearTimeout(reconnectTimeout);

  // Reconnect with existing port and agent ID
  if (isRunning) {
    setTimeout(() => connect(currentPort, currentAgentId), 100);
  }
}

function connect(port: number, agentId: string): void {
  currentPort = port;
  currentAgentId = agentId;

  const wsUrl = WORKER_URL.replace('https://', 'wss://').replace('http://', 'ws://');

  const url = `${wsUrl}/connect?agent_id=${encodeURIComponent(agentId)}`;

  log.info(`Connecting to ${url}...`);

  ws = new WebSocket(url);

  ws.on('open', () => {
    log.info(`✓ Connected as agent: ${agentId}`);
    log.info(`✓ Proxying to: http://127.0.0.1:${port}`);

    const publicUrl = WORKER_URL.replace(/\/$/, '') + `/tunnel/${agentId}`;
    log.info(`✓ Public URL: ${publicUrl}`);

    // Send keepalive ping every 20 seconds to prevent DO hibernation
    if (pingInterval) clearInterval(pingInterval);
    pingInterval = setInterval(() => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.ping();
      }
    }, 20000);
  });

  ws.on('message', async (data) => {
    const message = JSON.parse(data.toString());
    await handleRequest(message);
  });

  ws.on('close', () => {
    log.info('✗ Connection closed, reconnecting immediately...');
    if (pingInterval) clearInterval(pingInterval);
    // Reconnect immediately, not after 5 seconds
    if (isRunning) {
      reconnectTimeout = setTimeout(() => connect(currentPort, currentAgentId), 100);
    }
  });

  ws.on('error', (err) => {
    log.error('✗ WebSocket error:', err.message);
  });
}

export function startLapstoneTunnel(port: number, secret: string, goosedPid: number): TunnelInfo {
  isRunning = true;
  const agentId = getAgentId();
  const publicUrl = `${WORKER_URL}/tunnel/${agentId}`;

  connect(port, agentId);
  startNetworkMonitoring();
  startHealthChecks();

  return {
    url: publicUrl,
    ipv4: '',
    ipv6: '',
    hostname: new URL(WORKER_URL).hostname,
    secret,
    port,
    pids: {
      goosed: goosedPid,
      tailscale_serve: 0,
    },
  };
}

export function stopLapstoneTunnel(): void {
  isRunning = false;

  if (reconnectTimeout) clearTimeout(reconnectTimeout);
  if (pingInterval) clearInterval(pingInterval);
  if (healthCheckInterval) clearInterval(healthCheckInterval);
  if (networkMonitorInterval) clearInterval(networkMonitorInterval);

  if (ws) {
    ws.close();
    ws = null;
  }

  lastNetworkSnapshot = null;
}
