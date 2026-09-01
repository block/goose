import { setImmediate } from 'node:timers';
import http from 'node:http';
import https from 'node:https';
import crypto from 'node:crypto';
import { Buffer } from 'node:buffer';
import tls from 'node:tls';
import type { Socket } from 'node:net';
import type { App, Cookies, Session } from 'electron';
import {
  acpHttpUrlFromHttpBase,
  acpWebSocketUrlFromHttpBase,
  normalizeAcpHttpBaseUrl,
  statusHttpUrlFromHttpBase,
} from './acp/url';
import log from './utils/logger';
import { proxyFor, type ProxyEnvironment } from './proxy';

// The renderer cannot make these connections itself: Chromium aborts with
// ERR_SSL_CLIENT_AUTH_CERT_NEEDED when a backend behind mTLS requests a client
// certificate, WebSocket upgrades emit no event to answer it, and a renderer
// cannot set the User-Agent that auth proxies gate on. Node declines the
// certificate request and reports the real handshake status instead of
// collapsing every refusal into close code 1006.

const OPAQUE_ORIGIN = 'null';

const TEXT_FRAME = 0x1;
const CLOSE_FRAME = 0x8;
const PING_FRAME = 0x9;
const PONG_FRAME = 0xa;
const NORMAL_CLOSURE = 1000;
const ABNORMAL_CLOSURE = 1006;
const REQUEST_TIMEOUT_MS = 15_000;

const secureProtocols = new Set(['https:', 'wss:']);

interface Trust {
  hostname: string;
  fingerprint: string | null;
}

const trusted = new Set<Trust>();

let userAgent = 'goose-desktop';

const normalizeFingerprint = (fingerprint: string): string => {
  if (!fingerprint.startsWith('sha256/')) {
    return fingerprint.toUpperCase();
  }

  return Array.from(Buffer.from(fingerprint.slice('sha256/'.length), 'base64'))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join(':')
    .toUpperCase();
};

const trustsFor = (hostname: string): Trust[] => {
  const normalized = hostname.toLowerCase();
  return [...trusted].filter((trust) => trust.hostname === normalized);
};

const trust = (hostname: string, fingerprint: string | null): BackendTrust => {
  const entry: Trust = {
    hostname: hostname.toLowerCase(),
    fingerprint: fingerprint && normalizeFingerprint(fingerprint),
  };
  trusted.add(entry);

  return {
    pin: (candidate: string) => {
      const normalized = normalizeFingerprint(candidate);
      if (entry.fingerprint && entry.fingerprint !== normalized) {
        return false;
      }
      entry.fingerprint = normalized;
      return true;
    },
    release: () => {
      trusted.delete(entry);
    },
  };
};

const verify = (hostname: string, fingerprint: string): boolean => {
  const trusts = trustsFor(hostname);
  const normalized = normalizeFingerprint(fingerprint);

  if (trusts.some((entry) => entry.fingerprint === normalized)) {
    return true;
  }

  const unpinned = trusts.find((entry) => entry.fingerprint === null);
  if (unpinned) {
    unpinned.fingerprint = normalized;
    return true;
  }

  return false;
};

let cookieJar: Cookies | null = null;
let proxyEnvironment: ProxyEnvironment = process.env;

const dialHost = (url: URL): string =>
  url.hostname.startsWith('[') ? url.hostname.slice(1, -1) : url.hostname;

const dialPort = (url: URL): number =>
  Number(url.port || (secureProtocols.has(url.protocol) ? 443 : 80));

const cookieHeader = async (url: URL): Promise<http.OutgoingHttpHeaders> => {
  const cookies = await cookieJar?.get({ url: url.toString() }).catch(() => []);
  return cookies?.length
    ? { Cookie: cookies.map(({ name, value }) => `${name}=${value}`).join('; ') }
    : {};
};

