import { createServer, type Server } from 'node:http'
import { AddressInfo } from 'node:net'
import { SignJWT, exportJWK, generateKeyPair } from 'jose'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import WebSocket from 'ws'

import { listenGateway } from './server.js'
import { InstanceSupervisor } from '../instance/supervisor.js'
import { ZITADEL_ORG_CLAIM } from '../auth/tenant-context.js'
import { resetJwksCacheForTests, type AuthSettings } from '../auth/index.js'
import { mkdtemp, writeFile, chmod } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'

async function mint(opts: {
  privateKey: CryptoKey
  issuer: string
  audience: string
  sub: string
  roles?: string[]
}): Promise<string> {
  const rolesClaim =
    opts.roles && opts.roles.length > 0
      ? Object.fromEntries(opts.roles.map((r) => [r, { o: 'd' }]))
      : undefined
  return new SignJWT({
    ...(rolesClaim ? { 'urn:zitadel:iam:org:project:roles': rolesClaim } : {}),
    [ZITADEL_ORG_CLAIM]: 'tenant1',
  })
    .setProtectedHeader({ alg: 'RS256', kid: 'k1' })
    .setIssuer(opts.issuer)
    .setAudience(opts.audience)
    .setSubject(opts.sub)
    .setExpirationTime('1h')
    .sign(opts.privateKey)
}

async function writeFakeGoose(dir: string): Promise<string> {
  const bin = path.join(dir, 'fake-goose.js')
  const script = `
const http = require('http');
const crypto = require('crypto');
const args = process.argv.slice(2);
const port = Number(args[args.indexOf('--port') + 1]);
const secret = process.env.GOOSE_SERVER__SECRET_KEY;
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  const token = url.searchParams.get('token');
  const headerSecret = req.headers['x-secret-key'];
  if (url.pathname === '/status') { res.writeHead(200); res.end('ok'); return; }
  if (url.pathname === '/acp' && req.method === 'POST') {
    if (headerSecret !== secret) { res.writeHead(401); res.end('no'); return; }
    const origin = req.headers.origin || '';
    const accept = req.headers.accept || '';
    res.setHeader('acp-connection-id', 'conn-1');
    if (accept.includes('text/event-stream')) {
      res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' });
      res.write('data: one\\n\\n');
      setTimeout(() => { res.write('data: two\\n\\n'); res.end(); }, 30);
      return;
    }
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify({ ok: true, origin, accept, token: token || null }));
    return;
  }
  res.writeHead(404); res.end('no');
});
server.on('upgrade', (req, socket) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  if (url.searchParams.get('token') !== secret) {
    socket.write('HTTP/1.1 401 Unauthorized\\r\\n\\r\\n');
    socket.destroy();
    return;
  }
  if (req.headers.origin) {
    socket.write('HTTP/1.1 403 Forbidden\\r\\n\\r\\n');
    socket.destroy();
    return;
  }
  const key = req.headers['sec-websocket-key'];
  const accept = crypto
    .createHash('sha1')
    .update(key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11')
    .digest('base64');
  socket.write(
    'HTTP/1.1 101 Switching Protocols\\r\\nUpgrade: websocket\\r\\nConnection: Upgrade\\r\\nSec-WebSocket-Accept: ' +
      accept +
      '\\r\\n\\r\\n'
  );
  const payload = Buffer.from(JSON.stringify({ type: 'hello' }));
  const frame = Buffer.alloc(2 + payload.length);
  frame[0] = 0x81;
  frame[1] = payload.length;
  payload.copy(frame, 2);
  socket.write(frame);
});
server.listen(port, '127.0.0.1');
process.on('SIGTERM', () => server.close(() => process.exit(0)));
`
  await writeFile(bin, script, 'utf8')
  await chmod(bin, 0o755)
  return bin
}

