import WebSocket from 'ws';
import * as http from 'http';
import * as crypto from 'crypto';
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
let currentPort: number = 0;
let currentAgentId: string = '';
let isRunning: boolean = false;
let lastPongReceived: number = Date.now();

// Monitoring configuration
const PING_INTERVAL = 20000; // 20 seconds - send ping to keep connection alive
const HEALTH_CHECK_INTERVAL = 25000; // 25 seconds - check if pong received
const PONG_TIMEOUT = 30000; // 30 seconds - if no pong after this, connection is dead

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

function checkTunnelHealth(): boolean {
  // Check if WebSocket is connected
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    log.warn('Tunnel health check failed: WebSocket not connected');
    return false;
  }

  // Check if we received a pong recently (within timeout period)
  const timeSinceLastPong = Date.now() - lastPongReceived;
  if (timeSinceLastPong > PONG_TIMEOUT) {
    log.warn(`Tunnel health check failed: No pong received for ${timeSinceLastPong}ms`);
    return false;
  }

  log.info('Tunnel health check passed: WebSocket alive and responsive');
  return true;
}

function startHealthChecks(): void {
  if (healthCheckInterval) clearInterval(healthCheckInterval);

  healthCheckInterval = setInterval(() => {
    if (!isRunning) return;

    try {
      const healthy = checkTunnelHealth();

      if (!healthy) {
        restartTunnel();
      }
    } catch (err) {
      // Health check threw an error - log and restart
      log.error('Health check error:', err);
      restartTunnel();
    }
  }, HEALTH_CHECK_INTERVAL);

  // Run initial health check after a short delay (give WS time to fully connect)
  setTimeout(() => {
    if (!isRunning) return;

    try {
      checkTunnelHealth();
    } catch (err) {
      // Initial health check failed - just log it, interval will retry
      log.error('Initial health check error:', err);
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

    // Reset pong tracking
    lastPongReceived = Date.now();

    // Send keepalive ping every 20 seconds to keep connection alive and detect dead connections
    if (pingInterval) clearInterval(pingInterval);
    pingInterval = setInterval(() => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.ping();
      }
    }, PING_INTERVAL);
  });

  ws.on('pong', () => {
    // Update last pong received time
    lastPongReceived = Date.now();
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

  if (ws) {
    ws.close();
    ws = null;
  }
}
