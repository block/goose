import { afterEach, describe, expect, it } from 'vitest';

import { isSecurityModelPricingHidden, resolveSecurityPricingMode } from './pricingMode';

describe('pricingMode', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('defaults to enabled when no app config flag is present', () => {
    expect(resolveSecurityPricingMode()).toBe('enabled');
    expect(isSecurityModelPricingHidden()).toBe(false);
  });

  it('hides pricing when Security Goose marks Token Plan pricing as unsupported', () => {
    (window as unknown as Record<string, unknown>).appConfig = {
      get: (key: string) => (key === 'SECURITY_MODEL_PRICING_MODE' ? 'disabled-token-plan' : undefined),
      getAll: () => ({ SECURITY_MODEL_PRICING_MODE: 'disabled-token-plan' }),
    };

    expect(resolveSecurityPricingMode()).toBe('disabled-token-plan');
    expect(isSecurityModelPricingHidden()).toBe(true);
  });
});
