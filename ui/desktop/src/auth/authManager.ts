import {
  isZitadelAuthEnabled,
  loadZitadelAuthConfig,
  type ZitadelAuthConfig,
} from './config'
import { startLoopbackServer } from './loopback'
import { buildAuthorizeUrl, createPkcePair } from './pkce'
import { TokenStore, type SafeStorageLike, type StoredTokens } from './tokenStore'

export type AuthStatus =
  | { state: 'disabled' }
  | { state: 'signedOut' }
  | { state: 'signingIn' }
  | { state: 'accessDenied'; email?: string }
  | {
      state: 'signedIn'
      email?: string
      name?: string
      sub: string
      tenantId?: string
      roles: string[]
      expiresAt: number
    }

export type AuthManagerDeps = {
  userDataPath: string
  safeStorage: SafeStorageLike
  openExternal: (url: string) => Promise<void>
  fetchImpl?: typeof fetch
  env?: NodeJS.ProcessEnv
  onStatusChange?: (status: AuthStatus) => void
}

type TokenResponse = {
  access_token: string
  refresh_token?: string
  id_token?: string
  expires_in?: number
  token_type?: string
  scope?: string
}

function decodeJwtPayload(token: string): Record<string, unknown> {
  const parts = token.split('.')
  if (parts.length < 2) return {}
  const json = Buffer.from(parts[1].replace(/-/g, '+').replace(/_/g, '/'), 'base64').toString(
    'utf8'
  )
  return JSON.parse(json) as Record<string, unknown>
}

function extractRoles(payload: Record<string, unknown>): string[] {
  const roles = new Set<string>()
  for (const [key, value] of Object.entries(payload)) {
    if (
      key === 'urn:zitadel:iam:org:project:roles' ||
      (key.startsWith('urn:zitadel:iam:org:project:') && key.endsWith(':roles'))
    ) {
      if (value && typeof value === 'object') {
        for (const role of Object.keys(value as object)) {
          if (role.trim()) roles.add(role.trim())
        }
      }
    }
  }
  return [...roles].sort()
}

export class AuthManager {
  private readonly store: TokenStore
  private readonly openExternal: (url: string) => Promise<void>
  private readonly fetchImpl: typeof fetch
  private readonly env: NodeJS.ProcessEnv
  private readonly onStatusChange?: (status: AuthStatus) => void
  private status: AuthStatus
  private config: ZitadelAuthConfig | null = null
  private tokens: StoredTokens | null = null
  private refreshTimer?: NodeJS.Timeout

  constructor(deps: AuthManagerDeps) {
    this.store = new TokenStore(deps.userDataPath, deps.safeStorage)
    this.openExternal = deps.openExternal
    this.fetchImpl = deps.fetchImpl ?? fetch
    this.env = deps.env ?? process.env
    this.onStatusChange = deps.onStatusChange

    if (!isZitadelAuthEnabled(this.env)) {
      this.status = { state: 'disabled' }
      return
    }

    this.config = loadZitadelAuthConfig(this.env)
    this.tokens = this.store.load()
    this.status = this.tokens
      ? this.statusFromTokens(this.tokens)
      : { state: 'signedOut' }
    if (this.tokens) {
      this.scheduleRefresh(this.tokens)
    }
  }

  getStatus(): AuthStatus {
    return this.status
  }

  isEnabled(): boolean {
    return this.status.state !== 'disabled'
  }

  async getAccessToken(options?: { forceRefresh?: boolean }): Promise<string | null> {
    if (!this.config || !this.tokens) return null
    const skewMs = 60_000
    const needsRefresh =
      options?.forceRefresh || this.tokens.expiresAt - Date.now() < skewMs
    if (needsRefresh) {
      if (!this.tokens.refreshToken) {
        this.setStatus({ state: 'signedOut' })
        this.tokens = null
        this.store.clear()
        return null
      }
      try {
        await this.refresh(this.tokens.refreshToken)
      } catch {
        this.setStatus({ state: 'signedOut' })
        this.tokens = null
        this.store.clear()
        return null
      }
    }
    return this.tokens?.accessToken ?? null
  }

  async login(): Promise<AuthStatus> {
    if (!this.config) {
      throw new Error('Zitadel auth is not configured')
    }
    this.setStatus({ state: 'signingIn' })
    const pkce = createPkcePair()
    const loopback = await startLoopbackServer(this.config.loopbackPort)
    try {
      const authorizeUrl = buildAuthorizeUrl(this.config.issuer, {
        clientId: this.config.clientId,
        redirectUri: this.config.redirectUri,
        scope: this.config.scopes,
        state: pkce.state,
        codeChallenge: pkce.codeChallenge,
      })
      await this.openExternal(authorizeUrl)
      const result = await loopback.waitForCallback()
      if (result.type === 'error') {
        this.setStatus({ state: 'signedOut' })
        throw new Error(result.description || result.error)
      }
      if (result.type !== 'code') {
        this.setStatus({ state: 'signedOut' })
        throw new Error('Unexpected callback')
      }
      if (result.state !== pkce.state) {
        this.setStatus({ state: 'signedOut' })
        throw new Error('OAuth state mismatch')
      }
      await this.exchangeCode(result.code, pkce.codeVerifier)
      return this.status
    } catch (error) {
      if (this.status.state === 'signingIn') {
        this.setStatus({ state: 'signedOut' })
      }
      throw error
    } finally {
      await loopback.close().catch(() => undefined)
    }
  }

