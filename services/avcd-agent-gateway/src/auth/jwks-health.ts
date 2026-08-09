import { zitadelJwksUrl } from './verify-bearer.js'

export type JwksHealthResult =
  | { ok: true; jwksUrl: string }
  | { ok: false; jwksUrl?: string; reason: string }

export async function checkJwksReachability(
  issuer: string | undefined,
  fetchImpl: typeof fetch = fetch
): Promise<JwksHealthResult> {
  if (!issuer) {
    return { ok: false, reason: 'zitadel issuer not configured' }
  }

  const jwksUrl = zitadelJwksUrl(issuer)

  try {
    const response = await fetchImpl(jwksUrl, {
      method: 'GET',
      headers: { accept: 'application/json' },
      signal: AbortSignal.timeout(5_000),
    })

    if (!response.ok) {
      return {
        ok: false,
        jwksUrl,
        reason: `JWKS fetch failed with status ${response.status}`,
      }
    }

    const body = (await response.json()) as { keys?: unknown[] }
    if (!Array.isArray(body.keys) || body.keys.length === 0) {
      return { ok: false, jwksUrl, reason: 'JWKS response missing keys' }
    }

    return { ok: true, jwksUrl }
  } catch (error) {
    return {
      ok: false,
      jwksUrl,
      reason: error instanceof Error ? error.message : 'JWKS fetch failed',
    }
  }
}
