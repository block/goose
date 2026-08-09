import type { JWTPayload } from 'jose'

export interface UserContext {
  userId?: string
  roles: string[]
}

const PROJECT_ROLES_PREFIX = 'urn:zitadel:iam:org:project:'
const PROJECT_ROLES_SUFFIX = ':roles'
const GENERIC_PROJECT_ROLES = 'urn:zitadel:iam:org:project:roles'

function collectRoleKeys(value: unknown, out: Set<string>): void {
  if (value == null) return
  if (Array.isArray(value)) {
    for (const item of value) {
      if (typeof item === 'string' && item.trim()) out.add(item.trim())
    }
    return
  }
  if (typeof value === 'object') {
    for (const key of Object.keys(value as Record<string, unknown>)) {
      if (key.trim()) out.add(key.trim())
    }
  }
}

function parseRoles(payload: JWTPayload | undefined): string[] {
  if (!payload) return []

  const roles = new Set<string>()
  for (const [key, value] of Object.entries(payload)) {
    if (key === GENERIC_PROJECT_ROLES) {
      collectRoleKeys(value, roles)
      continue
    }
    if (key.startsWith(PROJECT_ROLES_PREFIX) && key.endsWith(PROJECT_ROLES_SUFFIX)) {
      collectRoleKeys(value, roles)
    }
  }

  return [...roles].sort()
}

export function extractUserContext(payload: JWTPayload | undefined): UserContext {
  const sub = payload?.sub
  return {
    userId: typeof sub === 'string' && sub.trim() ? sub.trim() : undefined,
    roles: parseRoles(payload),
  }
}
