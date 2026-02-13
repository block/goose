/**
 * HTTP CONNECT proxy with logging, live domain blocklist, and optional
 * LaunchDarkly egress control.
 *
 * Runs in the Electron main process. All outbound traffic from a sandboxed
 * goosed process is funneled through this proxy (the macOS seatbelt profile
 * blocks direct outbound network, only allowing localhost).
 *
 * Blocking layers (checked in order):
 *   1. Local blocklist (blocked.txt) — fast, no network, live-reloaded
 *   2. LaunchDarkly flag ("egress-allowlist") — if configured, evaluates
 *      per-domain with a TTL cache. Unreachable LD → default allow.
 */

import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import fs from 'node:fs';
import os from 'node:os';
import crypto from 'node:crypto';
import { URL } from 'node:url';
import { Buffer } from 'node:buffer';
import log from '../utils/logger';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LaunchDarklyConfig {
  clientId: string;
  username?: string;
  cacheTtlSeconds?: number;
}

export interface ProxyOptions {
  port?: number;
  blockedPath?: string;
  launchDarkly?: LaunchDarklyConfig;
}

export interface ProxyInstance {
  port: number;
  server: http.Server;
  close: () => Promise<void>;
}

// ---------------------------------------------------------------------------
// Local blocklist
// ---------------------------------------------------------------------------

function loadBlocked(blockedPath: string | undefined): Set<string> {
  if (!blockedPath) return new Set();
  try {
    if (!fs.existsSync(blockedPath)) return new Set();
    const domains = new Set<string>();
    for (const line of fs.readFileSync(blockedPath, 'utf-8').split('\n')) {
      const trimmed = line.trim().toLowerCase();
      if (trimmed && !trimmed.startsWith('#')) {
        domains.add(trimmed);
      }
    }
    return domains;
  } catch {
    return new Set();
  }
}

function matchesBlocked(host: string, blocked: Set<string>): boolean {
  const h = host.toLowerCase();
  if (blocked.has(h)) return true;
  const parts = h.split('.');
  for (let i = 1; i < parts.length; i++) {
    const parent = parts.slice(i).join('.');
    if (blocked.has(parent)) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// LaunchDarkly client-side evaluation (no SDK — direct REST calls)
// ---------------------------------------------------------------------------

interface LDFlagResult {
  value: boolean;
  variation?: number;
  version?: number;
  flagVersion?: number;
}

class TTLCache {
  private cache = new Map<string, { value: boolean; ts: number }>();
  private ttl: number;

  constructor(ttlSeconds: number) {
    this.ttl = ttlSeconds * 1000;
  }

  get(key: string): boolean | undefined {
    const entry = this.cache.get(key);
    if (!entry) return undefined;
    if (Date.now() - entry.ts > this.ttl) {
      this.cache.delete(key);
      return undefined;
    }
    return entry.value;
  }

  put(key: string, value: boolean): void {
    this.cache.set(key, { value, ts: Date.now() });
  }
}

function httpsRequest(
  url: string,
  method: string,
  headers: Record<string, string>,
  body?: string
): Promise<{ status: number; body: string }> {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const req = https.request(
      {
        hostname: parsed.hostname,
        port: parsed.port || 443,
        path: parsed.pathname + parsed.search,
        method,
        headers,
        timeout: 5000,
      },
      (res) => {
        const chunks: Buffer[] = [];
        res.on('data', (chunk: Buffer) => chunks.push(chunk));
        res.on('end', () => {
          resolve({
            status: res.statusCode || 0,
            body: Buffer.concat(chunks).toString('utf-8'),
          });
        });
      }
    );
    req.on('error', reject);
    req.on('timeout', () => {
      req.destroy();
      reject(new Error('Request timed out'));
    });
    if (body) req.write(body);
    req.end();
  });
}

