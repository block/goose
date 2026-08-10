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
import { ProvisioningError, provisionAvocadoKey } from './provisioning.js'

export { ProvisioningError } from './provisioning.js'

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
  /**
   * Set from 'exit'/'error'. Needed because a child killed by a signal keeps
   * `exitCode === null`, and `killed` only reflects our own kill() calls — so
   * the raw flags report a dead instance as alive.
   */
  exited: boolean
  spawnError?: string
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
  logger?: (message: string) => void
  /** When set, provision a per-user Avocado key before each spawn (AC-1). */
  avocadoProvisionUrl?: string
  /** Host injected as AVOCADO_HOST; default https://dev.avocado.tech/llm */
  avocadoHost?: string
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

function isAlive(inst: InternalInstance): boolean {
  const { child } = inst
  return (
    !inst.exited &&
    !child.killed &&
    child.exitCode === null &&
    child.signalCode === null
  )
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
  private readonly inFlight = new Map<string, Promise<RunningInstance>>()
  private readonly gooseBin: string
  private readonly instanceConfig: InstanceConfig
  private readonly idleTtlMs: number
  private readonly readinessTimeoutMs: number
  private readonly readinessIntervalMs: number
  private readonly killGraceMs: number
  private readonly fetchImpl: typeof fetch
  private readonly now: () => number
  private readonly logger: (message: string) => void
  private readonly avocadoProvisionUrl?: string
  private readonly avocadoHost: string
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
    this.logger = opts.logger ?? (() => {})
    this.avocadoProvisionUrl = opts.avocadoProvisionUrl?.trim() || undefined
    this.avocadoHost = opts.avocadoHost?.trim() || 'https://dev.avocado.tech/llm'
    this.reapTimer = setInterval(() => {
      void this.reapIdle()
    }, Math.min(60_000, this.idleTtlMs))
    this.reapTimer.unref?.()
  }

  async getOrStart(
    instanceKey: InstanceKey,
    accessToken?: string
  ): Promise<RunningInstance> {
    const existing = this.instances.get(instanceKey.key)
    if (existing && isAlive(existing)) {
      existing.lastUsedAt = this.now()
      return this.publicView(existing)
    }

    const pending = this.inFlight.get(instanceKey.key)
    if (pending) {
      return pending
    }

    // Register in-flight synchronously so concurrent callers share one start
    // (including one provisioning call) even when we still need to stop a dead child.
    const promise = (async () => {
      const again = this.instances.get(instanceKey.key)
      if (again && isAlive(again)) {
        again.lastUsedAt = this.now()
        return this.publicView(again)
      }
      if (again) {
        await this.stop(instanceKey.key)
      }
      return this.start(instanceKey, accessToken)
    })().finally(() => {
      this.inFlight.delete(instanceKey.key)
    })
    this.inFlight.set(instanceKey.key, promise)
    return promise
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

  private async resolveInstanceConfig(
    accessToken: string | undefined
  ): Promise<InstanceConfig> {
    if (!this.avocadoProvisionUrl) {
      return this.instanceConfig
    }
    if (!accessToken) {
      throw new ProvisioningError('missing access token for provisioning', 401)
    }

    const result = await provisionAvocadoKey(
      this.avocadoProvisionUrl,
      accessToken,
      this.fetchImpl
    )
    if (!result.ok) {
      throw new ProvisioningError(result.error, result.statusCode)
    }

    return {
      ...this.instanceConfig,
      gooseProvider: 'avocado',
      providerApiKeyEnv: 'AVOCADO_API_KEY',
      providerApiKey: result.apiKey,
      extraEnv: {
        ...this.instanceConfig.extraEnv,
        AVOCADO_HOST: this.avocadoHost,
      },
    }
  }

  private async start(
    instanceKey: InstanceKey,
    accessToken?: string
  ): Promise<RunningInstance> {
    const port = await findAvailablePort()

    let cfg: InstanceConfig
    try {
      cfg = await this.resolveInstanceConfig(accessToken)
    } catch (error) {
      // Provisioning happens before spawn — nothing to clean up in the map.
      if (error instanceof ProvisioningError) throw error
      throw new ProvisioningError(
        error instanceof Error ? error.message : 'provisioning failed',
        503
      )
    }

    const built = buildInstanceEnv(instanceKey, cfg)
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
      exited: false,
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
    // Without this listener a spawn failure (bad gooseBin) raises an unhandled
    // 'error' event and takes the gateway down with it.
    child.on('error', (error) => {
      inst.exited = true
      inst.spawnError = error.message
      this.logger(`instance ${inst.key} failed to spawn: ${error.message}`)
    })
    child.on('exit', (code, signal) => {
      inst.exited = true
      this.logger(
        `instance ${inst.key} exited (pid=${inst.pid} code=${code} signal=${signal}); ` +
          'it will be respawned on the next request'
      )
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
        isAlive: () => isAlive(inst),
      })
      this.logger(`instance ${inst.key} ready on ${inst.baseUrl} (pid=${inst.pid})`)
      // Drain stdio after readiness so pipes cannot fill and hang the child.
      child.stdout?.resume()
      child.stderr?.resume()
      return this.publicView(inst)
    } catch (error) {
      const spawnDetail = inst.spawnError ? ` spawnError=${inst.spawnError}` : ''
      const detail =
        `stdout=${inst.stdoutBuf.slice(-500)} stderr=${inst.stderrBuf.slice(-500)}` + spawnDetail
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
    if (!isAlive(inst)) return
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
