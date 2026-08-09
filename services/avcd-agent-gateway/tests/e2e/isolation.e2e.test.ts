/**
 * E2E-1 — binding acceptance for Zitadel desktop login / per-user isolation.
 * covers AC-2, AC-3, AC-4, AC-5
 */
import { createServer, type Server } from 'node:http'
import { AddressInfo } from 'node:net'
import { mkdtemp, writeFile, chmod } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { SignJWT, exportJWK, generateKeyPair } from 'jose'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'

import { listenGateway } from '../../src/proxy/server.js'
import { InstanceSupervisor } from '../../src/instance/supervisor.js'
import { ZITADEL_ORG_CLAIM } from '../../src/auth/tenant-context.js'
import { resetJwksCacheForTests, type AuthSettings } from '../../src/auth/index.js'

async function mint(opts: {
  privateKey: CryptoKey
  issuer: string
  audience: string
  sub: string
  roles?: string[]
  expired?: boolean
}): Promise<string> {
  const rolesClaim =
    opts.roles && opts.roles.length > 0
      ? Object.fromEntries(opts.roles.map((r) => [r, { o: 'd' }]))
      : undefined
  return new SignJWT({
    ...(rolesClaim ? { 'urn:zitadel:iam:org:project:roles': rolesClaim } : {}),
    [ZITADEL_ORG_CLAIM]: 'tenant1',
  })
    .setProtectedHeader({ alg: 'RS256', kid: 'e2e' })
    .setIssuer(opts.issuer)
    .setAudience(opts.audience)
    .setSubject(opts.sub)
    .setIssuedAt(opts.expired ? Math.floor(Date.now() / 1000) - 3600 : undefined)
    .setExpirationTime(opts.expired ? Math.floor(Date.now() / 1000) - 10 : '1h')
    .sign(opts.privateKey)
}

async function writeFakeGoose(dir: string): Promise<string> {
  const bin = path.join(dir, 'fake-goose.js')
  const script = `
const http = require('http');
const args = process.argv.slice(2);
const port = Number(args[args.indexOf('--port') + 1]);
const secret = process.env.GOOSE_SERVER__SECRET_KEY;
const pathRoot = process.env.GOOSE_PATH_ROOT;
const sessions = new Map();
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  if (url.pathname === '/status') { res.writeHead(200); res.end('ok'); return; }
  if (url.pathname === '/acp' && req.method === 'POST') {
    if (req.headers['x-secret-key'] !== secret) { res.writeHead(401); res.end('no'); return; }
    let body = '';
    req.on('data', (c) => body += c);
    req.on('end', () => {
      try {
        const parsed = JSON.parse(body || '{}');
        if (parsed.op === 'create') {
          const id = 's-' + Math.random().toString(16).slice(2);
          sessions.set(id, { id, pathRoot });
          res.writeHead(200, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ id, pathRoot }));
          return;
        }
        if (parsed.op === 'list') {
          res.writeHead(200, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ sessions: [...sessions.values()] }));
          return;
        }
      } catch {}
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ ok: true, pathRoot }));
    });
    return;
  }
  res.writeHead(404); res.end('no');
});
server.listen(port, '127.0.0.1');
process.on('SIGTERM', () => server.close(() => process.exit(0)));
`
  await writeFile(bin, script, 'utf8')
  await chmod(bin, 0o755)
  return bin
}