const storeCookies = (url: URL, headers: http.IncomingHttpHeaders): void => {
  for (const cookie of headers['set-cookie'] ?? []) {
    const [pair] = cookie.split(';');
    const separator = pair.indexOf('=');
    if (separator > 0) {
      void cookieJar
        ?.set({
          url: url.toString(),
          name: pair.slice(0, separator).trim(),
          value: pair.slice(separator + 1).trim(),
        })
        .catch(() => undefined);
    }
  }
};

const connectDirectly = (url: URL): tls.TLSSocket => {
  const socket = tls.connect({
    host: dialHost(url),
    port: dialPort(url),
    servername: dialHost(url),
    rejectUnauthorized: false,
  });
  socket.on('secureConnect', () => verifyPeer(socket, url));
  return socket;
};

const verifyPeer = (socket: tls.TLSSocket, url: URL): void => {
  const fingerprint = socket.getPeerCertificate().fingerprint256;
  if (!fingerprint || !verify(url.hostname, fingerprint)) {
    socket.destroy(new Error(`The backend TLS certificate for ${url.hostname} is not trusted.`));
  }
};

const openTunnel = (url: URL, proxy: URL): Promise<Socket> =>
  new Promise((resolve, reject) => {
    const authority = `${dialHost(url)}:${dialPort(url)}`;
    const request = (secureProtocols.has(proxy.protocol) ? https : http).request({
      host: dialHost(proxy),
      port: dialPort(proxy),
      method: 'CONNECT',
      path: authority,
      headers: {
        Host: authority,
        ...(proxy.username
          ? {
              'Proxy-Authorization': `Basic ${Buffer.from(
                `${decodeURIComponent(proxy.username)}:${decodeURIComponent(proxy.password)}`
              ).toString('base64')}`,
            }
          : {}),
      },
    });
    request.on('connect', (response, socket) => {
      if (response.statusCode !== 200) {
        socket.destroy();
        reject(
          new Error(`The proxy refused a tunnel to ${authority} (HTTP ${response.statusCode}).`)
        );
        return;
      }
      resolve(socket);
    });
    request.on('error', reject);
    request.end();
  });

const openRequest = async (
  url: URL,
  headers: http.OutgoingHttpHeaders
): Promise<http.ClientRequest> => {
  const secure = secureProtocols.has(url.protocol);
  const proxy = proxyFor(url, proxyEnvironment);
  const requestHeaders = { 'User-Agent': userAgent, ...(await cookieHeader(url)), ...headers };

  if (!secure) {
    return http.request({
      host: dialHost(proxy ?? url),
      port: dialPort(proxy ?? url),
      path: proxy ? url.toString() : `${url.pathname}${url.search}`,
      method: 'GET',
      headers: requestHeaders,
    });
  }

  return https.request({
    host: dialHost(url),
    port: dialPort(url),
    path: `${url.pathname}${url.search}`,
    method: 'GET',
    headers: requestHeaders,
    createConnection: proxy
      ? (_options, callback) => {
          openTunnel(url, proxy).then(
            (tunnel) => {
              const socket = tls.connect({
                socket: tunnel,
                servername: dialHost(url),
                rejectUnauthorized: false,
              });
              socket.on('secureConnect', () => verifyPeer(socket, url));
              callback(null, socket);
            },
            (error: Error) => callback(error, null as unknown as Socket)
          );
          return undefined;
        }
      : () => connectDirectly(url),
  });
};

const proxyNote = (headers: http.IncomingHttpHeaders): string => {
  const doormanError = headers['x-sq-cf-doorman-error'];
  return typeof doormanError === 'string' && doormanError !== 'none'
    ? ` A proxy in front of the backend reported "${doormanError}".`
    : '';
};

interface BackendResponse {
  ok: boolean;
  status?: number;
  detail?: string;
}

