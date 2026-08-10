import { randomBytes } from 'node:crypto'
import path from 'node:path'

import type { InstanceKey } from '../auth/access.js'

export type InstanceConfig = {
  dataRoot: string
  gooseProvider?: string
  gooseModel?: string
  providerApiKeyEnv?: string
  providerApiKey?: string
  extraEnv?: Record<string, string>
}

export type BuiltInstanceEnv = {
  env: Record<string, string>
  pathRoot: string
  secretKey: string
}

export class InstanceEnvError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'InstanceEnvError'
  }
}

function assertAbsoluteRoot(dataRoot: string): string {
  const trimmed = dataRoot.trim()
  if (!trimmed) {
    throw new InstanceEnvError('AVCD_AGENT_DATA_ROOT must be a non-empty absolute path')
  }
  if (!path.isAbsolute(trimmed)) {
    throw new InstanceEnvError(
      `AVCD_AGENT_DATA_ROOT must be absolute (got relative: ${trimmed})`
    )
  }
  return path.resolve(trimmed)
}

export function buildInstanceEnv(
  instanceKey: InstanceKey,
  cfg: InstanceConfig,
  parentEnv: NodeJS.ProcessEnv = process.env
): BuiltInstanceEnv {
  const dataRoot = assertAbsoluteRoot(cfg.dataRoot)
  const pathRoot = path.join(dataRoot, instanceKey.tenantId, instanceKey.sub)
  if (!pathRoot.startsWith(dataRoot + path.sep) && pathRoot !== dataRoot) {
    throw new InstanceEnvError('Resolved GOOSE_PATH_ROOT escapes data root')
  }

  const secretKey = randomBytes(32).toString('hex')
  const env: Record<string, string> = {
    HOME: pathRoot,
    GOOSE_PATH_ROOT: pathRoot,
    GOOSE_DISABLE_KEYRING: 'true',
    GOOSE_SERVER__SECRET_KEY: secretKey,
    GOOSE_TELEMETRY_OFF: 'true',
  }

  // Explicitly drop fixed OAuth callback port — multi-instance cannot share it.
  // Do not copy GOOSE_OAUTH_CALLBACK_PORT even if present in parent.
  for (const [key, value] of Object.entries(parentEnv)) {
    if (value === undefined) continue
    if (key === 'GOOSE_OAUTH_CALLBACK_PORT') continue
    if (key === 'GOOSE_OAUTH_CALLBACK_BIND') continue
    if (key in env) continue
    // Avoid leaking parent secrets into child accidentally beyond PATH/HOME basics.
    if (
      key === 'PATH' ||
      key === 'LANG' ||
      key === 'LC_ALL' ||
      key === 'TZ' ||
      key === 'USER' ||
      key === 'LOGNAME'
    ) {
      env[key] = value
    }
  }

  if (cfg.gooseProvider) env.GOOSE_PROVIDER = cfg.gooseProvider
  if (cfg.gooseModel) env.GOOSE_MODEL = cfg.gooseModel
  if (cfg.providerApiKeyEnv && cfg.providerApiKey) {
    env[cfg.providerApiKeyEnv] = cfg.providerApiKey
  }
  if (cfg.extraEnv) {
    for (const [k, v] of Object.entries(cfg.extraEnv)) {
      if (k === 'GOOSE_OAUTH_CALLBACK_PORT') continue
      env[k] = v
    }
  }

  if ('GOOSE_OAUTH_CALLBACK_PORT' in env) {
    delete env.GOOSE_OAUTH_CALLBACK_PORT
  }

  // Avocado-provisioned instances must never carry the legacy shared OpenRouter key.
  const avocadoProvisioned =
    cfg.gooseProvider === 'avocado' || cfg.providerApiKeyEnv === 'AVOCADO_API_KEY'
  if (avocadoProvisioned && 'OPENROUTER_API_KEY' in env) {
    delete env.OPENROUTER_API_KEY
  }

  return { env, pathRoot, secretKey }
}

export function buildInstanceArgs(port: number): string[] {
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    throw new InstanceEnvError(`Invalid port: ${port}`)
  }
  return [
    'serve',
    '--platform',
    'desktop',
    '--enable-scheduler',
    '--host',
    '127.0.0.1',
    '--port',
    String(port),
  ]
}
