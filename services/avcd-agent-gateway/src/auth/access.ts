import type { JWTPayload } from 'jose'

import { BearerAuthError } from './verify-bearer.js'
import { resolveTenantId, TenantRequiredError } from './tenant-context.js'
import { extractUserContext } from './user-context.js'
import { type AuthSettings, loadAuthSettings } from './settings.js'

export type InstanceKey = {
  tenantId: string
  sub: string
  /** Path-safe sticky key: `${tenantId}/${sub}` */
  key: string
}

export class ForbiddenError extends Error {
  readonly statusCode = 403

  constructor(message: string) {
    super(message)
    this.name = 'ForbiddenError'
  }
}

export function requireAgentAccess(
  payload: JWTPayload,
  settings: AuthSettings = loadAuthSettings()
): void {
  const { roles } = extractUserContext(payload)
  const roleKey = settings.agentAccessRoleKey
  if (!roles.includes(roleKey)) {
    throw new ForbiddenError(`Missing required role: ${roleKey}`)
  }
}

const SAFE_SEGMENT = /^[A-Za-z0-9._@+-]+$/

function assertSafeSegment(label: string, value: string): string {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new BearerAuthError(`${label} is required`)
  }
  if (trimmed.includes('..') || trimmed.includes('/') || trimmed.includes('\\')) {
    throw new BearerAuthError(`Invalid ${label}`)
  }
  if (!SAFE_SEGMENT.test(trimmed)) {
    throw new BearerAuthError(`Invalid ${label}`)
  }
  return trimmed
}

export function resolveInstanceKey(
  payload: JWTPayload,
  settings: AuthSettings = loadAuthSettings()
): InstanceKey {
  requireAgentAccess(payload, settings)

  let tenantId: string
  try {
    tenantId = resolveTenantId(payload, settings.jwtRequired)
  } catch (error) {
    if (error instanceof TenantRequiredError) {
      throw new BearerAuthError(error.message)
    }
    throw error
  }

  const subRaw = typeof payload.sub === 'string' ? payload.sub : ''
  const sub = assertSafeSegment('sub', subRaw)
  const safeTenant = assertSafeSegment('tenantId', tenantId)

  return {
    tenantId: safeTenant,
    sub,
    key: `${safeTenant}/${sub}`,
  }
}