const requestBackend = (
  rawUrl: string,
  extraHeaders: http.OutgoingHttpHeaders = {}
): Promise<BackendResponse> =>
  new Promise((resolve) => {
    let url: URL;
    try {
      url = new URL(rawUrl);
    } catch (error) {
      resolve({ ok: false, detail: error instanceof Error ? error.message : String(error) });
      return;
    }

    openRequest(url, extraHeaders).then((request) => {
      request.on('response', (response) => {
        response.resume();
        storeCookies(url, response.headers);
        resolve({ ok: true, status: response.statusCode, detail: proxyNote(response.headers) });
      });
      request.setTimeout(REQUEST_TIMEOUT_MS, () => {
        request.destroy();
        resolve({
          ok: false,
          detail: `The backend did not respond within ${REQUEST_TIMEOUT_MS}ms.`,
        });
      });
      request.on('error', (error: Error) => resolve({ ok: false, detail: error.message }));
      request.end();
    });
  });

interface BackendSocketHandlers {
  onOpen: () => void;
  onMessage: (data: string) => void;
  onClose: (code: number, reason: string) => void;
  onError: (message: string) => void;
}

export interface BackendSocket {
  send: (data: string) => void;
  close: (code?: number, reason?: string) => void;
}

export function openBackendSocket(
  rawUrl: string,
  handlers: BackendSocketHandlers,
  timeoutMs?: number
): BackendSocket {
  let socket: Socket | null = null;
  let closed = false;

  const finishClose = (code: number, reason: string): void => {
    if (closed) {
      return;
    }
    closed = true;
    socket?.destroy();
    socket = null;
    handlers.onClose(code, reason);
  };

  const fail = (message: string): void => {
    handlers.onError(message);
    finishClose(ABNORMAL_CLOSURE, message);
  };

  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setImmediate(() => fail(message));
    return { send: () => undefined, close: () => undefined };
  }

  let request: http.ClientRequest | null = null;

  void openRequest(url, {
    Connection: 'Upgrade',
    Upgrade: 'websocket',
    'Sec-WebSocket-Version': '13',
    'Sec-WebSocket-Key': crypto.randomBytes(16).toString('base64'),
    Origin: OPAQUE_ORIGIN,
  }).then((pending) => {
    if (closed) {
      pending.destroy();
      return;
    }
    request = pending;

    pending.on('upgrade', (response, upgraded) => {
      storeCookies(url, response.headers);
      socket = upgraded;
      upgraded.setNoDelay(true);

      const decoder = new FrameDecoder();
      upgraded.on('data', (chunk: Buffer) => {
        const { messages, pings, close } = decoder.push(chunk);
        for (const message of messages) {
          handlers.onMessage(message);
        }
        for (const payload of pings) {
          upgraded.write(encodeFrame(PONG_FRAME, payload));
        }
        if (close) {
          upgraded.write(encodeFrame(CLOSE_FRAME, Buffer.alloc(0)));
          finishClose(close.code, close.reason);
        }
      });

      upgraded.on('error', (error: Error) => fail(error.message));
      upgraded.on('close', () => finishClose(ABNORMAL_CLOSURE, 'The connection closed.'));
      upgraded.on('end', () => finishClose(ABNORMAL_CLOSURE, 'The connection ended.'));

      setImmediate(() => handlers.onOpen());
    });

    pending.on('response', (response) => {
      response.resume();
      storeCookies(url, response.headers);
      fail(
        `The ACP WebSocket upgrade was refused with HTTP ${response.statusCode}.` +
          proxyNote(response.headers)
      );
    });

    if (timeoutMs) {
      pending.setTimeout(timeoutMs, () =>
        fail(`The WebSocket did not respond within ${timeoutMs}ms.`)
      );
    }
    pending.on('error', (error: Error) => fail(error.message));
    pending.end();
  });

  return {
    send: (data) => socket?.write(encodeFrame(TEXT_FRAME, Buffer.from(data, 'utf8'))),
    close: (code = NORMAL_CLOSURE, reason = '') => {
      if (closed) {
        return;
      }
      if (socket) {
        const payload = Buffer.alloc(2 + Buffer.byteLength(reason));
        payload.writeUInt16BE(code, 0);
        payload.write(reason, 2, 'utf8');
        socket.write(encodeFrame(CLOSE_FRAME, payload));
      }
      request?.destroy();
      finishClose(code, reason);
    },
  };
}

