/**
 * Unit tests for registry logic functions
 *
 * Tests the pure logic functions used by the registry server.
 * No I/O, no mocking of external services.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  isSubdomainAvailable,
  getUserClaims,
  getSubdomainsToRelease,
  createClaim,
  releaseClaim,
  processSubscriptionChange
} from '../../lib/registry-logic.js';

describe('isSubdomainAvailable', () => {
  let registry;

  beforeEach(() => {
    registry = {
      claims: {
        'alice': { userId: 'user_1', claimedAt: '2025-01-01T00:00:00Z' },
        'bob': { userId: 'user_2', claimedAt: '2025-01-02T00:00:00Z' }
      },
      reserved: ['admin', 'api', 'www', 'billing'],
      preallocated: {
        'enterprise': 'user_enterprise'
      }
    };
  });

  describe('reserved subdomains', () => {
    it('rejects reserved subdomain names', () => {
      const result = isSubdomainAvailable(registry, 'admin');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('reserved');
    });

    it('rejects reserved names case-insensitively', () => {
      const result = isSubdomainAvailable(registry, 'ADMIN');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('reserved');
    });

    it('rejects all reserved names', () => {
      for (const name of ['api', 'www', 'billing']) {
        const result = isSubdomainAvailable(registry, name);
        expect(result.available).toBe(false);
        expect(result.reason).toBe('reserved');
      }
    });
  });

  describe('preallocated subdomains', () => {
    it('rejects preallocated subdomain names', () => {
      const result = isSubdomainAvailable(registry, 'enterprise');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('preallocated');
      expect(result.ownerId).toBe('user_enterprise');
    });

    it('rejects preallocated names case-insensitively', () => {
      const result = isSubdomainAvailable(registry, 'ENTERPRISE');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('preallocated');
    });
  });

  describe('claimed subdomains', () => {
    it('rejects already claimed subdomain names', () => {
      const result = isSubdomainAvailable(registry, 'alice');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('claimed');
      expect(result.ownerId).toBe('user_1');
    });

    it('rejects claimed names case-insensitively', () => {
      const result = isSubdomainAvailable(registry, 'ALICE');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('claimed');
    });
  });

  describe('subdomain format validation', () => {
    it('rejects subdomains shorter than 3 characters', () => {
      const result = isSubdomainAvailable(registry, 'ab');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('too_short');
    });

    it('rejects single character subdomains', () => {
      const result = isSubdomainAvailable(registry, 'a');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('too_short');
    });

    it('rejects subdomains longer than 63 characters', () => {
      const longName = 'a'.repeat(64);
      const result = isSubdomainAvailable(registry, longName);
      expect(result.available).toBe(false);
      expect(result.reason).toBe('too_long');
    });

    it('rejects subdomains starting with hyphen', () => {
      const result = isSubdomainAvailable(registry, '-invalid');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('invalid_format');
    });

    it('rejects subdomains ending with hyphen', () => {
      const result = isSubdomainAvailable(registry, 'invalid-');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('invalid_format');
    });

    it('rejects subdomains with special characters', () => {
      const result = isSubdomainAvailable(registry, 'inv@lid');
      expect(result.available).toBe(false);
      expect(result.reason).toBe('invalid_format');
    });
  });

  describe('available subdomains', () => {
    it('accepts valid subdomain names', () => {
      const result = isSubdomainAvailable(registry, 'mysite');
      expect(result.available).toBe(true);
    });

    it('accepts subdomains with hyphens', () => {
      const result = isSubdomainAvailable(registry, 'my-site');
      expect(result.available).toBe(true);
    });

    it('accepts subdomains with numbers', () => {
      const result = isSubdomainAvailable(registry, 'site123');
      expect(result.available).toBe(true);
    });

    it('accepts 3 character subdomains', () => {
      const result = isSubdomainAvailable(registry, 'abc');
      expect(result.available).toBe(true);
    });

    it('accepts 63 character subdomains', () => {
      const maxName = 'a'.repeat(63);
      const result = isSubdomainAvailable(registry, maxName);
      expect(result.available).toBe(true);
    });

    it('normalizes to lowercase', () => {
      const result = isSubdomainAvailable(registry, 'MySite');
      expect(result.available).toBe(true);
    });

    it('trims whitespace', () => {
      const result = isSubdomainAvailable(registry, '  mysite  ');
      expect(result.available).toBe(true);
    });
  });

  describe('empty registry', () => {
    it('handles empty registry', () => {
      const emptyRegistry = { claims: {}, reserved: [], preallocated: {} };
      const result = isSubdomainAvailable(emptyRegistry, 'anything');
      expect(result.available).toBe(true);
    });

    it('handles undefined fields', () => {
      const partialRegistry = {};
      const result = isSubdomainAvailable(partialRegistry, 'anything');
      expect(result.available).toBe(true);
    });
  });
});

describe('getUserClaims', () => {
  let registry;

  beforeEach(() => {
    registry = {
      claims: {
        'first': { userId: 'user_1', claimedAt: '2025-01-01T00:00:00Z' },
        'second': { userId: 'user_1', claimedAt: '2025-01-02T00:00:00Z' },
        'third': { userId: 'user_1', claimedAt: '2025-01-03T00:00:00Z' },
        'other': { userId: 'user_2', claimedAt: '2025-01-01T00:00:00Z' }
      },
      reserved: [],
      preallocated: {}
    };
  });

  it('returns all claims for a user', () => {
    const claims = getUserClaims(registry, 'user_1');
    expect(claims).toHaveLength(3);
    expect(claims).toContain('first');
    expect(claims).toContain('second');
    expect(claims).toContain('third');
  });

  it('returns claims sorted by claimedAt descending (newest first)', () => {
    const claims = getUserClaims(registry, 'user_1');
    expect(claims[0]).toBe('third'); // newest
    expect(claims[1]).toBe('second');
    expect(claims[2]).toBe('first'); // oldest
  });

  it('returns empty array for user with no claims', () => {
    const claims = getUserClaims(registry, 'user_nonexistent');
    expect(claims).toEqual([]);
  });

  it('does not include other users claims', () => {
    const claims = getUserClaims(registry, 'user_1');
    expect(claims).not.toContain('other');
  });

  it('handles empty registry', () => {
    const claims = getUserClaims({ claims: {} }, 'user_1');
    expect(claims).toEqual([]);
  });

  it('handles undefined claims', () => {
    const claims = getUserClaims({}, 'user_1');
    expect(claims).toEqual([]);
  });
});

describe('getSubdomainsToRelease', () => {
  let registry;

  beforeEach(() => {
    registry = {
      claims: {
        'oldest': { userId: 'user_1', claimedAt: '2025-01-01T00:00:00Z' },
        'middle': { userId: 'user_1', claimedAt: '2025-01-02T00:00:00Z' },
        'newest': { userId: 'user_1', claimedAt: '2025-01-03T00:00:00Z' }
      },
      reserved: [],
      preallocated: {}
    };
  });

  it('returns empty array when quantity equals claims', () => {
    const toRelease = getSubdomainsToRelease(registry, 'user_1', 3);
    expect(toRelease).toEqual([]);
  });

  it('returns empty array when quantity exceeds claims', () => {
    const toRelease = getSubdomainsToRelease(registry, 'user_1', 5);
    expect(toRelease).toEqual([]);
  });

  it('releases newest first (LIFO) when quantity decreases', () => {
    const toRelease = getSubdomainsToRelease(registry, 'user_1', 2);
    expect(toRelease).toEqual(['newest']);
  });

  it('releases multiple subdomains in LIFO order', () => {
    const toRelease = getSubdomainsToRelease(registry, 'user_1', 1);
    expect(toRelease).toEqual(['newest', 'middle']);
  });

  it('releases all subdomains when quantity is 0', () => {
    const toRelease = getSubdomainsToRelease(registry, 'user_1', 0);
    expect(toRelease).toEqual(['newest', 'middle', 'oldest']);
  });

  it('returns empty for user with no claims', () => {
    const toRelease = getSubdomainsToRelease(registry, 'user_nonexistent', 0);
    expect(toRelease).toEqual([]);
  });
});

describe('createClaim', () => {
  let registry;

  beforeEach(() => {
    registry = {
      claims: {
        'taken': { userId: 'user_1', claimedAt: '2025-01-01T00:00:00Z' }
      },
      reserved: ['admin'],
      preallocated: {}
    };
  });

  it('creates a new claim successfully', () => {
    const result = createClaim(registry, 'newsite', 'user_2');
    expect(result.success).toBe(true);
    expect(result.subdomain).toBe('newsite');
    expect(registry.claims['newsite']).toBeDefined();
    expect(registry.claims['newsite'].userId).toBe('user_2');
  });

  it('normalizes subdomain to lowercase', () => {
    const result = createClaim(registry, 'NewSite', 'user_2');
    expect(result.success).toBe(true);
    expect(result.subdomain).toBe('newsite');
  });

  it('fails for already claimed subdomain', () => {
    const result = createClaim(registry, 'taken', 'user_2');
    expect(result.success).toBe(false);
    expect(result.error).toBe('claimed');
  });

  it('fails for reserved subdomain', () => {
    const result = createClaim(registry, 'admin', 'user_2');
    expect(result.success).toBe(false);
    expect(result.error).toBe('reserved');
  });

  it('fails for invalid subdomain format', () => {
    const result = createClaim(registry, 'ab', 'user_2');
    expect(result.success).toBe(false);
    expect(result.error).toBe('too_short');
  });

  it('sets claimedAt timestamp', () => {
    const before = new Date().toISOString();
    const result = createClaim(registry, 'newsite', 'user_2');
    const after = new Date().toISOString();

    expect(result.success).toBe(true);
    const claimedAt = registry.claims['newsite'].claimedAt;
    expect(claimedAt >= before).toBe(true);
    expect(claimedAt <= after).toBe(true);
  });

  it('creates claims object if undefined', () => {
    const emptyRegistry = { reserved: [], preallocated: {} };
    const result = createClaim(emptyRegistry, 'newsite', 'user_1');
    expect(result.success).toBe(true);
    expect(emptyRegistry.claims).toBeDefined();
    expect(emptyRegistry.claims['newsite']).toBeDefined();
  });
});

describe('releaseClaim', () => {
  let registry;

  beforeEach(() => {
    registry = {
      claims: {
        'mysite': { userId: 'user_1', claimedAt: '2025-01-01T00:00:00Z' }
      },
      reserved: [],
      preallocated: {}
    };
  });

  it('releases an existing claim', () => {
    const result = releaseClaim(registry, 'mysite');
    expect(result).toBe(true);
    expect(registry.claims['mysite']).toBeUndefined();
  });

  it('handles case-insensitive release', () => {
    const result = releaseClaim(registry, 'MySite');
    expect(result).toBe(true);
    expect(registry.claims['mysite']).toBeUndefined();
  });

  it('returns false for non-existent claim', () => {
    const result = releaseClaim(registry, 'nonexistent');
    expect(result).toBe(false);
  });

  it('handles empty registry', () => {
    const result = releaseClaim({ claims: {} }, 'anything');
    expect(result).toBe(false);
  });

  it('handles undefined claims', () => {
    const result = releaseClaim({}, 'anything');
    expect(result).toBe(false);
  });
});

describe('processSubscriptionChange', () => {
  let registry;

  beforeEach(() => {
    registry = {
      claims: {
        'oldest': { userId: 'user_1', claimedAt: '2025-01-01T00:00:00Z' },
        'middle': { userId: 'user_1', claimedAt: '2025-01-02T00:00:00Z' },
        'newest': { userId: 'user_1', claimedAt: '2025-01-03T00:00:00Z' },
        'other-user': { userId: 'user_2', claimedAt: '2025-01-01T00:00:00Z' }
      },
      reserved: [],
      preallocated: {}
    };
  });

  it('releases excess claims when quantity decreases', () => {
    const result = processSubscriptionChange(registry, 'user_1', 2);
    expect(result.released).toEqual(['newest']);
    expect(registry.claims['newest']).toBeUndefined();
    expect(registry.claims['middle']).toBeDefined();
    expect(registry.claims['oldest']).toBeDefined();
  });

  it('releases all claims when subscription is canceled (quantity 0)', () => {
    const result = processSubscriptionChange(registry, 'user_1', 0);
    expect(result.released).toEqual(['newest', 'middle', 'oldest']);
    expect(registry.claims['newest']).toBeUndefined();
    expect(registry.claims['middle']).toBeUndefined();
    expect(registry.claims['oldest']).toBeUndefined();
  });

  it('does not release claims when quantity increases', () => {
    const result = processSubscriptionChange(registry, 'user_1', 5);
    expect(result.released).toEqual([]);
    expect(Object.keys(registry.claims)).toHaveLength(4);
  });

  it('does not affect other users claims', () => {
    processSubscriptionChange(registry, 'user_1', 0);
    expect(registry.claims['other-user']).toBeDefined();
  });

  it('handles user with no claims', () => {
    const result = processSubscriptionChange(registry, 'user_nonexistent', 0);
    expect(result.released).toEqual([]);
  });
});
