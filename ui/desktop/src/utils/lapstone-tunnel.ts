import WebSocket from 'ws';
import * as http from 'http';
import * as net from 'net';
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
let currentPort: number = 0;
let currentAgentId: string = '';
let isRunning: boolean = false;
let lastActivityTime: number = Date.now();
let idleCheckInterval: ReturnType<typeof setInterval> | null = null;

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
    const isStreamingResponse = responseHeaders['content-type']?.includes('text/event-stream');

    if (isStreamingResponse) {
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
      const chunks: Buffer[] = [];

      res.on('data', (chunk) => chunks.push(chunk));

      res.on('end', () => {
        const responseBody = Buffer.concat(chunks).toString();
        const MAX_WS_SIZE = 900000; // 900KB limit for WebSocket messages

        if (responseBody.length > MAX_WS_SIZE) {
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

    // Enable TCP keepalive to detect dead connections faster
    const socket = (ws as WebSocket & { _socket: net.Socket })._socket;
    socket.setKeepAlive(true, 30000);

    lastActivityTime = Date.now();

    // Reconnect if no activity for 10 minutes
    if (idleCheckInterval) clearInterval(idleCheckInterval);
    idleCheckInterval = setInterval(() => {
      const idleTime = Date.now() - lastActivityTime;
      if (idleTime > 10 * 60 * 1000) {
        log.warn('No activity for 10 minutes, reconnecting...');
        ws?.close();
      }
    }, 60000);
  });

  ws.on('message', async (data) => {
    try {
      lastActivityTime = Date.now();
      const message = JSON.parse(data.toString());
      await handleRequest(message);
    } catch (error) {
      log.error('Error handling WebSocket message:', error);
    }
  });

  ws.on('close', () => {
    log.info('✗ Connection closed, reconnecting immediately...');
    if (idleCheckInterval) clearInterval(idleCheckInterval);
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
  if (idleCheckInterval) clearInterval(idleCheckInterval);

  if (ws) {
    ws.close();
    ws = null;
  }
}