function encodeFrame(opcode: number, payload: Buffer): Buffer {
  const mask = crypto.randomBytes(4);
  const masked = Buffer.from(payload);
  for (let index = 0; index < masked.length; index += 1) {
    masked[index] ^= mask[index % 4];
  }

  let header: Buffer;
  if (payload.length < 126) {
    header = Buffer.from([0x80 | opcode, 0x80 | payload.length]);
  } else if (payload.length < 0x10000) {
    header = Buffer.alloc(4);
    header[0] = 0x80 | opcode;
    header[1] = 0x80 | 126;
    header.writeUInt16BE(payload.length, 2);
  } else {
    header = Buffer.alloc(10);
    header[0] = 0x80 | opcode;
    header[1] = 0x80 | 127;
    header.writeBigUInt64BE(BigInt(payload.length), 2);
  }

  return Buffer.concat([header, mask, masked]);
}

interface DecodedFrames {
  messages: string[];
  pings: Buffer[];
  close: { code: number; reason: string } | null;
}

class FrameDecoder {
  private buffered: Buffer = Buffer.alloc(0);
  private continuation: Buffer[] = [];

  push(chunk: Buffer): DecodedFrames {
    this.buffered = Buffer.concat([this.buffered, chunk]);

    const messages: string[] = [];
    const pings: Buffer[] = [];
    let close: DecodedFrames['close'] = null;
    let offset = 0;

    while (offset + 2 <= this.buffered.length) {
      const isFinal = (this.buffered[offset] & 0x80) !== 0;
      const opcode = this.buffered[offset] & 0x0f;
      let length = this.buffered[offset + 1] & 0x7f;
      let payloadStart = offset + 2;

      if (length === 126) {
        if (payloadStart + 2 > this.buffered.length) break;
        length = this.buffered.readUInt16BE(payloadStart);
        payloadStart += 2;
      } else if (length === 127) {
        if (payloadStart + 8 > this.buffered.length) break;
        length = Number(this.buffered.readBigUInt64BE(payloadStart));
        payloadStart += 8;
      }

      if (payloadStart + length > this.buffered.length) {
        break;
      }

      const payload = this.buffered.subarray(payloadStart, payloadStart + length);

      if (opcode === 0x0 || opcode === TEXT_FRAME) {
        this.continuation.push(Buffer.from(payload));
        if (isFinal) {
          messages.push(Buffer.concat(this.continuation).toString('utf8'));
          this.continuation = [];
        }
      } else if (opcode === PING_FRAME) {
        pings.push(Buffer.from(payload));
      } else if (opcode === CLOSE_FRAME) {
        close =
          payload.length >= 2
            ? { code: payload.readUInt16BE(0), reason: payload.subarray(2).toString('utf8') }
            : { code: NORMAL_CLOSURE, reason: '' };
      }

      offset = payloadStart + length;
    }

    this.buffered = Buffer.from(this.buffered.subarray(offset));
    return { messages, pings, close };
  }
}

export interface ConnectionTestStep {
  name: string;
  ok: boolean;
  detail: string;
}

export interface ConnectionTestResult {
  ok: boolean;
  steps: ConnectionTestStep[];
}

class StepFailure extends Error {}

const failStep = (detail: string): never => {
  throw new StepFailure(detail);
};