  async logout(): Promise<void> {
    if (!this.config) return
    const idToken = this.tokens?.idToken
    const refreshToken = this.tokens?.refreshToken
    this.tokens = null
    this.store.clear()
    if (this.refreshTimer) clearTimeout(this.refreshTimer)

    try {
      if (refreshToken) {
        await this.fetchImpl(`${this.config.issuer}/oauth/v2/revoke`, {
          method: 'POST',
          headers: { 'content-type': 'application/x-www-form-urlencoded' },
          body: new URLSearchParams({
            token: refreshToken,
            client_id: this.config.clientId,
            token_type_hint: 'refresh_token',
          }),
        })
      }
    } catch {
      // best-effort revoke
    }

    this.setStatus({ state: 'signedOut' })

    if (idToken) {
      const end = new URL(`${this.config.issuer}/oidc/v1/end_session`)
      end.searchParams.set('id_token_hint', idToken)
      end.searchParams.set('post_logout_redirect_uri', this.config.postLogoutRedirectUri)
      end.searchParams.set('client_id', this.config.clientId)
      try {
        const loopback = await startLoopbackServer(this.config.loopbackPort)
        await this.openExternal(end.toString())
        await Promise.race([
          loopback.waitForCallback(15_000),
          new Promise((r) => setTimeout(r, 3_000)),
        ])
        await loopback.close().catch(() => undefined)
      } catch {
        // ignore end_session failures
      }
    }
  }

  broadcast(status: AuthStatus = this.status): void {
    try {
      // Lazy require so unit tests that don't load Electron can still import AuthManager types.
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const { BrowserWindow } = require('electron') as typeof import('electron')
      for (const win of BrowserWindow.getAllWindows()) {
        win.webContents.send('auth:on-changed', status)
      }
    } catch {
      // Electron unavailable (unit tests)
    }
  }

  private async exchangeCode(code: string, codeVerifier: string): Promise<void> {
    if (!this.config) throw new Error('missing config')
    const res = await this.fetchImpl(`${this.config.issuer}/oauth/v2/token`, {
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri: this.config.redirectUri,
        client_id: this.config.clientId,
        code_verifier: codeVerifier,
      }),
    })
    if (!res.ok) {
      const text = await res.text()
      throw new Error(`Token exchange failed: ${res.status} ${text}`)
    }
    const body = (await res.json()) as TokenResponse
    this.applyTokenResponse(body)
  }

  private async refresh(refreshToken: string): Promise<void> {
    if (!this.config) throw new Error('missing config')
    const res = await this.fetchImpl(`${this.config.issuer}/oauth/v2/token`, {
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'refresh_token',
        refresh_token: refreshToken,
        client_id: this.config.clientId,
      }),
    })
    if (!res.ok) {
      throw new Error(`Refresh failed: ${res.status}`)
    }
    const body = (await res.json()) as TokenResponse
    this.applyTokenResponse({
      ...body,
      refresh_token: body.refresh_token || refreshToken,
    })
  }

  private applyTokenResponse(body: TokenResponse): void {
    if (!body.access_token) throw new Error('No access_token in response')
    const expiresIn = typeof body.expires_in === 'number' ? body.expires_in : 3600
    const tokens: StoredTokens = {
      accessToken: body.access_token,
      refreshToken: body.refresh_token,
      idToken: body.id_token,
      expiresAt: Date.now() + expiresIn * 1000,
      tokenType: body.token_type,
      scope: body.scope,
    }
    this.tokens = tokens
    this.store.save(tokens)
    this.scheduleRefresh(tokens)
    this.setStatus(this.statusFromTokens(tokens))
  }

  private statusFromTokens(tokens: StoredTokens): AuthStatus {
    const payload = decodeJwtPayload(tokens.accessToken)
    const roles = extractRoles(payload)
    const roleKey = this.config?.accessRoleKey || 'agent-access'
    const email =
      typeof payload.email === 'string'
        ? payload.email
        : typeof payload.preferred_username === 'string'
          ? payload.preferred_username
          : undefined
    const name = typeof payload.name === 'string' ? payload.name : undefined
    const sub = typeof payload.sub === 'string' ? payload.sub : ''
    const tenantId =
      typeof payload['urn:zitadel:iam:org:id'] === 'string'
        ? (payload['urn:zitadel:iam:org:id'] as string)
        : typeof payload['urn:zitadel:iam:user:resourceowner:id'] === 'string'
          ? (payload['urn:zitadel:iam:user:resourceowner:id'] as string)
          : undefined

    if (!roles.includes(roleKey)) {
      return { state: 'accessDenied', email }
    }
    return {
      state: 'signedIn',
      email,
      name,
      sub,
      tenantId,
      roles,
      expiresAt: tokens.expiresAt,
    }
  }

  private scheduleRefresh(tokens: StoredTokens): void {
    if (this.refreshTimer) clearTimeout(this.refreshTimer)
    const delay = Math.max(5_000, tokens.expiresAt - Date.now() - 90_000)
    this.refreshTimer = setTimeout(() => {
      void this.getAccessToken({ forceRefresh: true })
    }, delay)
    this.refreshTimer.unref?.()
  }

  private setStatus(status: AuthStatus): void {
    this.status = status
    this.onStatusChange?.(status)
    try {
      this.broadcast(status)
    } catch {
      // BrowserWindow may be unavailable in unit tests
    }
  }
}
