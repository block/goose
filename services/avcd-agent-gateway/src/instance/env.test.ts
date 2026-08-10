import { describe, expect, it } from 'vitest'

import { buildInstanceArgs, buildInstanceEnv, InstanceEnvError } from './env.js'

describe('buildInstanceEnv (covers AC-3 isolation env contract)', () => {
  const key = { tenantId: 'tenant1', sub: 'userA', key: 'tenant1/userA' }

  it('GivenAbsoluteRoot_WhenBuilding_ThenSetsPathRootAndKeyringOff', () => {
    const built = buildInstanceEnv(key, { dataRoot: '/var/lib/avcd-agent' }, {})
    expect(built.pathRoot).toBe('/var/lib/avcd-agent/tenant1/userA')
    expect(built.env.GOOSE_PATH_ROOT).toBe(built.pathRoot)
    expect(built.env.GOOSE_DISABLE_KEYRING).toBe('true')
    expect(built.env.GOOSE_SERVER__SECRET_KEY).toMatch(/^[a-f0-9]{64}$/)
    expect(built.env).not.toHaveProperty('GOOSE_OAUTH_CALLBACK_PORT')
  })

  it('GivenRelativeRoot_WhenBuilding_ThenThrows', () => {
    expect(() =>
      buildInstanceEnv(key, { dataRoot: 'relative/root' }, {})
    ).toThrow(InstanceEnvError)
  })

  it('GivenEmptyRoot_WhenBuilding_ThenThrows', () => {
    expect(() => buildInstanceEnv(key, { dataRoot: '  ' }, {})).toThrow(
      InstanceEnvError
    )
  })

  it('GivenTwoCalls_WhenBuilding_ThenSecretsDiffer', () => {
    const a = buildInstanceEnv(key, { dataRoot: '/tmp/avcd-agent-data' }, {})
    const b = buildInstanceEnv(key, { dataRoot: '/tmp/avcd-agent-data' }, {})
    expect(a.secretKey).not.toBe(b.secretKey)
  })

  it('GivenParentHasOauthCallbackPort_WhenBuilding_ThenPortAbsent', () => {
    const built = buildInstanceEnv(
      key,
      { dataRoot: '/tmp/avcd-agent-data' },
      {
        GOOSE_OAUTH_CALLBACK_PORT: '18787',
        GOOSE_OAUTH_CALLBACK_BIND: '0.0.0.0',
        PATH: '/usr/bin',
      }
    )
    expect(built.env).not.toHaveProperty('GOOSE_OAUTH_CALLBACK_PORT')
    expect(built.env).not.toHaveProperty('GOOSE_OAUTH_CALLBACK_BIND')
    expect(built.env.PATH).toBe('/usr/bin')
  })

  it('GivenExtraEnvTriesToSetCallbackPort_WhenBuilding_ThenStripped', () => {
    const built = buildInstanceEnv(
      key,
      {
        dataRoot: '/tmp/avcd-agent-data',
        extraEnv: { GOOSE_OAUTH_CALLBACK_PORT: '9999', FOO: 'bar' },
      },
      {}
    )
    expect(built.env).not.toHaveProperty('GOOSE_OAUTH_CALLBACK_PORT')
    expect(built.env.FOO).toBe('bar')
  })

  it('GivenProvisioningReturnsKey_WhenBuildingInstanceEnv_ThenAvocadoKeyAndProviderAreSetAndOpenrouterKeyIsAbsent', () => {
    const built = buildInstanceEnv(
      key,
      {
        dataRoot: '/tmp/avcd-agent-data',
        gooseProvider: 'avocado',
        providerApiKeyEnv: 'AVOCADO_API_KEY',
        providerApiKey: 'sk-user-a',
        extraEnv: {
          AVOCADO_HOST: 'https://dev.avocado.tech/llm',
          OPENROUTER_API_KEY: 'sk-shared-must-not-leak',
        },
      },
      {}
    )
    expect(built.env.AVOCADO_API_KEY).toBe('sk-user-a')
    expect(built.env.GOOSE_PROVIDER).toBe('avocado')
    expect(built.env.AVOCADO_HOST).toBe('https://dev.avocado.tech/llm')
    expect(built.env).not.toHaveProperty('OPENROUTER_API_KEY')
  })

  it('GivenProvisionUrlUnset_WhenBuildingInstanceEnv_ThenLegacyOpenrouterKeyIsInjectedUnchanged', () => {
    const built = buildInstanceEnv(
      key,
      {
        dataRoot: '/tmp/avcd-agent-data',
        gooseProvider: 'openrouter',
        providerApiKeyEnv: 'OPENROUTER_API_KEY',
        providerApiKey: 'sk-shared-legacy',
      },
      {}
    )
    expect(built.env.OPENROUTER_API_KEY).toBe('sk-shared-legacy')
    expect(built.env.GOOSE_PROVIDER).toBe('openrouter')
    expect(built.env).not.toHaveProperty('AVOCADO_API_KEY')
  })
})

describe('buildInstanceArgs', () => {
  it('GivenValidPort_WhenBuilding_ThenReturnsServeArgs', () => {
    expect(buildInstanceArgs(3456)).toEqual([
      'serve',
      '--platform',
      'desktop',
      '--enable-scheduler',
      '--host',
      '127.0.0.1',
      '--port',
      '3456',
    ])
  })

  it('GivenInvalidPort_WhenBuilding_ThenThrows', () => {
    expect(() => buildInstanceArgs(0)).toThrow(InstanceEnvError)
    expect(() => buildInstanceArgs(70000)).toThrow(InstanceEnvError)
  })
})
