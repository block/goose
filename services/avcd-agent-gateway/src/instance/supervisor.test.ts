import { mkdtemp, writeFile, chmod, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { InstanceSupervisor, ProvisioningError } from './supervisor.js'

function pidAlive(pid: number): boolean {
  if (pid <= 0) return false
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

async function writeFakeGoose(dir: string): Promise<string> {
  const bin = path.join(dir, 'fake-goose.js')
  // Minimal HTTP server that answers /status and exits on SIGTERM.
  const script = `
const http = require('http');
const args = process.argv.slice(2);
const portIdx = args.indexOf('--port');
const port = Number(args[portIdx + 1] || 0);
const server = http.createServer((req, res) => {
  const pathOnly = (req.url || '').split('?')[0];
  if (pathOnly === '/status' || pathOnly === '/health') {
    res.writeHead(200); res.end('ok'); return;
  }
  res.writeHead(404); res.end('no');
});
server.listen(port, '127.0.0.1', () => {
  process.stdout.write('listening\\n');
});
const shutdown = () => { server.close(() => process.exit(0)); };
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
`
  await writeFile(bin, script, 'utf8')
  await chmod(bin, 0o755)
  return bin
}

describe('InstanceSupervisor (covers AC-3)', () => {
  const supervisors: InstanceSupervisor[] = []

  afterEach(async () => {
    await Promise.all(supervisors.splice(0).map((s) => s.stopAll()))
  })

  it('GivenSameKeyTwice_WhenGetOrStart_ThenOneProcess', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 5_000,
    })
    supervisors.push(sup)

    const key = { tenantId: 't1', sub: 'u1', key: 't1/u1' }
    const a = await sup.getOrStart(key)
    const b = await sup.getOrStart(key)
    expect(a.pid).toBe(b.pid)
    expect(sup.list()).toHaveLength(1)
  })

  it('GivenTwoKeys_WhenGetOrStart_ThenTwoProcesses', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 5_000,
    })
    supervisors.push(sup)

    const a = await sup.getOrStart({ tenantId: 't1', sub: 'u1', key: 't1/u1' })
    const b = await sup.getOrStart({ tenantId: 't1', sub: 'u2', key: 't1/u2' })
    expect(a.pid).not.toBe(b.pid)
    expect(a.pathRoot).not.toBe(b.pathRoot)
    expect(a.secretKey).not.toBe(b.secretKey)
    expect(sup.list()).toHaveLength(2)
  })

  it('GivenReadinessTimeout_WhenStart_ThenCleansUp', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = path.join(dir, 'hang-goose.js')
    await writeFile(bin, `setInterval(() => {}, 10000);\n`, 'utf8')
    await chmod(bin, 0o755)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 400,
      readinessIntervalMs: 50,
      killGraceMs: 200,
    })
    supervisors.push(sup)

    await expect(
      sup.getOrStart({ tenantId: 't1', sub: 'u1', key: 't1/u1' })
    ).rejects.toThrow(/readiness timed out/)
    expect(sup.list()).toHaveLength(0)
  })

  it('GivenInstanceKilledBySignal_WhenGetOrStart_ThenRespawnsOnNewPort', async () => {
    // Regression: a SIGKILLed child keeps exitCode === null and killed === false,
    // so the raw flags reported it alive and the proxy kept forwarding to a dead
    // port until every request failed with ECONNREFUSED.
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 5_000,
      killGraceMs: 1_000,
    })
    supervisors.push(sup)

    const key = { tenantId: 't1', sub: 'u1', key: 't1/u1' }
    const first = await sup.getOrStart(key)

    process.kill(first.pid, 'SIGKILL')

    // Polling getOrStart is safe: it returns the stale entry until the 'exit'
    // event lands, then respawns exactly once.
    let second = first
    await vi.waitFor(async () => {
      second = await sup.getOrStart(key)
      expect(second.pid).not.toBe(first.pid)
    })

    expect(second.port).not.toBe(first.port)
    expect(sup.list()).toHaveLength(1)
  })

  it('GivenMissingBinary_WhenStart_ThenRejectsWithSpawnErrorAndNoCrash', async () => {
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const sup = new InstanceSupervisor({
      gooseBin: path.join(tmpdir(), 'definitely-not-a-real-goose-binary'),
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 2_000,
      readinessIntervalMs: 50,
      killGraceMs: 200,
    })
    supervisors.push(sup)

    await expect(sup.getOrStart({ tenantId: 't1', sub: 'u1', key: 't1/u1' })).rejects.toThrow(
      /spawnError=/
    )
    expect(sup.list()).toHaveLength(0)
  })

  it('GivenStop_WhenCalled_ThenProcessExits', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      readinessTimeoutMs: 5_000,
      killGraceMs: 1_000,
    })
    supervisors.push(sup)

    const inst = await sup.getOrStart({ tenantId: 't1', sub: 'u1', key: 't1/u1' })
    await sup.stop(inst.key)
    expect(sup.list()).toHaveLength(0)
  })
})

