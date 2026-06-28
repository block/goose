/**
 * Resolve the effective context window for a provider/model pair.
 * Uses the goosed model-info endpoint so GOOSE_CONTEXT_LIMIT and provider
 * defaults match what the backend uses for compaction.
 */

import { getProviderModelInfo } from '../api';
import { acpReadConfig } from '../acp/config';

export const DEFAULT_CONTEXT_LIMIT = 128_000;

export async function fetchResolvedModelContextLimit(
  provider: string,
  model: string
): Promise<number | null> {
  try {
    const response = await getProviderModelInfo({
      path: { name: provider },
      body: { model },
    });
    const limit = response.data?.context_limit;
    if (typeof limit === 'number' && limit > 0) {
      return limit;
    }
  } catch {
    // Provider may be unconfigured during onboarding.
  }
  return null;
}

export async function readConfiguredContextLimit(): Promise<number | null> {
  try {
    const value = await acpReadConfig('GOOSE_CONTEXT_LIMIT', false);
    if (typeof value === 'number' && value > 0) {
      return value;
    }
  } catch {
    // Key not set.
  }
  return null;
}

export async function resolveDisplayContextLimit(
  provider: string,
  model: string
): Promise<number> {
  const resolved = await fetchResolvedModelContextLimit(provider, model);
  if (resolved !== null) {
    return resolved;
  }

  const configured = await readConfiguredContextLimit();
  if (configured !== null) {
    return configured;
  }

  return DEFAULT_CONTEXT_LIMIT;
}
