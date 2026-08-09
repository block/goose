import { spawn, type ChildProcess } from 'node:child_process'
import { createServer } from 'node:net'
import { mkdir } from 'node:fs/promises'
import { setTimeout as delay } from 'node:timers/promises'

import type { InstanceKey } from '../auth/access.js'
import {
  buildInstanceArgs,
  buildInstanceEnv,
  type InstanceConfig,
} from './env.js'

export type RunningInstance = {
  key: string
  tenantId: string
  sub: string
  port: number
  baseUrl: string
  secretKey: string
  pathRoot: string
  pid: number
  lastUsedAt: number
}

type InternalInstance = RunningInstance & {
  child: ChildProcess
  stdoutBuf: string
  stderrBuf: string
}

export type SupervisorOptions = {
  gooseBin: string
  instanceConfig: InstanceConfig
  idleTtlMs?: number
  readinessTimeoutMs?: number
  readinessIntervalMs?: number
  killGraceMs?: number
  fetchImpl?: typeof fetch
  now?: () => number
}

async function findAvailablePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer()
    server.on('error', reject)
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address()
      if (!addr || typeof addr === 'string') {
        server.close()
        reject(new Error('Failed to allocate port'))
        return
      }
      const { port } = addr
      server.close((err) => (err ? reject(err) : resolve(port)))
    })
  })
}

async function waitForStatus(
  baseUrl: string,
  opts: {
    timeoutMs: number
    intervalMs: number
    fetchImpl: typeof fetch
    isAlive: () => boolean
  }
): Promise<void> {
  const deadline = Date.now() + opts.timeoutMs
  while (Date.now() < deadline) {
    if (!opts.isAlive()) {
      throw new Error('goose process exited before readiness')
    }
    try {
      const res = await opts.fetchImpl(`${baseUrl}/status`, {
        signal: AbortSignal.timeout(1_000),
      })
      if (res.ok) return
    } catch {
      // retry
    }
    await delay(opts.intervalMs)
  }
  throw new Error(`goose readiness timed out after ${opts.timeoutMs}ms`)
}

function spawnGoose(
  gooseBin: string,
  args: string[],
  env: Record<string, string>,
  cwd: string
): ChildProcess {
  // Prefer `node <script>` when gooseBin is a JS fake (tests) so shebang/env PATH issues never bite.
  if (gooseBin.endsWith('.js') || gooseBin.endsWith('.mjs') || gooseBin.endsWith('.cjs')) {
    return spawn(process.execPath, [gooseBin, ...args], {
      env,
      cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: false,
    })
  }
  return spawn(gooseBin, args, {
    env,
    cwd,
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: false,
  })
}

export class InstanceSupervisor {
  private readonly instances = new Map<string, InternalInstance>()
  private readonly gooseBin: string
  private readonly instanceConfig: InstanceConfig
  private readonly idleTtlMs: number
  private readonly readinessTimeoutMs: number
  private readonly readinessIntervalMs: number
  private readonly killGraceMs: number
  private readonly fetchImpl: typeof fetch
  private readonly now: () => number
  private reapTimer?: NodeJS.Timeout

  constructor(opts: SupervisorOptions) {
    this.gooseBin = opts.gooseBin
    this.instanceConfig = opts.instanceConfig
    this.idleTtlMs = opts.idleTtlMs ?? 30 * 60_000
    this.readinessTimeoutMs = opts.readinessTimeoutMs ?? 30_000
    this.readinessIntervalMs = opts.readinessIntervalMs ?? 100
    this.killGraceMs = opts.killGraceMs ?? 5_000
    this.fetchImpl = opts.fetchImpl ?? fetch
    this.now = opts.now ?? Date.now
    this.reapTimer = setInterval(() => {
      void this.reapIdle()
    }, Math.min(60_000, this.idleTtlMs))
    this.reapTimer.unref?.()
  }

  async getOrStart(instanceKey: InstanceKey): Promise<RunningInstance> {
    const existing = this.instances.get(instanceKey.key)
    if (existing && !existing.child.killed && existing.child.exitCode === null) {
      existing.lastUsedAt = this.now()
      return this.publicView(existing)
    }
    if (existing) {
      await this.stop(instanceKey.key)
    }
    return this.start(instanceKey)
  }