async function evaluateLDFlag(
  clientId: string,
  username: string,
  domain: string
): Promise<LDFlagResult | null> {
  const url = `https://clientsdk.launchdarkly.com/sdk/evalx/${clientId}/context`;
  const context = { kind: 'user', key: domain, username };
  try {
    const resp = await httpsRequest(url, 'REPORT', { 'Content-Type': 'application/json' }, JSON.stringify(context));
    const flags = JSON.parse(resp.body);
    const flag = flags['egress-allowlist'];
    if (!flag || !('value' in flag)) return null;
    return flag as LDFlagResult;
  } catch {
    return null;
  }
}

function sendLDEvent(clientId: string, username: string, domain: string, flag: LDFlagResult): void {
  // Fire-and-forget — don't await, don't block the proxy
  const url = `https://events.launchdarkly.com/events/bulk/${clientId}`;
  const ts = Date.now();
  const events = [
    {
      kind: 'index',
      creationDate: ts,
      context: { kind: 'user', key: domain, username },
    },
    {
      kind: 'summary',
      startDate: ts - 60000,
      endDate: ts,
      features: {
        'egress-allowlist': {
          default: false,
          contextKinds: ['user'],
          counters: [
            {
              variation: flag.variation,
              version: flag.version ?? flag.flagVersion,
              value: flag.value,
              count: 1,
            },
          ],
        },
      },
    },
  ];
  httpsRequest(
    url,
    'POST',
    {
      'Content-Type': 'application/json',
      'X-LaunchDarkly-Event-Schema': '4',
      'X-LaunchDarkly-Payload-ID': crypto.randomUUID(),
    },
    JSON.stringify(events)
  ).catch(() => {
    // fire-and-forget
  });
}

// ---------------------------------------------------------------------------
// Combined blocking check
// ---------------------------------------------------------------------------

async function checkBlocked(
  host: string,
  blockedPath: string | undefined,
  ldConfig: LaunchDarklyConfig | undefined,
  ldCache: TTLCache | undefined
): Promise<{ blocked: boolean; reason: string }> {
  // LaunchDarkly replaces blocked.txt when configured
  if (ldConfig && ldCache) {
    const domain = host.toLowerCase();
    const cached = ldCache.get(domain);
    if (cached !== undefined) {
      log.info(`[sandbox-proxy] LD:HIT ${host} ${cached ? 'allow' : 'deny'}`);
      return { blocked: !cached, reason: cached ? '' : 'launchdarkly (cached)' };
    }

    const flag = await evaluateLDFlag(
      ldConfig.clientId,
      ldConfig.username || os.userInfo().username,
      domain
    );
    if (flag !== null) {
      ldCache.put(domain, flag.value);
      const action = flag.value ? 'LD:OK' : 'LD:BLK';
      log.info(`[sandbox-proxy] ${action} ${host}`);
      sendLDEvent(ldConfig.clientId, ldConfig.username || os.userInfo().username, domain, flag);
      return { blocked: !flag.value, reason: flag.value ? '' : 'launchdarkly' };
    }

    // LD unreachable — default allow
    log.info(`[sandbox-proxy] LD:ERR ${host} (defaulting to allow)`);
    return { blocked: false, reason: '' };
  }

  // No LD — use local blocklist
  const blocked = loadBlocked(blockedPath);
  if (matchesBlocked(host, blocked)) {
    return { blocked: true, reason: 'blocklist' };
  }

  return { blocked: false, reason: '' };
}

// ---------------------------------------------------------------------------
// Proxy server
// ---------------------------------------------------------------------------