describe('E2E: gateway isolation - Goal: per-user goose instances behind Zitadel JWT', () => {
  let jwksServer: Server
  let issuer: string
  let privateKey: CryptoKey
  let supervisor: InstanceSupervisor
  let gatewayBase: string
  let gatewayServer: Server
  const projectId = 'e2e-project'

  beforeAll(async () => {
    resetJwksCacheForTests()
    const pair = await generateKeyPair('RS256')
    privateKey = pair.privateKey
    const publicJwk = (await exportJWK(pair.publicKey)) as Record<string, unknown>
    publicJwk.kid = 'e2e'
    publicJwk.alg = 'RS256'
    publicJwk.use = 'sig'

    jwksServer = createServer((_req, res) => {
      res.writeHead(200, { 'content-type': 'application/json' })
      res.end(JSON.stringify({ keys: [publicJwk] }))
    })
    await new Promise<void>((r) => jwksServer.listen(0, '127.0.0.1', r))
    issuer = `http://127.0.0.1:${(jwksServer.address() as AddressInfo).port}`

    const dir = await mkdtemp(path.join(tmpdir(), 'e2e-goose-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'e2e-data-'))
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
  }, 30_000)

  afterAll(async () => {
    await supervisor.stopAll()
    await new Promise<void>((r) => gatewayServer.close(() => r()))
    await new Promise<void>((r) => jwksServer.close(() => r()))
    resetJwksCacheForTests()
  })

  it('GivenNoToken_WhenCallingAcp_ThenReturns401WithWwwAuthenticate', async () => {
    // covers AC-2
    const res = await fetch(`${gatewayBase}/acp`, { method: 'POST', body: '{}' })
    expect(res.status).toBe(401)
    expect(res.headers.get('www-authenticate')).toMatch(/Bearer/)
  })

  it('GivenRolelessToken_WhenCallingAcp_ThenReturns403', async () => {
    // covers AC-2
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'roleless',
      roles: [],
    })
    const res = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
      body: '{}',
    })
    expect(res.status).toBe(403)
  })

  it('GivenExpiredToken_WhenCallingAcp_ThenReturns401', async () => {
    // covers AC-2 — auth-critical: Phase 8 repeats 10x
    for (let i = 0; i < 10; i++) {
      const token = await mint({
        privateKey,
        issuer,
        audience: projectId,
        sub: `expired-${i}`,
        roles: ['agent-access'],
        expired: true,
      })
      const res = await fetch(`${gatewayBase}/acp`, {
        method: 'POST',
        headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
        body: '{}',
      })
      expect(res.status).toBe(401)
    }
  })

  it('GivenTwoUsers_WhenEachCreatesSessions_ThenUserBCannotSeeUserASessions', async () => {
    // covers AC-3, AC-4
    const tokenA = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userA',
      roles: ['agent-access'],
    })
    const tokenB = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userB',
      roles: ['agent-access'],
    })

    const createA = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${tokenA}`, 'content-type': 'application/json' },
      body: JSON.stringify({ op: 'create' }),
    })
    expect(createA.status).toBe(200)
    const sessionA = (await createA.json()) as { id: string; pathRoot: string }

    const listB = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${tokenB}`, 'content-type': 'application/json' },
      body: JSON.stringify({ op: 'list' }),
    })
    expect(listB.status).toBe(200)
    const bodyB = (await listB.json()) as { sessions: Array<{ id: string; pathRoot: string }> }
    expect(bodyB.sessions.find((s) => s.id === sessionA.id)).toBeUndefined()

    const createB = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${tokenB}`, 'content-type': 'application/json' },
      body: JSON.stringify({ op: 'create' }),
    })
    const sessionB = (await createB.json()) as { pathRoot: string }
    expect(sessionA.pathRoot).not.toBe(sessionB.pathRoot)
    expect(sessionA.pathRoot).toContain('/userA')
    expect(sessionB.pathRoot).toContain('/userB')
  })

  it('GivenLogout_WhenPosted_ThenUserGooseProcessExits', async () => {
    // covers AC-5
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userLogout',
      roles: ['agent-access'],
    })
    await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
      body: '{}',
    })
    expect(supervisor.list().some((i) => i.sub === 'userLogout')).toBe(true)
    const res = await fetch(`${gatewayBase}/auth/logout`, {
      method: 'POST',
      headers: { authorization: `Bearer ${token}` },
    })
    expect(res.status).toBe(200)
    expect(supervisor.list().some((i) => i.sub === 'userLogout')).toBe(false)
  })
})