  async stop(key: string): Promise<void> {
    const inst = this.instances.get(key)
    if (!inst) return
    this.instances.delete(key)
    await this.terminate(inst)
  }

  async stopAll(): Promise<void> {
    const keys = [...this.instances.keys()]
    await Promise.all(keys.map((k) => this.stop(k)))
    if (this.reapTimer) clearInterval(this.reapTimer)
  }

  list(): RunningInstance[] {
    return [...this.instances.values()].map((i) => this.publicView(i))
  }

  private publicView(inst: InternalInstance): RunningInstance {
    const { child: _c, stdoutBuf: _o, stderrBuf: _e, ...view } = inst
    return view
  }

  private async start(instanceKey: InstanceKey): Promise<RunningInstance> {
    const port = await findAvailablePort()
    const built = buildInstanceEnv(instanceKey, this.instanceConfig)
    await mkdir(built.pathRoot, { recursive: true })
    const args = buildInstanceArgs(port)
    // Ensure PATH exists for shebang `env node` resolution.
    if (!built.env.PATH && process.env.PATH) {
      built.env.PATH = process.env.PATH
    }
    const child = spawnGoose(this.gooseBin, args, built.env, built.pathRoot)

    const inst: InternalInstance = {
      key: instanceKey.key,
      tenantId: instanceKey.tenantId,
      sub: instanceKey.sub,
      port,
      baseUrl: `http://127.0.0.1:${port}`,
      secretKey: built.secretKey,
      pathRoot: built.pathRoot,
      pid: child.pid ?? -1,
      lastUsedAt: this.now(),
      child,
      stdoutBuf: '',
      stderrBuf: '',
    }

    child.stdout?.on('data', (chunk: Buffer) => {
      inst.stdoutBuf += chunk.toString('utf8')
      if (inst.stdoutBuf.length > 64_000) {
        inst.stdoutBuf = inst.stdoutBuf.slice(-32_000)
      }
    })
    child.stderr?.on('data', (chunk: Buffer) => {
      inst.stderrBuf += chunk.toString('utf8')
      if (inst.stderrBuf.length > 64_000) {
        inst.stderrBuf = inst.stderrBuf.slice(-32_000)
      }
    })
    child.on('exit', () => {
      // Detach: leave map cleanup to next getOrStart/stop
      child.stdout?.removeAllListeners('data')
      child.stderr?.removeAllListeners('data')
      child.stdout?.resume()
      child.stderr?.resume()
    })

    this.instances.set(instanceKey.key, inst)

    try {
      await waitForStatus(inst.baseUrl, {
        timeoutMs: this.readinessTimeoutMs,
        intervalMs: this.readinessIntervalMs,
        fetchImpl: this.fetchImpl,
        isAlive: () => child.exitCode === null && !child.killed,
      })
      // Drain stdio after readiness so pipes cannot fill and hang the child.
      child.stdout?.resume()
      child.stderr?.resume()
      return this.publicView(inst)
    } catch (error) {
      const detail = `stdout=${inst.stdoutBuf.slice(-500)} stderr=${inst.stderrBuf.slice(-500)}`
      this.instances.delete(instanceKey.key)
      await this.terminate(inst)
      if (error instanceof Error) {
        error.message = `${error.message} (${detail})`
        throw error
      }
      throw new Error(`${String(error)} (${detail})`)
    }
  }

  private async terminate(inst: InternalInstance): Promise<void> {
    if (inst.child.exitCode !== null || inst.child.killed) return
    inst.child.kill('SIGTERM')
    const exited = await Promise.race([
      new Promise<boolean>((resolve) =>
        inst.child.once('exit', () => resolve(true))
      ),
      delay(this.killGraceMs).then(() => false),
    ])
    if (!exited && inst.child.exitCode === null) {
      inst.child.kill('SIGKILL')
      await new Promise<void>((resolve) => inst.child.once('exit', () => resolve()))
    }
  }

  private async reapIdle(): Promise<void> {
    const now = this.now()
    for (const [key, inst] of this.instances) {
      if (now - inst.lastUsedAt >= this.idleTtlMs) {
        await this.stop(key)
      }
    }
  }
}
