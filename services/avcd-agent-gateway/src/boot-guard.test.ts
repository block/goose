import { describe, expect, it } from 'vitest'

import { assertFailClosedBootEnv, BootConfigError } from './boot-guard.js'

describe('assertFailClosedBootEnv (covers AC-5, AC-7)', () => {
  it('GivenNoAvocadoProvisionUrl_WhenBooting_ThenThrows', () => {
    expect(() =>
      assertFailClosedBootEnv({
        AVCD_AGENT_DATA_ROOT: '/tmp/avcd-agent-data',
        OPENROUTER_API_KEY: 'sk-shared',
      })
    ).toThrow(BootConfigError)
    expect(() =>
      assertFailClosedBootEnv({
        AVCD_AGENT_DATA_ROOT: '/tmp/avcd-agent-data',
        OPENROUTER_API_KEY: 'sk-shared',
      })
    ).toThrow(/AVOCADO_PROVISION_URL/)
  })

  it('GivenProvisionUrlAndDataRoot_WhenBooting_ThenPasses', () => {
    expect(() =>
      assertFailClosedBootEnv({
        AVCD_AGENT_DATA_ROOT: '/tmp/avcd-agent-data',
        AVOCADO_PROVISION_URL: 'http://127.0.0.1:3001/keys/provision',
      })
    ).not.toThrow()
  })

  it('GivenProdWithoutJwtRequired_ThenBootRefused', () => {
    // covers AC-7
    expect(() =>
      assertFailClosedBootEnv({
        AVCD_AGENT_DATA_ROOT: '/tmp/avcd-agent-data',
        AVOCADO_PROVISION_URL: 'http://127.0.0.1:3001/keys/provision',
        NODE_ENV: 'production',
        JWT_REQUIRED: 'false',
      })
    ).toThrow(/JWT_REQUIRED/)
  })

  it('GivenProdWithJwtRequired_WhenBooting_ThenPasses', () => {
    expect(() =>
      assertFailClosedBootEnv({
        AVCD_AGENT_DATA_ROOT: '/tmp/avcd-agent-data',
        AVOCADO_PROVISION_URL: 'http://127.0.0.1:3001/keys/provision',
        NODE_ENV: 'production',
        JWT_REQUIRED: 'true',
      })
    ).not.toThrow()
  })
})