export async function startProxy(options: ProxyOptions = {}): Promise<ProxyInstance> {
  const { blockedPath, launchDarkly } = options;
  const ldCache = launchDarkly ? new TTLCache(launchDarkly.cacheTtlSeconds ?? 3600) : undefined;

  const server = http.createServer((req, res) => {
    const url = req.url || '';
    let host = '';
    try {
      const parsed = new URL(url);
      host = parsed.hostname || '';
    } catch {
      host = '';
    }

    // Use void to handle the async check without making the callback async
    void (async () => {
      if (host) {
        const result = await checkBlocked(host, blockedPath, launchDarkly, ldCache);
        if (result.blocked) {
          log.info(`[sandbox-proxy] BLOCK ${req.method} ${url.slice(0, 120)} (${result.reason})`);
          res.writeHead(403, { 'Content-Type': 'text/plain' });
          res.end(`Blocked by sandbox proxy: ${host}`);
          return;
        }
      }

      log.info(`[sandbox-proxy] ALLOW ${req.method} ${url.slice(0, 120)}`);

      let parsedUrl: URL;
      try {
        parsedUrl = new URL(url);
      } catch {
        res.writeHead(400);
        res.end('Bad request URL');
        return;
      }

      const proxyReq = http.request(
        {
          hostname: parsedUrl.hostname,
          port: parsedUrl.port || 80,
          path: parsedUrl.pathname + parsedUrl.search,
          method: req.method,
          headers: { ...req.headers, host: parsedUrl.host },
        },
        (proxyRes) => {
          res.writeHead(proxyRes.statusCode || 502, proxyRes.headers);
          proxyRes.pipe(res);
        }
      );

      proxyReq.on('error', (err) => {
        log.error(`[sandbox-proxy] ERROR ${req.method} ${url.slice(0, 120)}: ${err.message}`);
        if (!res.headersSent) {
          res.writeHead(502);
          res.end(`Proxy error: ${err.message}`);
        }
      });

      req.pipe(proxyReq);
    })();
  });

  // Handle CONNECT for HTTPS tunneling
  server.on('connect', (req, clientSocket, head) => {
    const target = req.url || '';
    const [host, portStr] = target.split(':');
    const port = parseInt(portStr || '443', 10);

    void (async () => {
      const result = await checkBlocked(host, blockedPath, launchDarkly, ldCache);
      if (result.blocked) {
        log.info(`[sandbox-proxy] BLOCK CONNECT ${target} (${result.reason})`);
        clientSocket.write('HTTP/1.1 403 Forbidden\r\n\r\n');
        clientSocket.destroy();
        return;
      }

      log.info(`[sandbox-proxy] ALLOW CONNECT ${target}`);

      const remoteSocket = net.connect(port, host, () => {
        clientSocket.write('HTTP/1.1 200 Connection Established\r\n\r\n');
        if (head.length > 0) {
          remoteSocket.write(head);
        }
        remoteSocket.pipe(clientSocket);
        clientSocket.pipe(remoteSocket);
      });

      remoteSocket.on('error', (err) => {
        log.error(`[sandbox-proxy] ERROR CONNECT ${target}: ${err.message}`);
        clientSocket.write('HTTP/1.1 502 Bad Gateway\r\n\r\n');
        clientSocket.destroy();
      });

      clientSocket.on('error', () => {
        remoteSocket.destroy();
      });
    })();
  });

  return new Promise((resolve, reject) => {
    const listenPort = options.port || 0;
    server.listen(listenPort, '127.0.0.1', () => {
      const addr = server.address();
      if (!addr || typeof addr === 'string') {
        reject(new Error('Failed to get proxy server address'));
        return;
      }
      const actualPort = addr.port;
      log.info(`[sandbox-proxy] Listening on 127.0.0.1:${actualPort}`);
      if (blockedPath) {
        log.info(`[sandbox-proxy] Blocked domains file: ${blockedPath}`);
      }
      if (launchDarkly) {
        log.info(
          `[sandbox-proxy] LaunchDarkly: enabled (user=${launchDarkly.username || os.userInfo().username}, flag=egress-allowlist, cache=${launchDarkly.cacheTtlSeconds ?? 3600}s)`
        );
      }

      resolve({
        port: actualPort,
        server,
        close: () =>
          new Promise<void>((res) => {
            server.close(() => res());
          }),
      });
    });

    server.on('error', reject);
  });
}
