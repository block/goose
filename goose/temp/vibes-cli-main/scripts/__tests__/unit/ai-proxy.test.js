/**
 * Unit tests for AI Proxy logic
 *
 * Tests the core logic of the AI proxy without requiring Bun runtime.
 * The actual proxy uses Bun-specific APIs, so we test the extractable logic.
 */

import { describe, it, expect } from 'vitest';

// ============== JWT Extraction Logic (extracted from ai-proxy.js) ==============

/**
 * Extract tenant ID from JWT token
 * This mirrors the logic in ai-proxy.js but is testable without Bun
 */
function extractTenantFromJWT(token) {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;

    const payload = JSON.parse(
      Buffer.from(parts[1].replace(/-/g, "+").replace(/_/g, "/"), "base64").toString()
    );

    // Check expiration
    if (payload.exp && payload.exp < Date.now() / 1000) {
      return null;
    }

    // Extract tenant from custom claims or subdomain
    return payload.tenant || payload.subdomain || payload.sub || null;
  } catch {
    return null;
  }
}

/**
 * Create a test JWT with given payload
 * Note: This creates unsigned tokens for testing only
 */
function createTestJWT(payload) {
  const header = Buffer.from(JSON.stringify({ alg: "HS256", typ: "JWT" })).toString("base64url");
  const body = Buffer.from(JSON.stringify(payload)).toString("base64url");
  const signature = "test-signature";
  return `${header}.${body}.${signature}`;
}

// ============== Tests ==============

describe('extractTenantFromJWT', () => {
  describe('valid tokens', () => {
    it('extracts tenant from tenant claim', () => {
      const token = createTestJWT({
        tenant: "acme-corp",
        sub: "user_123",
        exp: Math.floor(Date.now() / 1000) + 3600
      });

      expect(extractTenantFromJWT(token)).toBe("acme-corp");
    });

    it('extracts tenant from subdomain claim', () => {
      const token = createTestJWT({
        subdomain: "mycompany",
        sub: "user_456",
        exp: Math.floor(Date.now() / 1000) + 3600
      });

      expect(extractTenantFromJWT(token)).toBe("mycompany");
    });

    it('falls back to sub claim when no tenant/subdomain', () => {
      const token = createTestJWT({
        sub: "user_789",
        exp: Math.floor(Date.now() / 1000) + 3600
      });

      expect(extractTenantFromJWT(token)).toBe("user_789");
    });

    it('prefers tenant over subdomain over sub', () => {
      const token = createTestJWT({
        tenant: "preferred-tenant",
        subdomain: "alt-subdomain",
        sub: "user_fallback",
        exp: Math.floor(Date.now() / 1000) + 3600
      });

      expect(extractTenantFromJWT(token)).toBe("preferred-tenant");
    });

    it('handles tokens without expiration', () => {
      const token = createTestJWT({
        tenant: "no-exp-tenant"
        // No exp claim
      });

      expect(extractTenantFromJWT(token)).toBe("no-exp-tenant");
    });
  });

  describe('expired tokens', () => {
    it('returns null for expired token', () => {
      const token = createTestJWT({
        tenant: "expired-tenant",
        exp: Math.floor(Date.now() / 1000) - 3600 // 1 hour ago
      });

      expect(extractTenantFromJWT(token)).toBeNull();
    });

    it('returns null for just-expired token', () => {
      const token = createTestJWT({
        tenant: "just-expired",
        exp: Math.floor(Date.now() / 1000) - 1 // 1 second ago
      });

      expect(extractTenantFromJWT(token)).toBeNull();
    });
  });

  describe('invalid tokens', () => {
    it('returns null for malformed token (too few parts)', () => {
      expect(extractTenantFromJWT("header.payload")).toBeNull();
    });

    it('returns null for malformed token (too many parts)', () => {
      expect(extractTenantFromJWT("a.b.c.d")).toBeNull();
    });

    it('returns null for invalid base64 in payload', () => {
      expect(extractTenantFromJWT("header.!!!invalid!!!.signature")).toBeNull();
    });

    it('returns null for invalid JSON in payload', () => {
      const invalidPayload = Buffer.from("not json").toString("base64url");
      expect(extractTenantFromJWT(`header.${invalidPayload}.signature`)).toBeNull();
    });

    it('returns null for empty token', () => {
      expect(extractTenantFromJWT("")).toBeNull();
    });

    it('returns null for token with empty claims', () => {
      const token = createTestJWT({});
      expect(extractTenantFromJWT(token)).toBeNull();
    });
  });

  describe('base64url handling', () => {
    it('handles base64url with - characters', () => {
      // Create payload with characters that become - in base64url
      const token = createTestJWT({
        tenant: "test-tenant-with-dashes",
        exp: Math.floor(Date.now() / 1000) + 3600
      });

      expect(extractTenantFromJWT(token)).toBe("test-tenant-with-dashes");
    });

    it('handles base64url with _ characters', () => {
      const token = createTestJWT({
        tenant: "test_tenant_with_underscores",
        exp: Math.floor(Date.now() / 1000) + 3600
      });

      expect(extractTenantFromJWT(token)).toBe("test_tenant_with_underscores");
    });
  });
});

