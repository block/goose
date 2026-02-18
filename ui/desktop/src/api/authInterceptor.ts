/**
 * Auth interceptor for the generated API client.
 *
 * Reads the session token from localStorage and attaches it as a
 * Bearer token on every outgoing request. Also attaches the secret key
 * for server authentication.
 *
 * This ensures all API calls carry the user's identity for:
 * - Tenant-scoped session isolation
 * - Dual identity propagation (user + agent)
 * - OIDC-validated request context
 *
 * This file is NOT auto-generated — it's a manual integration point
 * between the auth system (useAuth hook / LoginView) and the generated
 * API client (@hey-api/openapi-ts).
 */

import { client } from './client.gen';

const TOKEN_KEY = 'goose_auth_token';
const SECRET_KEY_STORAGE = 'secretKey';

/**
 * Initialize the auth interceptor on the API client.
 * Call this once at app startup (before any API calls).
 *
 * The interceptor adds two headers to every request:
 * 1. Authorization: Bearer <session_token> — user identity (OIDC/API key/guest)
 * 2. X-Secret-Key: <secret> — server auth (existing check_token middleware)
 */
export function setupAuthInterceptor(): void {
  client.interceptors.request.use((request) => {
    // Attach user identity token (from login flow)
    const token = localStorage.getItem(TOKEN_KEY);
    if (token && !request.headers.has('Authorization')) {
      request.headers.set('Authorization', `Bearer ${token}`);
    }

    // Attach server secret key (existing auth mechanism)
    const secretKey = localStorage.getItem(SECRET_KEY_STORAGE);
    if (secretKey && !request.headers.has('X-Secret-Key')) {
      request.headers.set('X-Secret-Key', secretKey);
    }

    return request;
  });
}
