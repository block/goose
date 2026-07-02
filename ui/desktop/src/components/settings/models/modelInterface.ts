import { listLocalModels } from '../../../acp/local-inference';
import {
  acpGetCanonicalModelInfo,
  acpGetProviderInventoryEntry,
  acpListProviderDetails,
  acpRefreshProviderInventory,
  type ProviderInventoryEntryDto,
  type ProviderInventoryModelDto,
} from '../../../acp/providers';
import type { ProviderDetails, ThinkingEffort } from '../../../types/providers';
import { errorMessage as getErrorMessage } from '../../../utils/conversionUtils';

const INVENTORY_REFRESH_INITIAL_DELAY_MS = 250;
const INVENTORY_REFRESH_POLL_INTERVAL_MS = 500;
const INVENTORY_REFRESH_MAX_ATTEMPTS = 60;

export default interface Model {
  id?: number; // Make `id` optional to allow user-defined models
  name: string;
  provider: string;
  lastUsed?: string;
  alias?: string; // optional model display name
  subtext?: string; // goes below model name if not the provider
  context_limit?: number; // optional context limit override
  reasoning?: boolean; // optional reasoning/thinking support metadata
  request_params?: Record<string, unknown> & { thinking_effort?: ThinkingEffort }; // provider-specific request parameters
}

export function createModelStruct(
  modelName: string,
  provider: string,
  id?: number, // Make `id` optional to allow user-defined models
  lastUsed?: string,
  alias?: string, // optional model display name
  subtext?: string
): Model {
  // use the metadata to create a Model
  return {
    name: modelName,
    provider: provider,
    alias: alias,
    id: id,
    lastUsed: lastUsed,
    subtext: subtext,
  };
}

export async function getProviderMetadata(providerName: string) {
  const providers = await acpListProviderDetails();
  const matches = providers.find((providerMatch) => providerMatch.name === providerName);
  if (!matches) {
    throw Error(`No match for provider: ${providerName}`);
  }
  return matches.metadata;
}

export interface ProviderModelsResult {
  provider: ProviderDetails;
  models: Model[] | null;
  error: string | null;
  warning: string | null;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function providerInventoryModelToModel(
  providerName: string,
  model: ProviderInventoryModelDto
): Model {
  return {
    name: model.id,
    provider: providerName,
    context_limit: model.contextLimit ?? undefined,
    reasoning: model.reasoning ?? undefined,
  };
}

function knownModelsForProvider(provider: ProviderDetails): Model[] {
  return provider.metadata.known_models.map(
    (model) =>
      ({
        name: model.name,
        provider: provider.name,
        context_limit: model.context_limit,
        reasoning: model.reasoning ?? undefined,
      }) as Model
  );
}

async function waitForProviderInventoryRefresh(
  providerName: string
): Promise<ProviderInventoryEntryDto> {
  for (let attempt = 0; attempt < INVENTORY_REFRESH_MAX_ATTEMPTS; attempt++) {
    const entry = await acpGetProviderInventoryEntry(providerName);
    if (!entry) {
      throw new Error(`No inventory entry for provider: ${providerName}`);
    }
    if (!entry.refreshing) {
      return entry;
    }
    await delay(INVENTORY_REFRESH_POLL_INTERVAL_MS);
  }

  throw new Error(`Timed out refreshing models for ${providerName}`);
}

async function fetchInventoryModelsForProvider(
  provider: ProviderDetails
): Promise<{ models: Model[]; warning: string | null }> {
  let entry = await acpGetProviderInventoryEntry(provider.name);
  if (!entry) {
    throw new Error(`No inventory entry for provider: ${provider.name}`);
  }

  const supportsRefresh = provider.supports_refresh ?? entry.supportsRefresh;
  if (supportsRefresh) {
    const refresh = await acpRefreshProviderInventory(provider.name);
    const skippedForRefreshInProgress = refresh.skipped?.some(
      (skip) => skip.providerId === provider.name && skip.reason === 'already_refreshing'
    );
    if (refresh.started.includes(provider.name) || skippedForRefreshInProgress) {
      await delay(INVENTORY_REFRESH_INITIAL_DELAY_MS);
      entry = await waitForProviderInventoryRefresh(provider.name);
    } else {
      entry = (await acpGetProviderInventoryEntry(provider.name)) ?? entry;
    }
  }

  const models = entry.models.map((model) => providerInventoryModelToModel(provider.name, model));
  if (entry.lastRefreshError && models.length === 0) {
    throw new Error(entry.lastRefreshError);
  }

  return {
    models,
    warning: entry.lastRefreshError
      ? 'Could not refresh models from provider - showing cached models instead.'
      : null,
  };
}

export async function fetchModelsForProviders(
  activeProviders: ProviderDetails[]
): Promise<ProviderModelsResult[]> {
  const modelPromises = activeProviders.map(async (p) => {
    try {
      // For local provider, use listLocalModels and filter to only downloaded models
      if (p.name === 'local') {
        const allModels = await listLocalModels();
        const downloadedModels = allModels
          .filter((m) => m.status.state === 'Downloaded')
          .map((m) => ({ name: m.id, provider: p.name }) as Model);
        return { provider: p, models: downloadedModels, error: null, warning: null };
      }

      const { models, warning } = await fetchInventoryModelsForProvider(p);
      return { provider: p, models, error: null, warning };
    } catch (e: unknown) {
      // For custom providers, fall back to the configured model list
      if (p.provider_type === 'Custom') {
        const fallbackModels = knownModelsForProvider(p);
        if (fallbackModels.length > 0) {
          console.warn(`Failed to fetch models for ${p.name}:`, getErrorMessage(e));
          return {
            provider: p,
            models: fallbackModels,
            error: null,
            warning: `Could not fetch models from provider — showing configured models instead.`,
          };
        }
      }

      const errMsg = getErrorMessage(e);
      const errorMessage = `Failed to fetch models for ${p.name}${errMsg ? `: ${errMsg}` : ''}`;
      return {
        provider: p,
        models: null,
        error: errorMessage,
        warning: null,
      };
    }
  });

  return await Promise.all(modelPromises);
}

export async function fetchModelReasoning(
  provider: string,
  model: string,
  fallback?: boolean
): Promise<boolean | null> {
  try {
    const modelInfo = await acpGetCanonicalModelInfo(provider, model);
    return modelInfo?.reasoning ?? fallback ?? null;
  } catch {
    return fallback ?? null;
  }
}
