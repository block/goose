import os from 'node:os';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { resolveGoosePathRoot, sanitizeGoosePathRoot } from './pathUtils';

describe('resolveGoosePathRoot', () => {
  it('rejects empty and relative values', () => {
    expect(resolveGoosePathRoot(undefined)).toBeUndefined();
    expect(resolveGoosePathRoot('   ')).toBeUndefined();
    expect(resolveGoosePathRoot('relative/root')).toBeUndefined();
  });

  it('retains absolute paths without requiring them to exist', () => {
    const absolute = path.resolve('nonexistent-goose-root');
    expect(resolveGoosePathRoot(`  ${absolute}  `)).toBe(absolute);
  });

  it('expands a home-relative root before validation', () => {
    expect(resolveGoosePathRoot('~')).toBe(os.homedir());
  });

  it('removes a rejected value from the child-process environment', () => {
    const env = { GOOSE_PATH_ROOT: 'relative/root' };
    expect(sanitizeGoosePathRoot(env)).toBeUndefined();
    expect(env).not.toHaveProperty('GOOSE_PATH_ROOT');
  });
});
