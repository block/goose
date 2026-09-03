#!/usr/bin/env node
// Local stand-in for a remote ACP backend, for testing the desktop app's
// external-backend connection path without real remote infrastructure.
//
//   node scripts/remote-backend-testbed.mjs [scenario]
//
// Scenarios:
//   plain        TLS + secret key, permissive CORS. Should connect.
//   redirect     Redirects /status and /acp to a second origin. Should connect.
//   mtls         Requires a client certificate.
//   strict-cors  Rejects any request carrying an Origin header, like the Blox
//                edge proxy. Reproduces "Reachable: Failed to fetch".
//
// Prints the URL, secret and working directory to paste into the desktop app,
// then logs every request with its Origin and client certificate.

import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCENARIOS = new Set(['plain', 'redirect', 'mtls', 'strict-cors']);
const scenario = process.argv[2] ?? 'plain';
if (!SCENARIOS.has(scenario)) {
  console.error(`Unknown scenario "${scenario}". Use one of: ${[...SCENARIOS].join(', ')}`);
  process.exit(1);
}

const repoRoot = path.resolve(fileURLToPath(import.meta.url), '../../../..');
const gooseBinary =
  process.env.GOOSE_BINARY ??
  ['target/release/goose', 'target/debug/goose']
    .map((candidate) => path.join(repoRoot, candidate))
    .find((candidate) => fs.existsSync(candidate));

if (!gooseBinary) {
  console.error('No goose binary found. Run `cargo build --release` or set GOOSE_BINARY.');
  process.exit(1);
}

const workDir = fs.mkdtempSync(path.join(os.tmpdir(), 'goose-testbed-'));
const secret = 'testbed-secret-0123456789abcdef';

const log = (...parts) => console.log(...parts);
const shortUrl = (url) => (url.length > 58 ? `${url.slice(0, 58)}…` : url);

const sh = async (command) => {
  const child = spawn('bash', ['-c', command], { cwd: workDir, stdio: 'ignore' });
  const [code] = await once(child, 'close');
  if (code !== 0) throw new Error(`Command failed (${code}): ${command}`);
};

const capture = async (command) => {
  const child = spawn('bash', ['-c', command], { cwd: workDir });
  let out = '';
  child.stdout.on('data', (chunk) => (out += chunk));
  await once(child, 'close');
  return out.trim();
};

const freePort = async () => {
  const server = net.createServer();
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
};

