import { acpReadAllConfig } from '../../acp/config';

export const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';
export const ONBOARDING_TELEMETRY_PENDING_CONFIG_KEY = 'GOOSE_ONBOARDING_TELEMETRY_PENDING';

export async function readOnboardingTelemetryPending(): Promise<unknown> {
  const config = await acpReadAllConfig();
  return config[ONBOARDING_TELEMETRY_PENDING_CONFIG_KEY] ?? null;
}
