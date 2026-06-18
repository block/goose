function readAppConfigValue(key: string): string | undefined {
  const value = window.appConfig?.get(key);
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

export type SecurityPricingMode = 'enabled' | 'disabled-token-plan';

export function resolveSecurityPricingMode(): SecurityPricingMode {
  const configuredMode = readAppConfigValue('SECURITY_MODEL_PRICING_MODE');
  return configuredMode === 'disabled-token-plan' ? 'disabled-token-plan' : 'enabled';
}

export function isSecurityModelPricingHidden(): boolean {
  return resolveSecurityPricingMode() !== 'enabled';
}
