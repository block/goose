/**
 * JWT Validation Utilities
 *
 * Pure functions for JWT validation logic.
 * Extracted for easier testing.
 */

/**
 * Check if an azp (authorized party) claim matches permitted origins
 * Supports exact matches and wildcard patterns like https://*.domain.com
 *
 * @param {string} azp - The azp claim from the JWT
 * @param {string[]} permittedOrigins - Array of permitted origin patterns
 * @returns {boolean} True if azp matches any permitted origin
 */
export function matchAzp(azp, permittedOrigins) {
  if (!azp || !permittedOrigins || permittedOrigins.length === 0) {
    return true; // No restrictions if no permitted origins configured
  }

  return permittedOrigins.some(pattern => {
    // Exact match
    if (pattern === azp) return true;

    // Wildcard match: https://*.domain.com matches https://sub.domain.com
    if (pattern.includes('*')) {
      // Escape regex special chars except *, then replace * with [^.]+ (any subdomain segment)
      const escaped = pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp('^' + escaped.replace(/\*/g, '[^.]+') + '$');
      return regex.test(azp);
    }

    return false;
  });
}

/**
 * Validate JWT expiration claims
 *
 * @param {object} decoded - Decoded JWT payload
 * @param {number} [currentTime] - Current time in seconds (defaults to now)
 * @returns {{ valid: boolean, reason?: string }}
 */
export function validateJwtTiming(decoded, currentTime = Math.floor(Date.now() / 1000)) {
  // Check expiration (exp is the time AT which the token expires)
  if (decoded.exp && decoded.exp <= currentTime) {
    return { valid: false, reason: 'expired' };
  }

  // Check not-before
  if (decoded.nbf && decoded.nbf > currentTime) {
    return { valid: false, reason: 'not_yet_valid' };
  }

  return { valid: true };
}

/**
 * Parse a comma-separated list of permitted origins
 *
 * @param {string} originsString - Comma-separated origins (e.g., "https://foo.com,https://*.bar.com")
 * @returns {string[]} Array of origin patterns
 */
export function parsePermittedOrigins(originsString) {
  if (!originsString) return [];
  return originsString.split(',').map(s => s.trim()).filter(Boolean);
}
