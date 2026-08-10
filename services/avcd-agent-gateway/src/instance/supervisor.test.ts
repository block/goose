import { mkdtemp, writeFile, chmod } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { createServer } from 'node:http'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { InstanceSupervisor } from './supervisor.js'

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