const waitForPort = async (port, timeoutMs = 120_000) => {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const reachable = await new Promise((resolve) => {
      const socket = net.connect(port, '127.0.0.1');
      socket.once('connect', () => {
        socket.destroy();
        resolve(true);
      });
      socket.once('error', () => resolve(false));
      socket.setTimeout(1000, () => {
        socket.destroy();
        resolve(false);
      });
    });
    if (reachable) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Port ${port} never opened`);
};

const upstreamPort = await freePort();
const edgePort = await freePort();
const redirectTargetPort = await freePort();

log(`\n=== goose remote backend testbed: ${scenario} ===`);
log(`working dir: ${workDir}`);

await sh(
  `openssl req -x509 -newkey rsa:2048 -keyout ca-key.pem -out ca-cert.pem -days 2 -nodes ` +
    `-subj "/CN=Goose Testbed CA" 2>/dev/null`
);
await sh(
  `openssl req -newkey rsa:2048 -keyout edge-key.pem -out edge.csr -nodes -subj "/CN=localhost" 2>/dev/null && ` +
    `openssl x509 -req -in edge.csr -CA ca-cert.pem -CAkey ca-key.pem -CAcreateserial -out edge-cert.pem ` +
    `-days 2 -extfile <(printf "subjectAltName=DNS:localhost,IP:127.0.0.1") 2>/dev/null`
);

if (scenario === 'mtls') {
  await sh(
    `openssl req -newkey rsa:2048 -keyout client-key.pem -out client.csr -nodes ` +
      `-subj "/CN=Goose Testbed Client" 2>/dev/null && ` +
      `openssl x509 -req -in client.csr -CA ca-cert.pem -CAkey ca-key.pem -CAcreateserial ` +
      `-out client-cert.pem -days 2 -extfile <(printf "extendedKeyUsage=clientAuth") 2>/dev/null && ` +
      `openssl pkcs12 -export -inkey client-key.pem -in client-cert.pem -certfile ca-cert.pem ` +
      `-passout pass:goose -out client.p12 2>/dev/null`
  );
}

const edgeFingerprint = (
  await capture('openssl x509 -in edge-cert.pem -noout -fingerprint -sha256')
).split('=')[1];

const upstream = spawn(
  gooseBinary,
  [
    'serve',
    '--host',
    '127.0.0.1',
    '--port',
    String(upstreamPort),
    '--with-builtin',
    'developer',
    // The edge rewrites Origin to this so goose's own CORS layer accepts
    // requests arriving through the proxy.
    '--allowed-origin',
    'http://goose.testbed',
  ],
  {
    cwd: workDir,
    env: { ...process.env, GOOSE_SERVER__SECRET_KEY: secret },
    stdio: ['ignore', 'pipe', 'pipe'],
  }
);

const relay = (stream) => {
  let buffer = '';
  stream.on('data', (chunk) => {
    buffer += chunk;
    const lines = buffer.split('\n');
    buffer = lines.pop() ?? '';
    for (const line of lines) if (line.trim()) log(`  [goose serve] ${line}`);
  });
};
relay(upstream.stdout);
relay(upstream.stderr);

process.on('exit', () => upstream.kill());
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(0));

log(`\nwaiting for goose serve on :${upstreamPort} …`);
await waitForPort(upstreamPort);
log('upstream ready');

const tlsOptions = {
  key: fs.readFileSync(path.join(workDir, 'edge-key.pem')),
  cert: fs.readFileSync(path.join(workDir, 'edge-cert.pem')),
  // Advertising the testbed CA makes Chromium offer only certs issued by it,
  // which is how a real mTLS backend narrows the choice.
  ...(scenario === 'mtls'
    ? {
        requestCert: true,
        rejectUnauthorized: false,
        ca: [fs.readFileSync(path.join(workDir, 'ca-cert.pem'))],
      }
    : {}),
};

const describeCert = (socket) => {
  if (scenario !== 'mtls') return '';
  const cert = socket.getPeerCertificate?.();
  const name = cert && Object.keys(cert).length ? (cert.subject?.CN ?? '(unnamed)') : 'NONE';
  return ` clientCert=${name}`;
};

const corsHeaders = (req) => ({
  'access-control-allow-origin': req.headers.origin ?? '*',
  'access-control-allow-headers': 'x-secret-key,content-type,accept',
  'access-control-allow-methods': 'GET,POST,DELETE,OPTIONS',
  'access-control-max-age': '600',
});

const proxy = (req, res) => {
  const headers = { ...req.headers, host: `127.0.0.1:${upstreamPort}` };
  if (headers.origin) headers.origin = 'http://goose.testbed';
  const upstreamReq = http.request(
    { host: '127.0.0.1', port: upstreamPort, method: req.method, path: req.url, headers },
    (upstreamRes) => {
      // goose echoes the rewritten Origin in its CORS headers, which the browser
      // would reject, so restate them for the real caller.
      res.writeHead(upstreamRes.statusCode ?? 502, {
        ...upstreamRes.headers,
        ...corsHeaders(req),
      });
      upstreamRes.pipe(res);
    }
  );
  upstreamReq.on('error', (error) => {
    res.writeHead(502);
    res.end(String(error.message));
  });
  req.pipe(upstreamReq);
};

const proxyUpgrade = (req, socket, head) => {
  const headers = { ...req.headers, host: `127.0.0.1:${upstreamPort}` };
  if (headers.origin) headers.origin = 'http://goose.testbed';
  const upstreamReq = http.request({
    host: '127.0.0.1',
    port: upstreamPort,
    method: req.method,
    path: req.url,
    headers,
  });
  upstreamReq.end();
  upstreamReq.on('upgrade', (upstreamRes, upstreamSocket, upstreamHead) => {
    const raw = Object.entries(upstreamRes.headers)
      .map(([key, value]) => `${key}: ${value}\r\n`)
      .join('');
    socket.write(`HTTP/1.1 ${upstreamRes.statusCode} ${upstreamRes.statusMessage}\r\n${raw}\r\n`);
    if (upstreamHead?.length) socket.unshift(upstreamHead);
    upstreamSocket.pipe(socket).pipe(upstreamSocket);
  });
  upstreamReq.on('error', () => socket.destroy());
  if (head?.length) upstreamReq.write(head);
};

const edge = https.createServer(tlsOptions, (req, res) => {
  log(
    `  [edge] ${req.method} ${shortUrl(req.url ?? '')} origin=${req.headers.origin ?? '(none)'}` +
      describeCert(req.socket)
  );

  if (scenario === 'strict-cors' && req.headers.origin) {
    res.writeHead(403, { 'content-type': 'text/plain' });
    res.end('Origin not allowed');
    return;
  }

  if (req.method === 'OPTIONS') {
    res.writeHead(204, corsHeaders(req));
    res.end();
    return;
  }

  if (scenario === 'redirect') {
    const location = `https://127.0.0.1:${redirectTargetPort}/goose${req.url}`;
    log(`  [edge] -> 302 ${shortUrl(location)}`);
    res.writeHead(302, { ...corsHeaders(req), location });
    res.end();
    return;
  }

  proxy(req, res);
});

