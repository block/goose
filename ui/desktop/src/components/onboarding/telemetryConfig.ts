import { acpReadAllConfig } from '../../acp/config';

export const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';
export const ONBOARDING_TELEMETRY_PENDING_CONFIG_KEY = 'GOOSE_ONBOARDING_TELEMETRY_PENDING';

export async function readOnboardingTelemetryState(): Promise<{
  pending: unknown;
  persistedPreference: unknown;
}> {
  const config = await acpReadAllConfig();
  return {
    pending: config[ONBOARDING_TELEMETRY_PENDING_CONFIG_KEY] ?? null,
    persistedPreference: config[TELEMETRY_CONFIG_KEY] ?? null,
  };
}

export async function readOnboardingTelemetryPending(): Promise<unknown> {
  return (await readOnboardingTelemetryState()).pending;
}
