import { describe, expect, it } from 'vitest';
import { resources } from './resources';

function flattenKeys(obj: unknown, prefix = ''): string[] {
  if (Array.isArray(obj)) {
    return [];
  }
  if (!obj || typeof obj !== 'object') {
    return [];
  }

  return Object.entries(obj as Record<string, unknown>).flatMap(([key, value]) => {
    const nextPrefix = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      return [nextPrefix, ...flattenKeys(value, nextPrefix)];
    }
    return [nextPrefix];
  });
}

describe('i18n resources parity', () => {
  it('keeps zh-Hans and zh-Hant translation keys aligned with en', () => {
    const en = resources.en.translation;
    const zhHans = resources['zh-Hans'].translation;
    const zhHant = resources['zh-Hant'].translation;

    const enKeys = new Set(flattenKeys(en));
    const zhHansKeys = new Set(flattenKeys(zhHans));
    const zhHantKeys = new Set(flattenKeys(zhHant));

    expect(zhHansKeys).toEqual(enKeys);
    expect(zhHantKeys).toEqual(enKeys);
  });
});