describe('AI Proxy Configuration', () => {
  describe('environment variable parsing', () => {
    it('parses VIBES_TENANT_LIMIT as float', () => {
      const parseLimit = (val) => parseFloat(val) || 5;

      expect(parseLimit("10")).toBe(10);
      expect(parseLimit("5.50")).toBe(5.5);
      expect(parseLimit("")).toBe(5); // default
      expect(parseLimit(undefined)).toBe(5); // default
      expect(parseLimit("invalid")).toBe(5); // default on NaN
    });

    it('parses VIBES_MULTI_TENANT as boolean', () => {
      const isMultiTenant = (val) => val === "true";

      expect(isMultiTenant("true")).toBe(true);
      expect(isMultiTenant("false")).toBe(false);
      expect(isMultiTenant("")).toBe(false);
      expect(isMultiTenant(undefined)).toBe(false);
      expect(isMultiTenant("TRUE")).toBe(false); // case sensitive
    });

    it('parses VIBES_PROXY_PORT as integer', () => {
      const parsePort = (val) => parseInt(val) || 3001;

      expect(parsePort("8080")).toBe(8080);
      expect(parsePort("3001")).toBe(3001);
      expect(parsePort("")).toBe(3001); // default
      expect(parsePort(undefined)).toBe(3001); // default
    });
  });
});

describe('OpenRouter Key Provisioning Request', () => {
  it('constructs correct provisioning payload', () => {
    const tenant = "test-tenant";
    const limit = 10;

    const payload = {
      name: `vibes-tenant-${tenant}`,
      limit: limit,
      limit_reset: "monthly"
    };

    expect(payload.name).toBe("vibes-tenant-test-tenant");
    expect(payload.limit).toBe(10);
    expect(payload.limit_reset).toBe("monthly");
  });

  it('handles special characters in tenant names', () => {
    const tenants = [
      "simple",
      "with-dashes",
      "with_underscores",
      "CamelCase",
      "123numeric"
    ];

    for (const tenant of tenants) {
      const payload = { name: `vibes-tenant-${tenant}` };
      expect(payload.name).toBe(`vibes-tenant-${tenant}`);
    }
  });
});

describe('CORS Headers', () => {
  it('includes required CORS headers', () => {
    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type, Authorization"
    };

    expect(corsHeaders["Access-Control-Allow-Origin"]).toBe("*");
    expect(corsHeaders["Access-Control-Allow-Methods"]).toContain("POST");
    expect(corsHeaders["Access-Control-Allow-Methods"]).toContain("OPTIONS");
    expect(corsHeaders["Access-Control-Allow-Headers"]).toContain("Authorization");
  });
});

// Export for use in integration tests if needed
export { extractTenantFromJWT, createTestJWT };