async function runConnectionTest(
  url: string,
  secret: string,
  workingDir?: string
): Promise<ConnectionTestResult> {
  const steps: ConnectionTestStep[] = [];

  const step = async (name: string, probe: () => Promise<string> | string): Promise<void> => {
    try {
      steps.push({ name, ok: true, detail: await probe() });
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      steps.push({ name, ok: false, detail });
      throw error instanceof StepFailure ? error : new StepFailure(detail);
    }
  };

  let baseUrl = '';
  let handshake: AcpHandshake | undefined;

  try {
    await step('URL', () => (baseUrl = normalizeAcpHttpBaseUrl(url)));

    await step('Reachable', async () => {
      const status = await requestBackend(statusHttpUrlFromHttpBase(baseUrl), {
        'X-Secret-Key': secret,
      });
      if (!status.ok) {
        failStep(status.detail ?? 'The backend could not be reached.');
      }
      if (status.status !== 200) {
        failStep(`GET /status returned HTTP ${status.status}, expected 200.${status.detail ?? ''}`);
      }
      return `GET /status returned 200.${status.detail ?? ''}`;
    });

    await step('Secret key', async () => {
      const auth = await requestBackend(acpHttpUrlFromHttpBase(baseUrl, secret));
      if (!auth.ok) {
        failStep(auth.detail ?? 'GET /acp could not be reached.');
      }
      if (auth.status === 401 || auth.status === 403) {
        failStep(
          `The backend rejected the secret key (HTTP ${auth.status}). It must match GOOSE_SERVER__SECRET_KEY on the backend.${auth.detail ?? ''}`
        );
      }
      if (auth.status !== 406) {
        failStep(`GET /acp returned HTTP ${auth.status}, expected 406.${auth.detail ?? ''}`);
      }
      return 'The backend accepted the secret key.';
    });

    await step('ACP handshake', async () => {
      handshake = await openHandshake(acpWebSocketUrlFromHttpBase(baseUrl, secret));
      const result = await handshake.call('initialize', {
        protocolVersion: 1,
        clientCapabilities: {},
        clientInfo: { name: 'goose-connection-test', version: '1' },
      });
      const agent = result.agentInfo as { name?: string; version?: string } | undefined;
      return (
        `initialize returned protocol version ${result.protocolVersion ?? 'unknown'}.` +
        (agent?.name ? ` Backend is ${agent.name} ${agent.version ?? ''}.` : '')
      );
    });

    if (workingDir) {
      await step('Working directory', async () => {
        let probeSessionId: unknown;
        try {
          const session = await handshake!.call('session/new', {
            cwd: workingDir,
            mcpServers: [],
          });
          probeSessionId = session.sessionId;
        } catch (error) {
          failStep(
            `The backend rejected ${workingDir} (${error instanceof Error ? error.message : error}). It must be a directory that exists on the backend, not on this computer.`
          );
        }

        if (typeof probeSessionId === 'string') {
          await handshake!
            .call('session/delete', { sessionId: probeSessionId })
            .catch(() => undefined);
        }
        return `${workingDir} exists on the backend.`;
      });
    }
  } catch (error) {
    if (!(error instanceof StepFailure)) {
      throw error;
    }
  } finally {
    handshake?.close();
  }

  return { ok: steps.every((step) => step.ok), steps };
}

interface AcpHandshake {
  call: (method: string, params: unknown) => Promise<Record<string, unknown>>;
  close: () => void;
}

function openHandshake(wsUrl: string): Promise<AcpHandshake> {
  return new Promise((resolveOpen, rejectOpen) => {
    let nextId = 1;
    let pending: {
      resolve: (value: Record<string, unknown>) => void;
      reject: (e: Error) => void;
    } | null = null;

    const settle = (outcome: (p: NonNullable<typeof pending>) => void): void => {
      const current = pending;
      pending = null;
      if (current) {
        outcome(current);
      }
    };

    const abort = (message: string): void => {
      rejectOpen(new StepFailure(message));
      settle((p) => p.reject(new StepFailure(message)));
    };

    const socket = openBackendSocket(
      wsUrl,
      {
        onOpen: () =>
          resolveOpen({
            call: (method, params) =>
              new Promise((resolve, reject) => {
                pending = { resolve, reject };
                socket.send(JSON.stringify({ jsonrpc: '2.0', id: nextId++, method, params }));
              }),
            close: () => socket.close(),
          }),
        onMessage: (data) =>
          settle((p) => {
            let message: {
              error?: { message?: string; data?: unknown };
              result?: Record<string, unknown>;
            };
            try {
              message = JSON.parse(data);
            } catch {
              p.reject(new StepFailure(`the backend sent a non-JSON frame: ${data.slice(0, 200)}`));
              return;
            }

            if (message.error) {
              const { data: errorData, message: errorMessage } = message.error;
              p.reject(
                new StepFailure(
                  typeof errorData === 'string' ? errorData : (errorMessage ?? 'unknown error')
                )
              );
              return;
            }
            p.resolve(message.result ?? {});
          }),
        onError: abort,
        onClose: (_code, reason) =>
          abort(reason || 'The backend closed the WebSocket before replying.'),
      },
      REQUEST_TIMEOUT_MS
    );
  });
}

