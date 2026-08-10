import { SignJWT, exportJWK, generateKeyPair } from 'jose'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import {
  ForbiddenError,
  requireAgentAccess,
  resolveInstanceKey,
} from './access.js'
import { BearerAuthError, resetJwksCacheForTests, verifyBearerToken } from './verify-bearer.js'
import { type AuthSettings } from './settings.js'
import {
  ZITADEL_ORG_CLAIM,
  ZITADEL_RESOURCE_OWNER_CLAIM,
  extractTenantId,
  resolveTenantId,
} from './tenant-context.js'
import { extractUserContext } from './user-context.js'
import { createServer, type Server } from 'node:http'
import { AddressInfo } from 'node:net'

async function mintRs256Token(opts: {
  privateKey: CryptoKey
  issuer: string
  audience: string
  sub: string
  roles?: string[]
  orgId?: string
  resourceOwner?: string
  expired?: boolean
}): Promise<string> {
  const rolesClaim =
    opts.roles && opts.roles.length > 0
      ? Object.fromEntries(opts.roles.map((r) => [r, { org: 'domain' }]))
      : undefined

  const jwt = new SignJWT({
    ...(rolesClaim ? { 'urn:zitadel:iam:org:project:roles': rolesClaim } : {}),
    ...(opts.orgId ? { [ZITADEL_ORG_CLAIM]: opts.orgId } : {}),
    ...(opts.resourceOwner
      ? { [ZITADEL_RESOURCE_OWNER_CLAIM]: opts.resourceOwner }
      : {}),
  })
    .setProtectedHeader({ alg: 'RS256', kid: 'test-key' })
    .setIssuer(opts.issuer)
    .setAudience(opts.audience)
    .setSubject(opts.sub)
    .setIssuedAt(opts.expired ? Math.floor(Date.now() / 1000) - 3600 : undefined)
    .setExpirationTime(
      opts.expired ? Math.floor(Date.now() / 1000) - 10 : '1h'
    )

  return jwt.sign(opts.privateKey)
}