describe('InstanceSupervisor avocado provisioning (covers AC-1, AC-8, AC-9)', () => {
  const supervisors: InstanceSupervisor[] = []

  afterEach(async () => {
    await Promise.all(supervisors.splice(0).map((s) => s.stopAll()))
  })

  async function writeEnvEchoFakeGoose(dir: string): Promise<string> {
    const bin = path.join(dir, 'fake-goose-env.js')
    const script = `
const http = require('http');
const fs = require('fs');
const path = require('path');
const args = process.argv.slice(2);
const port = Number(args[args.indexOf('--port') + 1] || 0);
const envSnap = {
  AVOCADO_API_KEY: process.env.AVOCADO_API_KEY || null,
  AVOCADO_HOST: process.env.AVOCADO_HOST || null,
  GOOSE_PROVIDER: process.env.GOOSE_PROVIDER || null,
  OPENROUTER_API_KEY: process.env.OPENROUTER_API_KEY || null,
};
const out = path.join(process.env.GOOSE_PATH_ROOT || process.cwd(), 'env-echo.json');
fs.writeFileSync(out, JSON.stringify(envSnap));
const server = http.createServer((req, res) => {
  const pathOnly = (req.url || '').split('?')[0];
  if (pathOnly === '/status' || pathOnly === '/health') {
    res.writeHead(200); res.end('ok'); return;
  }
  res.writeHead(404); res.end('no');
});
server.listen(port, '127.0.0.1');
const shutdown = () => { server.close(() => process.exit(0)); };
process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);
`
    await writeFile(bin, script, 'utf8')
    await chmod(bin, 0o755)
    return bin
  }

  function provisionFetch(
    handler: (authHeader: string | null) => Response | Promise<Response>
  ): typeof fetch {
    return async (input, init) => {
      const url = String(input)
      if (url.includes('/keys/provision')) {
        const headers = new Headers(init?.headers)
        return handler(headers.get('authorization'))
      }
      return fetch(input, init)
    }
  }

  it('GivenTwoUsers_WhenEachStarts_ThenEachChildReceivesItsOwnKey', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeEnvEchoFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const fetchImpl = provisionFetch(async (auth) => {
      const token = auth?.startsWith('Bearer ') ? auth.slice('Bearer '.length) : ''
      return new Response(
        JSON.stringify({
          apiKey: `sk-test-${token}`,
          baseUrl: 'https://dev.avocado.tech/llm',
          userId: `tenant1:${token}`,
          expiresAt: '2099-01-01T00:00:00.000Z',
        }),
        { status: 200, headers: { 'content-type': 'application/json' } }
      )
    })
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: {
        dataRoot,
        gooseProvider: 'openrouter',
        providerApiKeyEnv: 'OPENROUTER_API_KEY',
        providerApiKey: 'sk-shared-must-not-leak',
      },
      avocadoProvisionUrl: 'http://provision.test/keys/provision',
      avocadoHost: 'https://dev.avocado.tech/llm',
      fetchImpl,
      readinessTimeoutMs: 5_000,
    })
    supervisors.push(sup)

    const a = await sup.getOrStart({ tenantId: 't1', sub: 'u1', key: 't1/u1' }, 'user-a')
    const b = await sup.getOrStart({ tenantId: 't1', sub: 'u2', key: 't1/u2' }, 'user-b')
    const envA = JSON.parse(
      await readFile(path.join(a.pathRoot, 'env-echo.json'), 'utf8')
    ) as Record<string, string | null>
    const envB = JSON.parse(
      await readFile(path.join(b.pathRoot, 'env-echo.json'), 'utf8')
    ) as Record<string, string | null>

    expect(envA.AVOCADO_API_KEY).toBe('sk-test-user-a')
    expect(envB.AVOCADO_API_KEY).toBe('sk-test-user-b')
    expect(envA.AVOCADO_API_KEY).not.toBe(envB.AVOCADO_API_KEY)
    expect(envA.GOOSE_PROVIDER).toBe('avocado')
    expect(envA.OPENROUTER_API_KEY).toBeNull()
    expect(envB.OPENROUTER_API_KEY).toBeNull()
  })

  it('GivenProvisioningReturns403_WhenStarting_ThenThrowsProvisioningErrorAndNoInstanceIsRegistered', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    const fetchImpl = provisionFetch(async () =>
      new Response(JSON.stringify({ error: 'forbidden', detail: 'missing agent-access role' }), {
        status: 403,
        headers: { 'content-type': 'application/json' },
      })
    )
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot, providerApiKey: 'sk-shared', providerApiKeyEnv: 'OPENROUTER_API_KEY' },
      avocadoProvisionUrl: 'http://provision.test/keys/provision',
      fetchImpl,
      readinessTimeoutMs: 5_000,
    })
    supervisors.push(sup)

    await expect(
      sup.getOrStart({ tenantId: 't1', sub: 'blocked', key: 't1/blocked' }, 'tok')
    ).rejects.toBeInstanceOf(ProvisioningError)

    expect(sup.list()).toHaveLength(0)
    expect(sup.list().some((i) => i.sub === 'blocked')).toBe(false)
  })

  it(
    'GivenProvisioningTimesOut_WhenStarting_ThenFailsWithoutOrphanChildProcess',
    async () => {
      const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
      const bin = await writeFakeGoose(dir)
      const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
      // Hang until AbortSignal.timeout(5000) fires inside provisionAvocadoKey.
      const abortAware: typeof fetch = async (input, init) => {
        const url = String(input)
        if (url.includes('/keys/provision')) {
          const signal = init?.signal
          return await new Promise<Response>((_resolve, reject) => {
            if (signal?.aborted) {
              reject(signal.reason ?? new DOMException('The operation was aborted', 'AbortError'))
              return
            }
            signal?.addEventListener('abort', () => {
              reject(signal.reason ?? new DOMException('The operation was aborted', 'AbortError'))
            })
          })
        }
        return fetch(input, init)
      }
      const sup = new InstanceSupervisor({
        gooseBin: bin,
        instanceConfig: { dataRoot },
        avocadoProvisionUrl: 'http://provision.test/keys/provision',
        fetchImpl: abortAware,
        readinessTimeoutMs: 5_000,
      })
      supervisors.push(sup)

      const key = { tenantId: 't1', sub: 'slow', key: 't1/slow' }
      await expect(sup.getOrStart(key, 'tok')).rejects.toBeInstanceOf(ProvisioningError)
      expect(sup.list()).toHaveLength(0)
      // No child was registered; nothing to orphan. Guard against accidental spawn.
      expect(sup.list().every((i) => !pidAlive(i.pid))).toBe(true)
    },
    15_000
  )

  it('GivenTwoConcurrentStartsForSameUser_WhenProvisioning_ThenProvisioningIsCalledOnceAndOneProcessRuns', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'gw-sup-'))
    const bin = await writeFakeGoose(dir)
    const dataRoot = await mkdtemp(path.join(tmpdir(), 'gw-data-'))
    let provisionCalls = 0
    const fetchImpl: typeof fetch = async (input, init) => {
      const url = String(input)
      if (url.includes('/keys/provision')) {
        provisionCalls += 1
        await new Promise((r) => setTimeout(r, 150))
        return new Response(
          JSON.stringify({
            apiKey: 'sk-once',
            baseUrl: 'https://dev.avocado.tech/llm',
            userId: 't1:u1',
            expiresAt: '2099-01-01T00:00:00.000Z',
          }),
          { status: 200, headers: { 'content-type': 'application/json' } }
        )
      }
      return fetch(input, init)
    }
    const sup = new InstanceSupervisor({
      gooseBin: bin,
      instanceConfig: { dataRoot },
      avocadoProvisionUrl: 'http://provision.test/keys/provision',
      fetchImpl,
      readinessTimeoutMs: 5_000,
    })
    supervisors.push(sup)

    const key = { tenantId: 't1', sub: 'u1', key: 't1/u1' }
    const [a, b] = await Promise.all([
      sup.getOrStart(key, 'tok'),
      sup.getOrStart(key, 'tok'),
    ])
    expect(a.pid).toBe(b.pid)
    expect(provisionCalls).toBe(1)
    expect(sup.list()).toHaveLength(1)
  })
})

