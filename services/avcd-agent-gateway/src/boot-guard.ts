/**
 * Fail-closed boot checks for the gateway process.
 * covers AC-5, AC-7
 */
export class BootConfigError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'BootConfigError'
  }
}

function envFlag(env: NodeJS.ProcessEnv, name: string): boolean {
  return env[name]?.trim().toLowerCase() === 'true'
}

/**
 * Refuse to start without per-user Avocado provisioning, and require JWT in
 * production so tenant identity cannot collapse to DEFAULT_DEV_TENANT_ID.
 */
export function assertFailClosedBootEnv(
  env: NodeJS.ProcessEnv = process.env
): void {
  const dataRoot = env.AVCD_AGENT_DATA_ROOT?.trim()
  if (!dataRoot) {
    throw new BootConfigError('AVCD_AGENT_DATA_ROOT is required')
  }

  const provisionUrl = env.AVOCADO_PROVISION_URL?.trim()
  if (!provisionUrl) {
    throw new BootConfigError(
      'AVOCADO_PROVISION_URL is required — refusing to boot with a shared owner LLM key'
    )
  }

  const isProd =
    env.NODE_ENV?.trim().toLowerCase() === 'production' ||
    env.AVCD_GATEWAY_ENV?.trim().toLowerCase() === 'production'
  if (isProd && !envFlag(env, 'JWT_REQUIRED')) {
    throw new BootConfigError(
      'JWT_REQUIRED=true is required in production — refusing to boot'
    )
  }
}
