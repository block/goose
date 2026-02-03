/**
 * Tests for compare.js version comparison and cache validation
 *
 * These functions determine which updates are needed by comparing
 * app versions against plugin cache versions.
 */

import { describe, it, expect } from 'vitest';
import { compareVersions, validateCacheSchema } from '../../lib/compare.js';

describe('compareVersions', () => {
  describe('standard semver', () => {
    it('returns -1 when v1 < v2', () => {
      expect(compareVersions('0.18.0', '0.19.0')).toBe(-1);
      expect(compareVersions('1.0.0', '2.0.0')).toBe(-1);
      expect(compareVersions('0.0.1', '0.0.2')).toBe(-1);
    });

    it('returns 1 when v1 > v2', () => {
      expect(compareVersions('0.20.0', '0.19.0')).toBe(1);
      expect(compareVersions('2.0.0', '1.0.0')).toBe(1);
      expect(compareVersions('1.0.1', '1.0.0')).toBe(1);
    });

    it('returns 0 when versions are equal', () => {
      expect(compareVersions('0.19.0', '0.19.0')).toBe(0);
      expect(compareVersions('1.0.0', '1.0.0')).toBe(0);
    });

    it('handles different segment counts', () => {
      expect(compareVersions('1.0', '1.0.0')).toBe(0);
      expect(compareVersions('1.0.0', '1.0')).toBe(0);
      expect(compareVersions('1', '1.0.0')).toBe(0);
    });
  });

  describe('dev/preview versions', () => {
    it('treats dev version as newer than same base version', () => {
      // dev > stable for same base = we want to downgrade from dev to stable
      expect(compareVersions('0.19.0-dev', '0.19.0')).toBe(1);
    });

    it('treats stable as older than dev of same base', () => {
      expect(compareVersions('0.19.0', '0.19.0-dev')).toBe(-1);
    });

    it('handles complex dev suffixes', () => {
      expect(compareVersions('0.19.0-dev-preview-50', '0.19.0')).toBe(1);
      expect(compareVersions('0.24.3-dev', '0.24.3')).toBe(1);
    });

    it('compares base versions when both are dev', () => {
      expect(compareVersions('0.18.0-dev', '0.19.0-dev')).toBe(-1);
      expect(compareVersions('0.20.0-dev', '0.19.0-dev')).toBe(1);
      expect(compareVersions('0.19.0-dev', '0.19.0-dev')).toBe(0);
    });

    it('compares base versions correctly regardless of dev suffix', () => {
      expect(compareVersions('0.20.0', '0.24.3-dev')).toBe(-1);
      expect(compareVersions('0.25.0', '0.24.3-dev')).toBe(1);
    });
  });

  describe('edge cases', () => {
    it('returns 0 for null/undefined inputs', () => {
      expect(compareVersions(null, '0.19.0')).toBe(0);
      expect(compareVersions('0.19.0', null)).toBe(0);
      expect(compareVersions(null, null)).toBe(0);
      expect(compareVersions(undefined, undefined)).toBe(0);
    });

    it('handles empty strings', () => {
      expect(compareVersions('', '0.19.0')).toBe(0);
      expect(compareVersions('0.19.0', '')).toBe(0);
    });

    it('handles non-numeric segments gracefully', () => {
      // Non-numeric segments become 0
      expect(compareVersions('abc.def', '0.0.0')).toBe(0);
    });
  });
});

describe('validateCacheSchema', () => {
  it('returns true for valid cache with required keys', () => {
    const cache = {
      imports: {
        'react': 'https://esm.sh/react@19.2.1',
        'use-vibes': 'https://esm.sh/use-vibes@0.19.0'
      }
    };
    expect(validateCacheSchema(cache)).toBe(true);
  });

  it('returns true when only react is present', () => {
    const cache = {
      imports: {
        'react': 'https://esm.sh/react@19.2.1'
      }
    };
    expect(validateCacheSchema(cache)).toBe(true);
  });

  it('returns true when only use-vibes is present', () => {
    const cache = {
      imports: {
        'use-vibes': 'https://esm.sh/use-vibes@0.19.0'
      }
    };
    expect(validateCacheSchema(cache)).toBe(true);
  });

  it('returns false for null', () => {
    expect(validateCacheSchema(null)).toBe(false);
  });

  it('returns false for undefined', () => {
    expect(validateCacheSchema(undefined)).toBe(false);
  });

  it('returns false for non-object', () => {
    expect(validateCacheSchema('string')).toBe(false);
    expect(validateCacheSchema(123)).toBe(false);
    expect(validateCacheSchema([])).toBe(false);
  });

  it('returns false for object without imports', () => {
    const cache = { version: '1.0.0' };
    expect(validateCacheSchema(cache)).toBe(false);
  });

  it('returns false for empty imports', () => {
    const cache = { imports: {} };
    expect(validateCacheSchema(cache)).toBe(false);
  });

  it('returns false when missing required keys', () => {
    const cache = {
      imports: {
        'lodash': 'https://esm.sh/lodash@4.17.21'
      }
    };
    expect(validateCacheSchema(cache)).toBe(false);
  });

  it('returns false when imports is not an object', () => {
    const cache = { imports: 'not-an-object' };
    expect(validateCacheSchema(cache)).toBe(false);
  });
});
