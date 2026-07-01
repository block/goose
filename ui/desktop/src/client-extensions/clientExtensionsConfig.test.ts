import { describe, expect, it } from 'vitest';
import {
  defaultClientExtensionsConfig,
  isClientExtensionEnabled,
} from '../main/clientExtensionsConfig';

describe('isClientExtensionEnabled', () => {
  it('enables installed extensions by default', () => {
    expect(
      isClientExtensionEnabled('demo', 'installed', defaultClientExtensionsConfig())
    ).toBe(true);
  });

  it('requires opt-in for dev extensions', () => {
    const config = defaultClientExtensionsConfig();
    expect(isClientExtensionEnabled('hello-page', 'dev', config)).toBe(false);

    expect(
      isClientExtensionEnabled('hello-page', 'dev', {
        ...config,
        enabledDev: ['hello-page'],
      })
    ).toBe(true);
  });

  it('respects disabled list for all sources', () => {
    const config = {
      disabled: ['demo', 'hello-page'],
      enabledDev: ['hello-page'],
    };
    expect(isClientExtensionEnabled('demo', 'installed', config)).toBe(false);
    expect(isClientExtensionEnabled('hello-page', 'dev', config)).toBe(false);
  });
});
