/**
 * HTTP CONNECT proxy with logging, live domain blocklist, and optional
 * LaunchDarkly egress control.
 *
 * Runs in the Electron main process. All outbound traffic from a sandboxed
 * goosed process is funneled through this proxy (the macOS seatbelt profile
 * blocks direct outbound network, only allowing localhost).
 *
 * SSH git operations are routed through this proxy via GIT_SSH_COMMAND
 * which uses a bundled connect-proxy script as ProxyCommand.
 *
 * Blocking layers (checked in order):
 *   1. Loopback detection (if blockLoopback enabled)
 *   2. IP address blocking (if !allowIPAddresses)
 *   3. Local blocklist (blocked.txt) — fast, no network, live-reloaded
 *   4. SSH/Git host restriction (port 22/2222/7999)
 *   5. LaunchDarkly flag ("egress-allowlist") — if configured
 */

import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import fs from 'node:fs';
import os from 'node:os';
import crypto from 'node:crypto';
import { URL } from 'node:url';
import { Buffer } from 'node:buffer';
const log = {
  info: (...args: unknown[]) => console.log('[sandbox-proxy]', ...args),
  warn: (...args: unknown[]) => console.warn('[sandbox-proxy]', ...args),
  error: (...args: unknown[]) => console.error('[sandbox-proxy]', ...args),
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LaunchDarklyConfig {
  clientId: string;
  username?: string;
  cacheTtlSeconds?: number;
  failoverMode?: 'allow' | 'deny' | 'blocklist';
}

export interface ProxyOptions {
  port?: number;
  blockedPath?: string;
  launchDarkly?: LaunchDarklyConfig;
  allowIPAddresses?: boolean;
  blockLoopback?: boolean;
  allowSSH?: boolean;
  gitHosts?: string[];
  allowSSHToAllHosts?: boolean;
}

export interface ProxyInstance {
  port: number;
  server: http.Server;
  close: () => Promise<void>;
}

// ---------------------------------------------------------------------------
// Local blocklist
// ---------------------------------------------------------------------------

export function loadBlocked(blockedPath: string | undefined): Set<string> {
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

export function normalizeDomain(host: string): string {
  let normalized = host.toLowerCase().trim();
  if (normalized.endsWith('.')) {
    normalized = normalized.slice(0, -1);
  }
  if (normalized.startsWith('[') && normalized.endsWith(']')) {
    normalized = normalized.slice(1, -1);
  }
  try {
    const url = new URL(`http://${normalized}`);
    normalized = url.hostname;
  } catch {
    // use as-is
  }
  return normalized;
}

export function isIPAddress(host: string): boolean {
  const ipv4 = /^(\d{1,3}\.){3}\d{1,3}$/;
  if (ipv4.test(host)) return true;
  if (host.includes(':')) return true;
  return false;
}

export function parseConnectTarget(target: string): { host: string; port: number } {
  // Handle [ipv6]:port
  const bracketMatch = target.match(/^\[([^\]]+)\]:(\d+)$/);
  if (bracketMatch) {
    return { host: bracketMatch[1], port: parseInt(bracketMatch[2], 10) };
  }

  // Handle host:port (only split on the last colon to avoid IPv6 issues)
  const lastColon = target.lastIndexOf(':');
  if (lastColon <= 0) {
    return { host: '', port: 0 };
  }

  const host = target.slice(0, lastColon);
  const port = parseInt(target.slice(lastColon + 1), 10);
  if (!host || isNaN(port) || port <= 0 || port > 65535) {
    return { host: '', port: 0 };
  }

  return { host, port };
}

const LOOPBACK_RE = /^(localhost|127\.\d+\.\d+\.\d+|::1|\[::1\])$/i;

export function isLoopback(host: string): boolean {
  return LOOPBACK_RE.test(host);
}

const DEFAULT_GIT_HOSTS = ['github.com', 'gitlab.com', 'bitbucket.org', 'ssh.dev.azure.com'];

export function matchesBlocked(host: string, blocked: Set<string>): boolean {
  const h = normalizeDomain(host);
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
    const resp = await httpsRequest(
      url,
      'REPORT',
      { 'Content-Type': 'application/json' },
      JSON.stringify(context)
    );
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

export async function checkBlocked(
  host: string,
  port: number,
  blocked: Set<string>,
  ldConfig: LaunchDarklyConfig | undefined,
  ldCache: TTLCache | undefined,
  options: ProxyOptions
): Promise<{ blocked: boolean; reason: string }> {
  const normalized = normalizeDomain(host);

  if (options.blockLoopback && isLoopback(normalized)) {
    log.warn(
      `[sandbox-proxy] BLOCK loopback ${host}:${port} — if this breaks a local tool, it may not be respecting no_proxy`
    );
    return { blocked: true, reason: 'loopback' };
  }

  if (!options.allowIPAddresses && isIPAddress(normalized)) {
    return { blocked: true, reason: 'ip-address' };
  }

  if (matchesBlocked(normalized, blocked)) {
    return { blocked: true, reason: 'blocklist' };
  }

  if (port === 22 || port === 2222 || port === 7999) {
    if (options.allowSSH === false) {
      return { blocked: true, reason: 'ssh-disabled' };
    }
    if (!options.allowSSHToAllHosts) {
      const gitHosts = options.gitHosts || DEFAULT_GIT_HOSTS;
      const isGitHost = gitHosts.some((gh) => normalized === gh || normalized.endsWith('.' + gh));
      if (!isGitHost) {
        return { blocked: true, reason: 'ssh-non-git-host' };
      }
    }
  }

  if (ldConfig && ldCache) {
    const cached = ldCache.get(normalized);
    if (cached !== undefined) {
      log.info(`[sandbox-proxy] LD:HIT ${host} ${cached ? 'allow' : 'deny'}`);
      return { blocked: !cached, reason: cached ? '' : 'launchdarkly (cached)' };
    }

    const flag = await evaluateLDFlag(
      ldConfig.clientId,
      ldConfig.username || os.userInfo().username,
      normalized
    );
    if (flag !== null) {
      ldCache.put(normalized, flag.value);
      const action = flag.value ? 'LD:OK' : 'LD:BLK';
      log.info(`[sandbox-proxy] ${action} ${host}`);
      sendLDEvent(ldConfig.clientId, ldConfig.username || os.userInfo().username, normalized, flag);
      return { blocked: !flag.value, reason: flag.value ? '' : 'launchdarkly' };
    }

    const failover = ldConfig.failoverMode || 'allow';
    if (failover === 'deny') {
      log.warn(`[sandbox-proxy] LD:FAILOVER-DENY ${host}`);
      return { blocked: true, reason: 'launchdarkly-unreachable' };
    }
    if (failover === 'blocklist') {
      log.warn(`[sandbox-proxy] LD:FAILOVER-BLOCKLIST ${host}`);
      if (matchesBlocked(normalized, blocked)) {
        return { blocked: true, reason: 'blocklist (LD fallback)' };
      }
    }
    log.info(`[sandbox-proxy] LD:ERR ${host} (defaulting to allow)`);
    return { blocked: false, reason: '' };
  }

  return { blocked: false, reason: '' };
}

// ---------------------------------------------------------------------------
// Proxy server
// ---------------------------------------------------------------------------

export async function startProxy(options: ProxyOptions = {}): Promise<ProxyInstance> {
  const { blockedPath, launchDarkly } = options;
  const ldCache = launchDarkly ? new TTLCache(launchDarkly.cacheTtlSeconds ?? 3600) : undefined;
  let blockedSet = loadBlocked(blockedPath);
  let watcher: fs.FSWatcher | undefined;
  if (blockedPath) {
    try {
      watcher = fs.watch(blockedPath, () => {
        blockedSet = loadBlocked(blockedPath);
      });
    } catch {
      // file may not exist yet
    }
  }

  const server = http.createServer((req, res) => {
    const url = req.url || '';
    let host = '';
    let reqPort = 80;
    try {
      const parsed = new URL(url);
      host = parsed.hostname || '';
      reqPort = parseInt(parsed.port, 10) || 80;
    } catch {
      host = '';
    }

    // Use void to handle the async check without making the callback async
    void (async () => {
      if (host) {
        const result = await checkBlocked(
          host,
          reqPort,
          blockedSet,
          launchDarkly,
          ldCache,
          options
        );
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
    const { host, port } = parseConnectTarget(target);

    if (!host || !port) {
      log.error(`[sandbox-proxy] REJECT CONNECT invalid target: ${target}`);
      clientSocket.write('HTTP/1.1 400 Bad Request\r\n\r\n');
      clientSocket.destroy();
      return;
    }

    void (async () => {
      const result = await checkBlocked(host, port, blockedSet, launchDarkly, ldCache, options);
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
    // Bind exclusively to IPv4 loopback — the proxy must never be reachable from non-loopback interfaces.
    // The sandboxed process connects via HTTP_PROXY=http://127.0.0.1:PORT so IPv6 is not needed.
    server.listen(listenPort, '127.0.0.1', () => {
      const addr = server.address();
      if (!addr || typeof addr === 'string') {
        reject(new Error('Failed to get proxy server address'));
        return;
      }
      const actualPort = addr.port;
      log.info(`[sandbox-proxy] Listening on 127.0.0.1:${actualPort} (loopback only)`);
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
            watcher?.close();
            server.close(() => res());
          }),
      });
    });

    server.on('error', reject);
  });
}