export interface BackendTrust {
  pin: (fingerprint: string) => boolean;
  release: () => void;
}

export interface BackendConnection {
  acpUrl: string;
  workingDir: string;
  release: () => Promise<void>;
}

export type ConnectionOutcome =
  { ok: true; connection: BackendConnection } | { ok: false; reason: string };

export interface BackendTarget {
  url: string;
  secret: string;
  certFingerprint?: string;
  workingDir?: string;
}

const trustHost = (rawUrl: string, certFingerprint?: string): BackendTrust | null => {
  try {
    const { protocol, hostname } = new URL(normalizeAcpHttpBaseUrl(rawUrl));
    return protocol === 'https:' ? trust(hostname, certFingerprint?.trim() || null) : null;
  } catch {
    return null;
  }
};

export async function connectToBackend({
  url,
  secret,
  certFingerprint,
  workingDir,
}: BackendTarget): Promise<ConnectionOutcome> {
  const baseUrl = normalizeAcpHttpBaseUrl(url);
  const hostTrust = trustHost(baseUrl, certFingerprint);

  try {
    const test = await runConnectionTest(baseUrl, secret, workingDir);
    if (!test.ok) {
      const failed = test.steps.find((step) => !step.ok);
      throw new Error(
        failed
          ? `${failed.name}: ${failed.detail}`.trim()
          : 'The backend did not respond as an ACP server.'
      );
    }
  } catch (error) {
    hostTrust?.release();
    return { ok: false, reason: error instanceof Error ? error.message : String(error) };
  }

  return {
    ok: true,
    connection: {
      acpUrl: acpWebSocketUrlFromHttpBase(baseUrl, secret),
      workingDir: workingDir ?? '',
      release: async () => hostTrust?.release(),
    },
  };
}

export async function probeBackend(target: BackendTarget): Promise<ConnectionTestResult> {
  const hostTrust = trustHost(target.url, target.certFingerprint);
  try {
    return await runConnectionTest(target.url, target.secret, target.workingDir);
  } finally {
    hostTrust?.release();
  }
}

export const trustLocalBackend = (): BackendTrust => trust('127.0.0.1', null);

export function installBackendTrust(
  targetApp: App,
  targetSessions: Pick<Session, 'setCertificateVerifyProc'>[],
  cookies?: Cookies,
  environment: ProxyEnvironment = process.env
): void {
  userAgent = `goose-desktop/${targetApp.getVersion()}`;
  cookieJar = cookies ?? null;
  proxyEnvironment = environment;

  targetApp.on('certificate-error', (event, _webContents, url, _error, certificate, callback) => {
    const { hostname } = new URL(url);
    if (trustsFor(hostname).length === 0) {
      callback(false);
      return;
    }

    event.preventDefault();
    callback(verify(hostname, certificate.fingerprint));
  });

  targetApp.on('select-client-certificate', (event, _webContents, url, certificates, callback) => {
    const [certificate] = certificates;
    if (!certificate) {
      return;
    }

    log.info(
      `[Main] Selected client certificate "${certificate.subjectName}" for ${url} of ${certificates.length} offered`
    );
    event.preventDefault();
    setTimeout(() => callback(certificate), 0);
  });

  for (const targetSession of targetSessions) {
    targetSession.setCertificateVerifyProc((request, callback) => {
      if (trustsFor(request.hostname).length === 0) {
        callback(-3);
        return;
      }
      callback(verify(request.hostname, request.certificate.fingerprint) ? 0 : -2);
    });
  }
}