describe('gateway proxy (covers AC-2, AC-4, AC-5)', () => {
  let jwksServer: Server
  let issuer: string
  let privateKey: CryptoKey
  let publicJwk: Record<string, unknown>
  let supervisor: InstanceSupervisor
  let gatewayBase: string
  let gatewayServer: Server
  const projectId = 'proj-1'
  const closers: Array<() => Promise<void>> = []

  beforeEach(async () => {
    resetJwksCacheForTests()
    const pair = await generateKeyPair('RS256')
    privateKey = pair.privateKey
    publicJwk = (await exportJWK(pair.publicKey)) as Record<string, unknown>
    publicJwk.kid = 'k1'
    publicJwk.alg = 'RS256'
    publicJwk.use = 'sig'

    jwksServer = createServer((_req, res) => {
      res.writeHead(200, { 'content-type': 'application/json' })
      res.end(JSON.stringify({ keys: [publicJwk] }))
    })
    await new Promise<void>((r) => jwksServer.listen(0, '127.0.0.1', r))
    issuer = `http://127.0.0.1:${(jwksServer.address() as AddressInfo).port}`

    const dir = await mkdtemp(path.join(tmpdir(), 'gw-proxy-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-pdata-'))
    supervisor = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 5_000,
    })
    const settings: AuthSettings = {
      jwtRequired: true,
      zitadelIssuer: issuer,
      zitadelProjectId: projectId,
      jwtIssuer: 'avcd',
      jwtAudience: 'avcd-agent',
      agentAccessRoleKey: 'agent-access',
    }
    const listened = await listenGateway({ supervisor, settings })
    gatewayServer = listened.server
    gatewayBase = listened.baseUrl
    closers.push(async () => {
      await supervisor.stopAll()
      await new Promise<void>((r) => gatewayServer.close(() => r()))
      await new Promise<void>((r) => jwksServer.close(() => r()))
      resetJwksCacheForTests()
    })
  })

  afterEach(async () => {
    while (closers.length) {
      await closers.pop()!()
    }
  })

  it('GivenNoToken_WhenPostAcp_Then401WithWwwAuthenticate', async () => {
    const res = await fetch(`${gatewayBase}/acp`, { method: 'POST', body: '{}' })
    expect(res.status).toBe(401)
    expect(res.headers.get('www-authenticate')).toMatch(/Bearer/)
  })

  it('GivenRolelessToken_WhenPostAcp_Then403', async () => {
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'u1',
      roles: [],
    })
    const res = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
      body: '{}',
    })
    expect(res.status).toBe(403)
  })

  it('GivenValidToken_WhenPostAcp_ThenForwardsAcceptAndOmitsOriginUpstream', async () => {
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userA',
      roles: ['agent-access'],
    })
    const res = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${token}`,
        'content-type': 'application/json',
        accept: 'application/json',
        origin: 'https://evil.example',
      },
      body: '{}',
    })
    expect(res.status).toBe(200)
    const body = (await res.json()) as { origin: string; accept: string; token: string | null }
    expect(body.origin).toBe('')
    expect(body.accept).toBe('application/json')
    expect(body.token).toBeNull()
    expect(res.headers.get('acp-connection-id')).toBe('conn-1')
  })

  it('GivenSseAccept_WhenGetAcp_ThenChunksArriveUnbuffered', async () => {
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userA',
      roles: ['agent-access'],
    })
    const res = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${token}`,
        accept: 'text/event-stream',
        'content-type': 'application/json',
      },
      body: '{}',
    })
    expect(res.status).toBe(200)
    const text = await res.text()
    expect(text).toContain('data: one')
    expect(text).toContain('data: two')
  })

  it('GivenValidToken_WhenWsUpgrade_ThenConnectsWithInternalSecret', async () => {
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userA',
      roles: ['agent-access'],
    })
    const wsUrl = gatewayBase.replace('http', 'ws') + `/acp?token=${encodeURIComponent(token)}`
    const msg = await new Promise<string>((resolve, reject) => {
      const ws = new WebSocket(wsUrl, { origin: 'https://should-be-stripped.example' })
      const t = setTimeout(() => reject(new Error('ws timeout')), 5_000)
      ws.on('message', (data) => {
        clearTimeout(t)
        resolve(String(data))
        ws.close()
      })
      ws.on('error', (err) => {
        clearTimeout(t)
        reject(err)
      })
    })
    expect(JSON.parse(msg).type).toBe('hello')
  })

  it('GivenLogout_WhenPosted_ThenInstanceStops', async () => {
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userA',
      roles: ['agent-access'],
    })
    // warm instance
    await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${token}`,
        'content-type': 'application/json',
      },
      body: '{}',
    })
    expect(supervisor.list().length).toBe(1)
    const res = await fetch(`${gatewayBase}/auth/logout`, {
      method: 'POST',
      headers: { authorization: `Bearer ${token}` },
    })
    expect(res.status).toBe(200)
    expect(supervisor.list().length).toBe(0)
  })
})
