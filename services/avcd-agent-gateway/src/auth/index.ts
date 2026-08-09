export {
  loadAuthSettings,
  logAuthSettingsOnStartup,
  type AuthSettings,
} from './settings.js'
export {
  BearerAuthError,
  extractAccessToken,
  extractBearerToken,
  resetJwksCacheForTests,
  verifyBearerToken,
  zitadelJwksUrl,
} from './verify-bearer.js'
export {
  extractUserContext,
  type UserContext,
} from './user-context.js'
export {
  DEFAULT_DEV_TENANT_ID,
  extractTenantId,
  resolveTenantId,
  TenantRequiredError,
  ZITADEL_ORG_CLAIM,
  ZITADEL_RESOURCE_OWNER_CLAIM,
} from './tenant-context.js'
export { checkJwksReachability, type JwksHealthResult } from './jwks-health.js'
export {
  getAuthMetrics,
  recordAuthFailure,
  resetAuthMetricsForTests,
} from './metrics.js'
export {
  ForbiddenError,
  requireAgentAccess,
  resolveInstanceKey,
  type InstanceKey,
} from './access.js'