describe('auth access + verify (covers AC-2, AC-6)', () => {
  let server: Server
  let issuer: string
  let privateKey: CryptoKey
  let publicJwk: Record<string, unknown>
  const projectId = 'test-project-id'

  beforeEach(async () => {
    resetJwksCacheForTests()
    const pair = await generateKeyPair('RS256')
    privateKey = pair.privateKey
    publicJwk = (await exportJWK(pair.publicKey)) as Record<string, unknown>
    publicJwk.kid = 'test-key'
    publicJwk.alg = 'RS256'
    publicJwk.use = 'sig'

    server = createServer((_req, res) => {
      res.writeHead(200, { 'content-type': 'application/json' })
      res.end(JSON.stringify({ keys: [publicJwk] }))
    })
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
    const { port } = server.address() as AddressInfo
    issuer = `http://127.0.0.1:${port}`
  })

  afterEach(async () => {
    resetJwksCacheForTests()
    delete process.env.JWT_REQUIRED
    delete process.env.TENANT_ID_CLAIM
    delete process.env.ALLOW_SUB_AS_TENANT
    await new Promise<void>((resolve, reject) =>
      server.close((err) => (err ? reject(err) : resolve()))
    )
  })

  function settings(overrides: Partial<AuthSettings> = {}): AuthSettings {
    return {
      jwtRequired: true,
      zitadelIssuer: issuer,
      zitadelProjectId: projectId,
      jwtIssuer: 'avcd',
      jwtAudience: 'avcd-agent',
      agentAccessRoleKey: 'agent-access',
      ...overrides,
    }
  }

  it('GivenValidTokenWithRole_WhenVerifying_ThenReturnsPayload', async () => {
    const token = await mintRs256Token({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'user-a',
      roles: ['agent-access'],
      orgId: 'org-1',
    })
    const payload = await verifyBearerToken(token, settings())
    expect(payload.sub).toBe('user-a')
    requireAgentAccess(payload, settings())
  })

  it('GivenExpiredToken_WhenVerifying_ThenThrows401', async () => {
    const token = await mintRs256Token({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'user-a',
      roles: ['agent-access'],
      expired: true,
    })
    await expect(verifyBearerToken(token, settings())).rejects.toBeInstanceOf(
      BearerAuthError
    )
  })

  it('GivenWrongIssuer_WhenVerifying_ThenThrows401', async () => {
    const token = await mintRs256Token({
      privateKey,
      issuer: 'http://evil.example',
      audience: projectId,
      sub: 'user-a',
      roles: ['agent-access'],
    })
    await expect(verifyBearerToken(token, settings())).rejects.toBeInstanceOf(
      BearerAuthError
    )
  })

  it('GivenWrongAudience_WhenVerifying_ThenThrows401', async () => {
    const token = await mintRs256Token({
      privateKey,
      issuer,
      audience: 'other-project',
      sub: 'user-a',
      roles: ['agent-access'],
    })
    await expect(verifyBearerToken(token, settings())).rejects.toBeInstanceOf(
      BearerAuthError
    )
  })

  it('GivenRolelessToken_WhenRequireAgentAccess_ThenThrows403', async () => {
    const token = await mintRs256Token({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'user-a',
      roles: [],
      orgId: 'org-1',
    })
    const payload = await verifyBearerToken(token, settings())
    expect(() => requireAgentAccess(payload, settings())).toThrow(ForbiddenError)
  })

  it('GivenRoleCheckMutation_WhenRoleAbsent_ThenRejects', () => {
    // mutation gate: roles.includes(roleKey) must not be inverted
    const payload = {
      sub: 'u',
      'urn:zitadel:iam:org:project:roles': { 'other-role': { x: 'y' } },
    }
    expect(() => requireAgentAccess(payload, settings())).toThrow(ForbiddenError)
    expect(() =>
      requireAgentAccess(
        {
          sub: 'u',
          'urn:zitadel:iam:org:project:roles': { 'agent-access': { x: 'y' } },
        },
        settings()
      )
    ).not.toThrow()
  })

  it('GivenProjectScopedRolesClaimOnly_WhenRequireAccess_ThenAllows', () => {
    // Zitadel emits roles under `...:project:<projectId>:roles` as well as the
    // generic key; a token carrying only the project-scoped key must pass.
    expect(() =>
      requireAgentAccess(
        {
          sub: 'u',
          [`urn:zitadel:iam:org:project:${projectId}:roles`]: {
            'agent-access': { 'org-1': 'zitadel.zitadel.avcd.ai' },
          },
        },
        settings()
      )
    ).not.toThrow()
  })

  it('GivenOrgClaim_WhenExtractTenant_ThenUsesOrgId', () => {
    expect(
      extractTenantId({
        sub: 'u',
        [ZITADEL_ORG_CLAIM]: ' org-abc ',
      })
    ).toBe('org-abc')
  })

  it('GivenOnlyResourceOwner_WhenExtractTenant_ThenUsesResourceOwner', () => {
    expect(
      extractTenantId({
        sub: 'u',
        [ZITADEL_RESOURCE_OWNER_CLAIM]: 'ro-1',
      })
    ).toBe('ro-1')
  })

  it('GivenWhitespaceOrgClaim_WhenExtractTenant_ThenFallsThrough', () => {
    expect(
      extractTenantId({
        sub: 'u',
        [ZITADEL_ORG_CLAIM]: '   ',
        [ZITADEL_RESOURCE_OWNER_CLAIM]: 'ro-2',
      })
    ).toBe('ro-2')
  })

  it('GivenJwtRequiredAndNoTenant_WhenResolveTenant_ThenThrows', () => {
    process.env.JWT_REQUIRED = 'true'
    expect(() => resolveTenantId({ sub: 'u' }, true)).toThrow()
  })

  it('GivenValidClaims_WhenResolveInstanceKey_ThenIncludesTenantAndSub', async () => {
    const token = await mintRs256Token({
      privateKey,
      issuer,
      audience: projectId,
      sub: 'userA',
      roles: ['agent-access'],
      orgId: 'tenant1',
    })
    const payload = await verifyBearerToken(token, settings())
    const key = resolveInstanceKey(payload, settings())
    expect(key.tenantId).toBe('tenant1')
    expect(key.sub).toBe('userA')
    expect(key.key).toBe('tenant1/userA')
    // mutation: must not drop tenant
    expect(key.key).not.toBe(key.sub)
  })

  it('GivenPathTraversalSub_WhenResolveInstanceKey_ThenRejects', () => {
    expect(() =>
      resolveInstanceKey(
        {
          sub: '../x',
          [ZITADEL_ORG_CLAIM]: 'tenant1',
          'urn:zitadel:iam:org:project:roles': { 'agent-access': { a: 'b' } },
        },
        settings()
      )
    ).toThrow(BearerAuthError)
  })

  it('GivenZitadelRoleObject_WhenExtractUserContext_ThenParsesRoleKeys', () => {
    const ctx = extractUserContext({
      sub: 'u1',
      'urn:zitadel:iam:org:project:roles': {
        'agent-access': { 'org-1': 'avcd.ai' },
        admin: { 'org-1': 'avcd.ai' },
      },
    })
    expect(ctx.userId).toBe('u1')
    expect(ctx.roles).toEqual(['admin', 'agent-access'])
  })
})
