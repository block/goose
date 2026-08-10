export type ZitadelAuthConfig = {
  issuer: string
  clientId: string
  projectId?: string
  orgId?: string
  googleIdpId?: string
  accessRoleKey: string
  scopes: string
  redirectUri: string
  postLogoutRedirectUri: string
  loopbackPort: number
}

export const AUTH_LOOPBACK_PORT = 47821
export const AUTH_REDIRECT_PATH = '/callback'
export const AUTH_LOGGED_OUT_PATH = '/logged-out'

export function isZitadelAuthEnabled(
  env: NodeJS.ProcessEnv = process.env
): boolean {
  const mode = env.AVCD_AUTH_MODE?.trim().toLowerCase()
  if (mode === 'off' || mode === 'false' || mode === '0') return false
  if (mode === 'zitadel' || mode === 'on' || mode === 'true' || mode === '1') {
    return Boolean(env.ZITADEL_ISSUER?.trim() && env.ZITADEL_CLIENT_ID?.trim())
  }
  // Default: enable when issuer + client id are present.
  return Boolean(env.ZITADEL_ISSUER?.trim() && env.ZITADEL_CLIENT_ID?.trim())
}

export function loadZitadelAuthConfig(
  env: NodeJS.ProcessEnv = process.env
): ZitadelAuthConfig {
  const issuer = (env.ZITADEL_ISSUER || '').trim().replace(/\/$/, '')
  const clientId = (env.ZITADEL_CLIENT_ID || '').trim()
  if (!issuer || !clientId) {
    throw new Error('ZITADEL_ISSUER and ZITADEL_CLIENT_ID are required for auth mode')
  }

  const loopbackPort = Number(env.AVCD_AUTH_LOOPBACK_PORT || AUTH_LOOPBACK_PORT)
  const projectId = env.ZITADEL_PROJECT_ID?.trim() || undefined
  const orgId = env.ZITADEL_ORG_ID?.trim() || undefined
  const googleIdpId = env.ZITADEL_GOOGLE_IDP_ID?.trim() || undefined
  const accessRoleKey = env.AGENT_ACCESS_ROLE_KEY?.trim() || 'agent-access'

  const defaultScopes = [
    'openid',
    'profile',
    'email',
    'offline_access',
    projectId ? `urn:zitadel:iam:org:project:id:${projectId}:aud` : '',
    projectId ? `urn:zitadel:iam:org:project:id:${projectId}:roles` : '',
    'urn:zitadel:iam:org:projects:roles',
    orgId ? `urn:zitadel:iam:org:id:${orgId}` : '',
    'urn:zitadel:iam:user:resourceowner',
    googleIdpId ? `urn:zitadel:iam:org:idp:id:${googleIdpId}` : '',
  ]
    .filter(Boolean)
    .join(' ')

  return {
    issuer,
    clientId,
    projectId,
    orgId,
    googleIdpId,
    accessRoleKey,
    scopes: env.ZITADEL_AUTH_SCOPES?.trim() || defaultScopes,
    redirectUri: `http://127.0.0.1:${loopbackPort}${AUTH_REDIRECT_PATH}`,
    postLogoutRedirectUri: `http://127.0.0.1:${loopbackPort}${AUTH_LOGGED_OUT_PATH}`,
    loopbackPort,
  }
}
