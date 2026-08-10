/**
 * E2E — avocado per-user LLM credential provisioning.
 * covers AC-1, AC-8
 *
 * Expected FAILING until Phase 4 (gateway injection) is complete.
 */
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'node:http'
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
}): Promise<string> {
  const rolesClaim =
    opts.roles && opts.roles.length > 0
      ? Object.fromEntries(opts.roles.map((r) => [r, { o: 'd' }]))
      : undefined
  return new SignJWT({
    ...(rolesClaim ? { 'urn:zitadel:iam:org:project:roles': rolesClaim } : {}),
    [ZITADEL_ORG_CLAIM]: 'tenant1',
  })
    .setProtectedHeader({ alg: 'RS256', kid: 'avocado-e2e' })
    .setIssuer(opts.issuer)
    .setAudience(opts.audience)
    .setSubject(opts.sub)
    .setExpirationTime('1h')
    .sign(opts.privateKey)
}

async function writeEnvEchoFakeGoose(dir: string): Promise<string> {
  const bin = path.join(dir, 'fake-goose.js')
  const script = `
const http = require('http');
const args = process.argv.slice(2);
const port = Number(args[args.indexOf('--port') + 1]);
const secret = process.env.GOOSE_SERVER__SECRET_KEY;
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://127.0.0.1');
  if (url.pathname === '/status') { res.writeHead(200); res.end('ok'); return; }
  if (url.pathname === '/acp' && req.method === 'POST') {
    if (req.headers['x-secret-key'] !== secret) { res.writeHead(401); res.end('no'); return; }
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify({
      env: {
        AVOCADO_API_KEY: process.env.AVOCADO_API_KEY || null,
        AVOCADO_HOST: process.env.AVOCADO_HOST || null,
        GOOSE_PROVIDER: process.env.GOOSE_PROVIDER || null,
        OPENROUTER_API_KEY: process.env.OPENROUTER_API_KEY || null,
      }
    }));
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

describe('E2E: Avocado provisioning - Goal: each user gets their own LLM credential', () => {
  let jwksServer: Server
  let provisionServer: Server
  let issuer: string
  let provisionUrl: string
  let privateKey: CryptoKey
  let supervisor: InstanceSupervisor
  let gatewayBase: string
  let gatewayServer: Server
  const projectId = 'avocado-e2e-project'
  let rejectProvision = false

  beforeAll(async () => {
    resetJwksCacheForTests()
    const pair = await generateKeyPair('RS256')
    privateKey = pair.privateKey
    const publicJwk = (await exportJWK(pair.publicKey)) as Record<string, unknown>
    publicJwk.kid = 'avocado-e2e'
    publicJwk.alg = 'RS256'
    publicJwk.use = 'sig'

    jwksServer = createServer((_req, res) => {
      res.writeHead(200, { 'content-type': 'application/json' })
      res.end(JSON.stringify({ keys: [publicJwk] }))
    })
    await new Promise<void>((r) => jwksServer.listen(0, '127.0.0.1', r))
    issuer = `http://127.0.0.1:${(jwksServer.address() as AddressInfo).port}`

    provisionServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
      const url = new URL(req.url ?? '/', 'http://127.0.0.1')
      if (url.pathname === '/keys/provision' && req.method === 'POST') {
        if (rejectProvision) {
          res.writeHead(403, { 'content-type': 'application/json' })
          res.end(JSON.stringify({ error: 'forbidden', detail: 'missing agent-access role' }))
          return
        }
        const auth = req.headers.authorization ?? ''
        const token = auth.startsWith('Bearer ') ? auth.slice('Bearer '.length) : ''
        // Derive a stable key from the JWT payload.sub (middle segment) for test determinism.
        let sub = 'unknown'
        try {
          const payload = JSON.parse(
            Buffer.from(token.split('.')[1] ?? '', 'base64url').toString('utf8')
          ) as { sub?: string }
          sub = payload.sub ?? 'unknown'
        } catch {
          /* ignore */
        }
        res.writeHead(200, { 'content-type': 'application/json' })
        res.end(
          JSON.stringify({
            apiKey: `sk-test-${sub}`,
            baseUrl: 'https://dev.avocado.tech/llm',
            userId: `tenant1:${sub}`,
            expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
          })
        )
        return
      }
      res.writeHead(404)
      res.end('no')
    })
    await new Promise<void>((r) => provisionServer.listen(0, '127.0.0.1', r))
    provisionUrl = `http://127.0.0.1:${(provisionServer.address() as AddressInfo).port}`

    const dir = await mkdtemp(path.join(tmpdir(), 'avocado-e2e-goose-'))
    const bin = await writeEnvEchoFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'avocado-e2e-data-'))
    // Phase 4 will honor avocadoProvisionUrl / avocadoHost; until then these are ignored
    // and the assertions below FAIL (expected RED for Phase 0).
    supervisor = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: {
        dataRoot,
        gooseProvider: 'openrouter',
        providerApiKeyEnv: 'OPENROUTER_API_KEY',
        providerApiKey: 'sk-shared-must-not-leak',
      },
      readinessTimeoutMs: 5_000,
      // Phase 4 will honor these options; until then assertions FAIL (expected RED).
      ...({
        avocadoProvisionUrl: `${provisionUrl}/keys/provision`,
        avocadoHost: 'https://dev.avocado.tech/llm',
      } as object),
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
    await new Promise<void>((r) => provisionServer.close(() => r()))
    await new Promise<void>((r) => jwksServer.close(() => r()))
    resetJwksCacheForTests()
  })

  it('GivenTwoDistinctUsers_WhenEachStartsAnInstance_ThenEachChildHasItsOwnKeyAndNoSharedKey', async () => {
    // covers AC-1
    rejectProvision = false
    const tokenA = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'user-a',
      roles: ['agent-access'],
    })
    const tokenB = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'user-b',
      roles: ['agent-access'],
    })

    const resA = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${tokenA}`, 'content-type': 'application/json' },
      body: '{}',
    })
    expect(resA.status).toBe(200)
    const bodyA = (await resA.json()) as {
      env: {
        AVOCADO_API_KEY: string | null
        AVOCADO_HOST: string | null
        GOOSE_PROVIDER: string | null
        OPENROUTER_API_KEY: string | null
      }
    }

    const resB = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${tokenB}`, 'content-type': 'application/json' },
      body: '{}',
    })
    expect(resB.status).toBe(200)
    const bodyB = (await resB.json()) as {
      env: {
        AVOCADO_API_KEY: string | null
        AVOCADO_HOST: string | null
        GOOSE_PROVIDER: string | null
        OPENROUTER_API_KEY: string | null
      }
    }

    expect(bodyA.env.AVOCADO_API_KEY).toBe('sk-test-user-a')
    expect(bodyB.env.AVOCADO_API_KEY).toBe('sk-test-user-b')
    expect(bodyA.env.AVOCADO_API_KEY).not.toBe(bodyB.env.AVOCADO_API_KEY)
    expect(bodyA.env.GOOSE_PROVIDER).toBe('avocado')
    expect(bodyA.env.AVOCADO_HOST).toBe('https://dev.avocado.tech/llm')
    expect(bodyA.env.OPENROUTER_API_KEY).toBeNull()
    expect(bodyB.env.OPENROUTER_API_KEY).toBeNull()
  })

  it('GivenProvisioningRejectsWith403_WhenStartingAnInstance_ThenSpawnFailsAndNoChildSurvives', async () => {
    // covers AC-8
    rejectProvision = true
    const token = await mint({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'user-blocked',
      roles: ['agent-access'],
    })
    const res = await fetch(`${gatewayBase}/acp`, {
      method: 'POST',
      headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
      body: '{}',
    })
    expect(res.status).toBeGreaterThanOrEqual(400)
    expect(supervisor.list().some((i) => i.sub === 'user-blocked')).toBe(false)
  })
})
