export class ProvisioningError extends Error {
  readonly statusCode: number

  constructor(message: string, httpStatus?: number) {
    super(message)
    this.name = 'ProvisioningError'
    if (httpStatus === 401) {
      this.statusCode = 401
    } else if (httpStatus === 403) {
      this.statusCode = 403
    } else {
      this.statusCode = 503
    }
  }
}

export type ProvisionResult =
  | { ok: true; apiKey: string; baseUrl: string; userId: string; expiresAt: string }
  | { ok: false; statusCode: number; error: string }

type ProvisionResponseBody = {
  apiKey?: unknown
  baseUrl?: unknown
  userId?: unknown
  expiresAt?: unknown
  error?: unknown
  detail?: unknown
}

function errorMessageFromBody(body: ProvisionResponseBody, status: number): string {
  if (typeof body.error === 'string' && body.error) {
    if (typeof body.detail === 'string' && body.detail) {
      return `${body.error}: ${body.detail}`
    }
    return body.error
  }
  return `provisioning failed with status ${status}`
}

/**
 * Call the Avocado provisioning API with the caller's Zitadel access token.
 * Never logs apiKey. Network/timeout failures return `{ ok: false }` so the
 * supervisor can convert them to ProvisioningError.
 */
export async function provisionAvocadoKey(
  url: string,
  accessToken: string,
  fetchImpl: typeof fetch = fetch
): Promise<ProvisionResult> {
  let response: Response
  try {
    response = await fetchImpl(url, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${accessToken}`,
        accept: 'application/json',
      },
      body: undefined,
      signal: AbortSignal.timeout(5_000),
    })
  } catch (error) {
    const message =
      error instanceof Error
        ? error.name === 'TimeoutError' || error.name === 'AbortError'
          ? 'provisioning timed out'
          : error.message
        : 'provisioning network error'
    return { ok: false, statusCode: 503, error: message }
  }

  let body: ProvisionResponseBody = {}
  try {
    body = (await response.json()) as ProvisionResponseBody
  } catch {
    body = {}
  }

  if (!response.ok) {
    return {
      ok: false,
      statusCode: response.status,
      error: errorMessageFromBody(body, response.status),
    }
  }

  const apiKey = typeof body.apiKey === 'string' ? body.apiKey : ''
  const baseUrl = typeof body.baseUrl === 'string' ? body.baseUrl : ''
  const userId = typeof body.userId === 'string' ? body.userId : ''
  const expiresAt = typeof body.expiresAt === 'string' ? body.expiresAt : ''
  if (!apiKey || !baseUrl || !userId || !expiresAt) {
    return {
      ok: false,
      statusCode: 503,
      error: 'provisioning response missing required fields',
    }
  }

  return { ok: true, apiKey, baseUrl, userId, expiresAt }
}
