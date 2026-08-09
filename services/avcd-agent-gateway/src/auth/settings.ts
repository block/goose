export type AuthSettings = {
  jwtRequired: boolean
  zitadelIssuer?: string
  zitadelProjectId?: string
  jwtSecret?: string
  jwtIssuer: string
  jwtAudience: string
  agentAccessRoleKey: string
}

function envFlag(name: string): boolean {
  return process.env[name]?.trim().toLowerCase() === 'true'
}

export function loadAuthSettings(
  env: NodeJS.ProcessEnv = process.env
): AuthSettings {
  const zitadelIssuer = env.ZITADEL_ISSUER?.trim().replace(/\/$/, '')

  return {
    jwtRequired: envFlagWith(env, 'JWT_REQUIRED'),
    zitadelIssuer: zitadelIssuer || undefined,
    zitadelProjectId: env.ZITADEL_PROJECT_ID?.trim() || undefined,
    jwtSecret: env.JWT_SECRET?.trim() || undefined,
    jwtIssuer: env.JWT_ISSUER?.trim() || 'avcd',
    jwtAudience: env.JWT_AUDIENCE?.trim() || 'avcd-agent',
    agentAccessRoleKey: env.AGENT_ACCESS_ROLE_KEY?.trim() || 'agent-access',
  }
}

function envFlagWith(env: NodeJS.ProcessEnv, name: string): boolean {
  return env[name]?.trim().toLowerCase() === 'true'
}

export function logAuthSettingsOnStartup(settings: AuthSettings): void {
  const methods: string[] = []
  if (settings.zitadelIssuer) {
    methods.push(
      `zitadel issuer=${settings.zitadelIssuer} projectId=${settings.zitadelProjectId ?? '(none)'}`
    )
  }
  if (settings.jwtSecret) {
    methods.push('hs256-dev-fallback')
  }

  console.log(
    `[auth] jwtRequired=${settings.jwtRequired} role=${settings.agentAccessRoleKey} methods=[${methods.join(', ') || 'none'}]`
  )

  if (settings.jwtRequired && settings.zitadelIssuer && !settings.zitadelProjectId) {
    console.warn(
      '[auth] JWT_REQUIRED=true with ZITADEL_ISSUER but no ZITADEL_PROJECT_ID — audience validation is disabled'
    )
  }
}

// re-export helper used by tenant-context without coupling to process.env mutably
export { envFlag }
