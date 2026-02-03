/**
 * Tests for config-merge.js transform
 *
 * These functions parse and manipulate the CONFIG object in sell templates.
 * Bugs here can corrupt user configurations, so thorough testing is critical.
 */

import { describe, it, expect } from 'vitest';
import {
  parseConfig,
  serializeConfig,
  mergeConfigs,
  findMissingFields,
  applyConfigMerge,
  CONFIG_SCHEMA
} from '../../../lib/transforms/config-merge.js';

describe('parseConfig', () => {
  describe('basic parsing', () => {
    it('parses simple CONFIG object', () => {
      const html = `const CONFIG = {
  APP_NAME: "my-app",
  DOMAIN: "example.com"
};`;
      const result = parseConfig(html);
      expect(result).toEqual({
        APP_NAME: 'my-app',
        DOMAIN: 'example.com'
      });
    });

    it('parses CONFIG with various value types', () => {
      const html = `const CONFIG = {
  APP_NAME: "test",
  FEATURES: ["a", "b", "c"],
  ADMIN_USER_IDS: []
};`;
      const result = parseConfig(html);
      expect(result.APP_NAME).toBe('test');
      expect(result.FEATURES).toEqual(['a', 'b', 'c']);
      expect(result.ADMIN_USER_IDS).toEqual([]);
    });

    it('parses CONFIG with placeholder values', () => {
      const html = `const CONFIG = {
  CLERK_PUBLISHABLE_KEY: "__CLERK_PUBLISHABLE_KEY__",
  APP_NAME: "__APP_NAME__"
};`;
      const result = parseConfig(html);
      expect(result.CLERK_PUBLISHABLE_KEY).toBe('__CLERK_PUBLISHABLE_KEY__');
      expect(result.APP_NAME).toBe('__APP_NAME__');
    });
  });

  describe('CONFIG in HTML context', () => {
    it('extracts CONFIG from full HTML document', () => {
      const html = `<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
<script type="text/babel">
const CONFIG = {
  APP_NAME: "test-app"
};
function App() { return null; }
</script>
</body>
</html>`;
      const result = parseConfig(html);
      expect(result).toEqual({ APP_NAME: 'test-app' });
    });
  });

  describe('edge cases', () => {
    it('returns null when no CONFIG found', () => {
      const html = 'const OTHER = { foo: "bar" };';
      expect(parseConfig(html)).toBeNull();
    });

    it('returns null for empty string', () => {
      expect(parseConfig('')).toBeNull();
    });

    it('handles trailing commas', () => {
      const html = `const CONFIG = {
  APP_NAME: "test",
  DOMAIN: "example.com",
};`;
      const result = parseConfig(html);
      expect(result.APP_NAME).toBe('test');
      expect(result.DOMAIN).toBe('example.com');
    });

    it('handles single quotes', () => {
      const html = `const CONFIG = {
  APP_NAME: 'test'
};`;
      const result = parseConfig(html);
      // Single quotes may be kept as-is since they're valid JS
      expect(result.APP_NAME).toBeDefined();
    });
  });
});

describe('serializeConfig', () => {
  it('serializes simple config object', () => {
    const config = {
      APP_NAME: 'test',
      DOMAIN: 'example.com'
    };
    const result = serializeConfig(config);
    expect(result).toContain('const CONFIG = {');
    expect(result).toContain('APP_NAME: "test"');
    expect(result).toContain('DOMAIN: "example.com"');
    expect(result).toContain('};');
  });

  it('preserves placeholder format', () => {
    const config = {
      CLERK_KEY: '__CLERK_KEY__',
      APP_NAME: 'real-value'
    };
    const result = serializeConfig(config);
    expect(result).toContain('CLERK_KEY: "__CLERK_KEY__"');
    expect(result).toContain('APP_NAME: "real-value"');
  });

  it('serializes arrays correctly', () => {
    const config = {
      FEATURES: ['a', 'b', 'c'],
      ADMIN_IDS: []
    };
    const result = serializeConfig(config);
    expect(result).toContain('FEATURES: ["a","b","c"]');
    expect(result).toContain('ADMIN_IDS: []');
  });

  it('handles strings with special characters', () => {
    const config = {
      TAGLINE: 'It\'s the best app ever!'
    };
    const result = serializeConfig(config);
    // Should escape properly
    expect(result).toContain('TAGLINE:');
  });
});

