/**
 * Unit tests for JWT validation utilities
 *
 * Tests the azp matching and JWT timing validation.
 */

import { describe, it, expect } from 'vitest';
import {
  matchAzp,
  validateJwtTiming,
  parsePermittedOrigins
} from '../../lib/jwt-validation.js';

describe('matchAzp', () => {
  describe('exact matching', () => {
    it('matches exact origin', () => {
      const result = matchAzp('https://example.com', ['https://example.com']);
      expect(result).toBe(true);
    });

    it('rejects non-matching origin', () => {
      const result = matchAzp('https://other.com', ['https://example.com']);
      expect(result).toBe(false);
    });

    it('matches one of multiple permitted origins', () => {
      const result = matchAzp('https://second.com', [
        'https://first.com',
        'https://second.com',
        'https://third.com'
      ]);
      expect(result).toBe(true);
    });

    it('is case-sensitive', () => {
      const result = matchAzp('https://Example.com', ['https://example.com']);
      expect(result).toBe(false);
    });
  });

  describe('wildcard matching', () => {
    it('matches subdomain with wildcard pattern', () => {
      const result = matchAzp('https://test.example.com', ['https://*.example.com']);
      expect(result).toBe(true);
    });

    it('matches any subdomain', () => {
      const patterns = ['https://*.cronos.computer'];
      expect(matchAzp('https://alice.cronos.computer', patterns)).toBe(true);
      expect(matchAzp('https://bob.cronos.computer', patterns)).toBe(true);
      expect(matchAzp('https://test123.cronos.computer', patterns)).toBe(true);
    });

    it('does not match root domain with subdomain wildcard', () => {
      const result = matchAzp('https://cronos.computer', ['https://*.cronos.computer']);
      expect(result).toBe(false);
    });

    it('does not match nested subdomains with single wildcard', () => {
      // *.example.com should match sub.example.com but not deep.sub.example.com
      const result = matchAzp('https://deep.sub.example.com', ['https://*.example.com']);
      expect(result).toBe(false);
    });

    it('matches combined exact and wildcard patterns', () => {
      const patterns = ['https://cronos.computer', 'https://*.cronos.computer'];
      expect(matchAzp('https://cronos.computer', patterns)).toBe(true);
      expect(matchAzp('https://test.cronos.computer', patterns)).toBe(true);
    });

    it('does not match different TLD', () => {
      const result = matchAzp('https://test.cronos.net', ['https://*.cronos.computer']);
      expect(result).toBe(false);
    });

    it('handles wildcard at different positions', () => {
      // This is an edge case - wildcard in middle
      const result = matchAzp('https://api.v1.example.com', ['https://api.*.example.com']);
      expect(result).toBe(true);
    });
  });

  describe('edge cases', () => {
    it('returns true when no permitted origins configured', () => {
      expect(matchAzp('https://any.com', [])).toBe(true);
      expect(matchAzp('https://any.com', null)).toBe(true);
      expect(matchAzp('https://any.com', undefined)).toBe(true);
    });

    it('returns true when azp is empty and no restrictions', () => {
      expect(matchAzp('', [])).toBe(true);
      expect(matchAzp(null, [])).toBe(true);
    });

    it('handles special regex characters in domain', () => {
      // Dots should be escaped properly
      const result = matchAzp('https://testXexample.com', ['https://*.example.com']);
      expect(result).toBe(false); // X should not match escaped dot
    });

    it('handles ports in URLs', () => {
      const result = matchAzp('https://test.example.com:3000', ['https://*.example.com:3000']);
      expect(result).toBe(true);
    });
  });
});

describe('validateJwtTiming', () => {
  const NOW = 1700000000; // Fixed timestamp for testing

  describe('expiration', () => {
    it('accepts non-expired token', () => {
      const result = validateJwtTiming({ exp: NOW + 3600 }, NOW);
      expect(result.valid).toBe(true);
    });

    it('rejects expired token', () => {
      const result = validateJwtTiming({ exp: NOW - 1 }, NOW);
      expect(result.valid).toBe(false);
      expect(result.reason).toBe('expired');
    });

    it('rejects token that just expired', () => {
      const result = validateJwtTiming({ exp: NOW }, NOW);
      expect(result.valid).toBe(false);
      expect(result.reason).toBe('expired');
    });

    it('accepts token without exp claim', () => {
      const result = validateJwtTiming({}, NOW);
      expect(result.valid).toBe(true);
    });
  });

  describe('not-before', () => {
    it('accepts token that is valid now', () => {
      const result = validateJwtTiming({ nbf: NOW - 1 }, NOW);
      expect(result.valid).toBe(true);
    });

    it('accepts token that becomes valid exactly now', () => {
      const result = validateJwtTiming({ nbf: NOW }, NOW);
      expect(result.valid).toBe(true);
    });

    it('rejects token not yet valid', () => {
      const result = validateJwtTiming({ nbf: NOW + 1 }, NOW);
      expect(result.valid).toBe(false);
      expect(result.reason).toBe('not_yet_valid');
    });

    it('accepts token without nbf claim', () => {
      const result = validateJwtTiming({}, NOW);
      expect(result.valid).toBe(true);
    });
  });

  describe('combined claims', () => {
    it('accepts valid token with both exp and nbf', () => {
      const result = validateJwtTiming({ exp: NOW + 3600, nbf: NOW - 60 }, NOW);
      expect(result.valid).toBe(true);
    });

    it('rejects expired token even if nbf is valid', () => {
      const result = validateJwtTiming({ exp: NOW - 1, nbf: NOW - 3600 }, NOW);
      expect(result.valid).toBe(false);
      expect(result.reason).toBe('expired');
    });

    it('rejects not-yet-valid token even if exp is in future', () => {
      const result = validateJwtTiming({ exp: NOW + 3600, nbf: NOW + 60 }, NOW);
      expect(result.valid).toBe(false);
      expect(result.reason).toBe('not_yet_valid');
    });
  });

  describe('default time', () => {
    it('uses current time when not specified', () => {
      const futureExp = Math.floor(Date.now() / 1000) + 3600;
      const result = validateJwtTiming({ exp: futureExp });
      expect(result.valid).toBe(true);
    });
  });
});

describe('parsePermittedOrigins', () => {
  it('parses comma-separated origins', () => {
    const result = parsePermittedOrigins('https://a.com,https://b.com,https://c.com');
    expect(result).toEqual(['https://a.com', 'https://b.com', 'https://c.com']);
  });

  it('trims whitespace', () => {
    const result = parsePermittedOrigins('  https://a.com , https://b.com  ');
    expect(result).toEqual(['https://a.com', 'https://b.com']);
  });

  it('filters empty entries', () => {
    const result = parsePermittedOrigins('https://a.com,,https://b.com,');
    expect(result).toEqual(['https://a.com', 'https://b.com']);
  });

  it('returns empty array for empty string', () => {
    expect(parsePermittedOrigins('')).toEqual([]);
  });

  it('returns empty array for null/undefined', () => {
    expect(parsePermittedOrigins(null)).toEqual([]);
    expect(parsePermittedOrigins(undefined)).toEqual([]);
  });

  it('handles wildcard patterns', () => {
    const result = parsePermittedOrigins('https://example.com,https://*.example.com');
    expect(result).toEqual(['https://example.com', 'https://*.example.com']);
  });
});
