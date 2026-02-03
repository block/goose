/**
 * Registry Logic - Pure functions for subdomain registry operations
 *
 * Extracted from registry-server.ts for easier testing.
 * These functions have no I/O side effects.
 */

/**
 * Check if a subdomain is available
 * @param {object} registry - The registry object
 * @param {Record<string, {userId: string, claimedAt: string}>} registry.claims
 * @param {string[]} registry.reserved
 * @param {Record<string, string>} registry.preallocated
 * @param {string} subdomain - The subdomain to check
 * @returns {{ available: boolean, reason?: string, ownerId?: string }}
 */
export function isSubdomainAvailable(registry, subdomain) {
  const normalized = subdomain.toLowerCase().trim();

  // Check reserved names
  if (registry.reserved && registry.reserved.includes(normalized)) {
    return { available: false, reason: "reserved" };
  }

  // Check preallocated names
  if (registry.preallocated && normalized in registry.preallocated) {
    return { available: false, reason: "preallocated", ownerId: registry.preallocated[normalized] };
  }

  // Check existing claims
  if (registry.claims && normalized in registry.claims) {
    return { available: false, reason: "claimed", ownerId: registry.claims[normalized].userId };
  }

  // Validate subdomain format (alphanumeric and hyphens, 3-63 chars)
  if (normalized.length < 3) {
    return { available: false, reason: "too_short" };
  }

  if (normalized.length > 63) {
    return { available: false, reason: "too_long" };
  }

  if (!/^[a-z0-9][a-z0-9-]*[a-z0-9]$/.test(normalized) && normalized.length > 2) {
    return { available: false, reason: "invalid_format" };
  }

  // Single/double char subdomains have simpler rules
  if (normalized.length <= 2 && !/^[a-z0-9]+$/.test(normalized)) {
    return { available: false, reason: "invalid_format" };
  }

  return { available: true };
}

/**
 * Get all claims for a specific user, sorted by claimedAt descending (LIFO)
 * @param {object} registry - The registry object
 * @param {string} userId - The user ID to look up
 * @returns {string[]} Array of subdomain names, newest first
 */
export function getUserClaims(registry, userId) {
  const claims = [];

  for (const [subdomain, claim] of Object.entries(registry.claims || {})) {
    if (claim.userId === userId) {
      claims.push({ subdomain, claimedAt: claim.claimedAt });
    }
  }

  // Sort by claimedAt descending (newest first) for LIFO release
  claims.sort((a, b) => new Date(b.claimedAt).getTime() - new Date(a.claimedAt).getTime());

  return claims.map((c) => c.subdomain);
}

/**
 * Calculate which subdomains to release when subscription quantity decreases
 * @param {object} registry - The registry object
 * @param {string} userId - The user ID
 * @param {number} newQuantity - The new subscription quantity
 * @returns {string[]} Array of subdomain names to release (LIFO order)
 */
export function getSubdomainsToRelease(registry, userId, newQuantity) {
  const userClaims = getUserClaims(registry, userId);

  if (userClaims.length <= newQuantity) {
    return [];
  }

  // Release newest first (LIFO)
  return userClaims.slice(0, userClaims.length - newQuantity);
}

/**
 * Create a new claim in the registry
 * @param {object} registry - The registry object (will be mutated)
 * @param {string} subdomain - The subdomain to claim
 * @param {string} userId - The user ID claiming it
 * @returns {{ success: boolean, error?: string, subdomain?: string }}
 */
export function createClaim(registry, subdomain, userId) {
  const availability = isSubdomainAvailable(registry, subdomain);

  if (!availability.available) {
    return { success: false, error: availability.reason };
  }

  const normalized = subdomain.toLowerCase().trim();

  if (!registry.claims) {
    registry.claims = {};
  }

  registry.claims[normalized] = {
    userId,
    claimedAt: new Date().toISOString()
  };

  return { success: true, subdomain: normalized };
}

/**
 * Release a subdomain claim
 * @param {object} registry - The registry object (will be mutated)
 * @param {string} subdomain - The subdomain to release
 * @returns {boolean} True if released, false if not found
 */
export function releaseClaim(registry, subdomain) {
  const normalized = subdomain.toLowerCase().trim();

  if (registry.claims && normalized in registry.claims) {
    delete registry.claims[normalized];
    return true;
  }

  return false;
}

/**
 * Process a subscription change webhook
 * @param {object} registry - The registry object (will be mutated)
 * @param {string} userId - The user ID
 * @param {number} newQuantity - The new subscription quantity
 * @returns {{ released: string[] }} Array of released subdomain names
 */
export function processSubscriptionChange(registry, userId, newQuantity) {
  const toRelease = getSubdomainsToRelease(registry, userId, newQuantity);

  for (const subdomain of toRelease) {
    releaseClaim(registry, subdomain);
  }

  return { released: toRelease };
}