edge.on('upgrade', (req, socket, head) => {
  log(`  [edge] UPGRADE ${shortUrl(req.url ?? '')} origin=${req.headers.origin ?? '(none)'}`);
  if (scenario === 'strict-cors' && req.headers.origin) {
    socket.end('HTTP/1.1 403 Forbidden\r\n\r\n');
    return;
  }
  proxyUpgrade(req, socket, head);
});

await new Promise((resolve) => edge.listen(edgePort, '127.0.0.1', resolve));

if (scenario === 'redirect') {
  const target = https.createServer(tlsOptions, (req, res) => {
    const stripped = (req.url ?? '').replace(/^\/goose/, '') || '/';
    log(`  [target] ${req.method} ${shortUrl(req.url ?? '')} -> ${stripped}`);
    if (req.method === 'OPTIONS') {
      res.writeHead(204, corsHeaders(req));
      res.end();
      return;
    }
    req.url = stripped;
    proxy(req, res);
  });
  target.on('upgrade', (req, socket, head) => {
    req.url = (req.url ?? '').replace(/^\/goose/, '') || '/';
    log(`  [target] UPGRADE ${shortUrl(req.url)} origin=${req.headers.origin ?? '(none)'}`);
    proxyUpgrade(req, socket, head);
  });
  await new Promise((resolve) => target.listen(redirectTargetPort, '127.0.0.1', resolve));
}

log('\n──────────────────────────────────────────────────────────────');
log('Paste into Settings → External Backend (ACP):');
log(`  Backend base URL:      https://127.0.0.1:${edgePort}`);
log(`  Secret key:            ${secret}`);
log(`  Remote working dir:    ${workDir}`);
log(`  Cert fingerprint:      (blank = trust on first use)`);
log(`                         pin with: ${edgeFingerprint}`);
log('──────────────────────────────────────────────────────────────');
log('\nExpected result:');
if (scenario === 'plain') log('  Connect succeeds and every connection check passes.');
if (scenario === 'redirect')
  log(`  A Redirect step appears pointing at https://127.0.0.1:${redirectTargetPort}/goose.`);
if (scenario === 'mtls') {
  log('  Connects with clientCert=NONE, because the keychain has nothing issued');
  log('  by this throwaway CA. To exercise the certificate-sending path:');
  log(
    `    security import ${path.join(workDir, 'client.p12')} -k ~/Library/Keychains/login.keychain-db -P goose`
  );
  log('  then reconnect and expect clientCert=Goose Testbed Client.');
  log('  Delete it afterwards via Keychain Access ("Goose Testbed Client").');
}
if (scenario === 'strict-cors') {
  log('  Connect FAILS with "Failed to fetch", reproducing the Blox edge:');
  log('  any request carrying an Origin header is rejected.');
}
log('\nRequest log (Ctrl-C to stop):');
