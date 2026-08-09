import { createRemoteJWKSet, jwtVerify, type JWTPayload } from 'jose'

import { type AuthSettings } from './settings.js'

export const JWT_CLOCK_TOLERANCE_SECONDS = 5
export const JWKS_TIMEOUT_MS = 5_000
export const JWKS_COOLDOWN_MS = 30_000

export class BearerAuthError extends Error {
  readonly statusCode: number
  readonly wwwAuthenticate: string

  constructor(message: string, statusCode = 401) {
    super(message)
    this.name = 'BearerAuthError'
    this.statusCode = statusCode
    this.wwwAuthenticate =
      statusCode === 401 ? 'Bearer error="invalid_token"' : 'Bearer'
  }
}

export function extractBearerToken(authorization: string | null | undefined): string {
  if (!authorization?.startsWith('Bearer ')) {
    throw new BearerAuthError('Missing Authorization header')
  }

  const token = authorization.slice('Bearer '.length).trim()
  if (!token || token.split('.').length !== 3) {
    throw new BearerAuthError('Invalid bearer token')
  }

  return token
}

/** Extract token from Authorization Bearer header or ?token= query (ACP WS). */
export function extractAccessToken(
  authorization: string | null | undefined,
  queryToken: string | null | undefined
): string {
  if (authorization?.startsWith('Bearer ')) {
    return extractBearerToken(authorization)
  }
  const fromQuery = queryToken?.trim()
  if (fromQuery && fromQuery.split('.').length === 3) {
    return fromQuery
  }
  throw new BearerAuthError('Missing access token')
}

const jwksCache = new Map<string, ReturnType<typeof createRemoteJWKSet>>()

function getJwks(url: string) {
  const cached = jwksCache.get(url)
  if (cached) return cached
  const jwks = createRemoteJWKSet(new URL(url), {
    timeoutDuration: JWKS_TIMEOUT_MS,
    cooldownDuration: JWKS_COOLDOWN_MS,
  })
  jwksCache.set(url, jwks)
  return jwks
}

/** Test-only: clear JWKS cache between cases. */
export function resetJwksCacheForTests(): void {
  jwksCache.clear()
}

function baseVerifyOptions(): Parameters<typeof jwtVerify>[2] {
  return {
    clockTolerance: JWT_CLOCK_TOLERANCE_SECONDS,
  }
}

async function verifyZitadelToken(
  token: string,
  settings: AuthSettings
): Promise<JWTPayload> {
  const issuer = settings.zitadelIssuer
  if (!issuer) {
    throw new BearerAuthError('Zitadel auth is not configured')
  }

  const verifyOptions: Parameters<typeof jwtVerify>[2] = {
    ...baseVerifyOptions(),
    issuer,
  }
  if (settings.zitadelProjectId) {
    // Zitadel adds the raw project ID to aud when the client requests
    // urn:zitadel:iam:org:project:id:{id}:aud — validate the bare ID.
    verifyOptions.audience = settings.zitadelProjectId
  }

  const { payload } = await jwtVerify(
    token,
    getJwks(`${issuer.replace(/\/$/, '')}/oauth/v2/keys`),
    verifyOptions
  )
  return payload
}

async function verifyHs256Token(
  token: string,
  settings: AuthSettings
): Promise<JWTPayload> {
  if (!settings.jwtSecret) {
    throw new BearerAuthError('HS256 auth is not configured')
  }

  const secret = new TextEncoder().encode(settings.jwtSecret)
  const { payload } = await jwtVerify(token, secret, {
    ...baseVerifyOptions(),
    issuer: settings.jwtIssuer,
    audience: settings.jwtAudience,
    algorithms: ['HS256'],
  })
  return payload
}

export async function verifyBearerToken(
  token: string,
  settings: AuthSettings
): Promise<JWTPayload> {
  const errors: string[] = []

  if (settings.zitadelIssuer) {
    try {
      return await verifyZitadelToken(token, settings)
    } catch (error) {
      errors.push(error instanceof Error ? error.message : 'Zitadel validation failed')
    }
  }

  if (settings.jwtSecret) {
    try {
      return await verifyHs256Token(token, settings)
    } catch (error) {
      errors.push(error instanceof Error ? error.message : 'HS256 validation failed')
    }
  }

  throw new BearerAuthError(errors[0] ?? 'No authentication method configured')
}

export function zitadelJwksUrl(issuer: string): string {
  return `${issuer.replace(/\/$/, '')}/oauth/v2/keys`
}
