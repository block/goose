/**
 * E2E-1 — binding acceptance for Zitadel desktop login / per-user isolation.
 * covers AC-2, AC-3, AC-4, AC-5
 *
 * Phase 0: written failing. Implementation in Phases 2–5/8.
 */
import { describe, expect, it } from 'vitest'

import { gatewayPlaceholder } from '../../src/index.js'

describe('E2E: gateway isolation - Goal: per-user goose instances behind Zitadel JWT', () => {
  it('GivenNoToken_WhenCallingAcp_ThenReturns401WithWwwAuthenticate', async () => {
    // covers AC-2
    expect(gatewayPlaceholder()).not.toBe('avcd-agent-gateway-not-ready')
    // Phase 8 will replace with a live gateway request:
    // const res = await fetch(`${gatewayUrl}/acp`, { method: 'POST', ... })
    // expect(res.status).toBe(401)
    // expect(res.headers.get('www-authenticate')).toMatch(/Bearer/)
  })

  it('GivenRolelessToken_WhenCallingAcp_ThenReturns403', async () => {
    // covers AC-2
    expect(gatewayPlaceholder()).not.toBe('avcd-agent-gateway-not-ready')
  })

  it('GivenExpiredToken_WhenCallingAcp_ThenReturns401', async () => {
    // covers AC-2 — auth-critical: Phase 8 repeats 10x
    expect(gatewayPlaceholder()).not.toBe('avcd-agent-gateway-not-ready')
  })

  it('GivenTwoUsers_WhenEachCreatesSessions_ThenUserBCannotSeeUserASessions', async () => {
    // covers AC-3, AC-4
    // Arrange: mint JWTs for random subA/subB with agent-access
    // Act: userA creates session via WS /acp; userB lists sessions
    // Assert: userB list is empty of userA sessions; path roots differ
    expect(gatewayPlaceholder()).not.toBe('avcd-agent-gateway-not-ready')
  })

  it('GivenLogout_WhenPosted_ThenUserGooseProcessExits', async () => {
    // covers AC-5
    expect(gatewayPlaceholder()).not.toBe('avcd-agent-gateway-not-ready')
  })
})