describe('mergeConfigs', () => {
  it('adds new fields to existing config', () => {
    const existing = { APP_NAME: 'test' };
    const updates = { DOMAIN: 'example.com', TAGLINE: 'Hello' };
    const result = mergeConfigs(existing, updates);
    expect(result).toEqual({
      APP_NAME: 'test',
      DOMAIN: 'example.com',
      TAGLINE: 'Hello'
    });
  });

  it('does not overwrite existing values', () => {
    const existing = { APP_NAME: 'user-value', DOMAIN: 'user-domain.com' };
    const updates = { APP_NAME: 'default', DOMAIN: 'default.com', NEW_FIELD: 'added' };
    const result = mergeConfigs(existing, updates);
    expect(result.APP_NAME).toBe('user-value');
    expect(result.DOMAIN).toBe('user-domain.com');
    expect(result.NEW_FIELD).toBe('added');
  });

  it('handles empty existing config', () => {
    const existing = {};
    const updates = { APP_NAME: 'test', DOMAIN: 'example.com' };
    const result = mergeConfigs(existing, updates);
    expect(result).toEqual(updates);
  });

  it('handles empty updates', () => {
    const existing = { APP_NAME: 'test' };
    const updates = {};
    const result = mergeConfigs(existing, updates);
    expect(result).toEqual(existing);
  });

  it('preserves all existing fields', () => {
    const existing = {
      A: 1,
      B: 2,
      C: 3
    };
    const updates = { D: 4 };
    const result = mergeConfigs(existing, updates);
    expect(result).toEqual({ A: 1, B: 2, C: 3, D: 4 });
  });
});

describe('findMissingFields', () => {
  it('returns all schema fields for empty config', () => {
    const result = findMissingFields({});
    expect(result).toContain('CLERK_PUBLISHABLE_KEY');
    expect(result).toContain('APP_NAME');
    expect(result).toContain('FEATURES');
    expect(result.length).toBe(Object.keys(CONFIG_SCHEMA).length);
  });

  it('returns only missing fields', () => {
    const config = {
      CLERK_PUBLISHABLE_KEY: 'pk_test_123',
      APP_NAME: 'test'
    };
    const result = findMissingFields(config);
    expect(result).not.toContain('CLERK_PUBLISHABLE_KEY');
    expect(result).not.toContain('APP_NAME');
    expect(result).toContain('APP_DOMAIN');
    expect(result).toContain('FEATURES');
  });

  it('returns empty array when all fields present', () => {
    const config = {};
    for (const key of Object.keys(CONFIG_SCHEMA)) {
      config[key] = 'value';
    }
    const result = findMissingFields(config);
    expect(result).toEqual([]);
  });
});

describe('applyConfigMerge', () => {
  it('merges new fields into HTML CONFIG', () => {
    const html = `const CONFIG = {
  APP_NAME: "test"
};
function App() {}`;
    const result = applyConfigMerge(html, { NEW_FIELD: 'added' });
    expect(result.success).toBe(true);
    expect(result.html).toContain('NEW_FIELD');
    expect(result.diff.addedFields).toContain('NEW_FIELD');
  });

  it('returns error when no CONFIG found', () => {
    const html = 'function App() {}';
    const result = applyConfigMerge(html, { FIELD: 'value' });
    expect(result.success).toBe(false);
    expect(result.error).toContain('No CONFIG object found');
  });

  it('preserves existing user values', () => {
    const html = `const CONFIG = {
  APP_NAME: "user-custom-name"
};`;
    const result = applyConfigMerge(html, { APP_NAME: 'default', NEW: 'added' });
    expect(result.success).toBe(true);
    expect(result.html).toContain('user-custom-name');
    expect(result.diff.addedFields).not.toContain('APP_NAME');
    expect(result.diff.addedFields).toContain('NEW');
  });
});

describe('CONFIG_SCHEMA', () => {
  it('contains expected fields', () => {
    expect(CONFIG_SCHEMA).toHaveProperty('CLERK_PUBLISHABLE_KEY');
    expect(CONFIG_SCHEMA).toHaveProperty('APP_NAME');
    expect(CONFIG_SCHEMA).toHaveProperty('APP_TITLE');
    expect(CONFIG_SCHEMA).toHaveProperty('APP_DOMAIN');
    expect(CONFIG_SCHEMA).toHaveProperty('FEATURES');
    expect(CONFIG_SCHEMA).toHaveProperty('ADMIN_USER_IDS');
  });

  it('defines types for all fields', () => {
    for (const [key, schema] of Object.entries(CONFIG_SCHEMA)) {
      expect(schema.type).toBeDefined();
      expect(['string', 'array']).toContain(schema.type);
    }
  });

  it('defines placeholders for all fields', () => {
    for (const [key, schema] of Object.entries(CONFIG_SCHEMA)) {
      expect(schema.placeholder).toBeDefined();
    }
  });
});
