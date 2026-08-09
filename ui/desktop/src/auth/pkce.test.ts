import { describe, expect, it } from 'vitest'

import { buildAuthorizeUrl, createPkcePair } from './pkce'

describe('pkce', () => {
  it('GivenCreatePkcePair_WhenCalledTwice_ThenValuesDiffer', () => {
    const a = createPkcePair()
    const b = createPkcePair()
    expect(a.codeVerifier).not.toBe(b.codeVerifier)
    expect(a.state).not.toBe(b.state)
    expect(a.codeChallenge).toMatch(/^[A-Za-z0-9_-]+$/)
  })

  it('GivenAuthorizeParams_WhenBuilding_ThenIncludesPkceFields', () => {
    const url = buildAuthorizeUrl('https://zitadel.avcd.ai', {
      clientId: 'client',
      redirectUri: 'http://127.0.0.1:47821/callback',
      scope: 'openid email',
      state: 'abc',
      codeChallenge: 'challenge',
    })
    const parsed = new URL(url)
    expect(parsed.pathname).toBe('/oauth/v2/authorize')
    expect(parsed.searchParams.get('code_challenge_method')).toBe('S256')
    expect(parsed.searchParams.get('code_challenge')).toBe('challenge')
    expect(parsed.searchParams.get('state')).toBe('abc')
  })
})
