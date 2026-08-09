import { createHash, randomBytes } from 'node:crypto'

export type PkcePair = {
  codeVerifier: string
  codeChallenge: string
  state: string
}

function base64Url(buf: Buffer): string {
  return buf
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/g, '')
}

export function createPkcePair(): PkcePair {
  const codeVerifier = base64Url(randomBytes(32))
  const codeChallenge = base64Url(createHash('sha256').update(codeVerifier).digest())
  const state = base64Url(randomBytes(16))
  return { codeVerifier, codeChallenge, state }
}

export function buildAuthorizeUrl(
  issuer: string,
  params: {
    clientId: string
    redirectUri: string
    scope: string
    state: string
    codeChallenge: string
  }
): string {
  const url = new URL(`${issuer.replace(/\/$/, '')}/oauth/v2/authorize`)
  url.searchParams.set('response_type', 'code')
  url.searchParams.set('client_id', params.clientId)
  url.searchParams.set('redirect_uri', params.redirectUri)
  url.searchParams.set('scope', params.scope)
  url.searchParams.set('state', params.state)
  url.searchParams.set('code_challenge', params.codeChallenge)
  url.searchParams.set('code_challenge_method', 'S256')
  return url.toString()
}
