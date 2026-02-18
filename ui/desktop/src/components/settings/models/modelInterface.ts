import { ProviderDetails, ModelInfo, getProviderModelInfo } from '../../../api';
import { errorMessage as getErrorMessage } from '../../../utils/conversionUtils';

export default interface Model {
  id?: number; // Make `id` optional to allow user-defined models
  name: string;
  provider: string;
  lastUsed?: string;
  alias?: string; // optional model display name
  subtext?: string; // goes below model name if not the provider
  context_limit?: number; // optional context limit override
  request_params?: Record<string, unknown>; // provider-specific request parameters
  variant?: string; // reasoning effort variant (low, medium, high, max)
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

export async function getProviderMetadata(
  providerName: string,
  getProvidersFunc: (b: boolean) => Promise<ProviderDetails[]>
) {
  const providers = await getProvidersFunc(false);
  const matches = providers.find((providerMatch) => providerMatch.name === providerName);
  if (!matches) {
    throw Error(`No match for provider: ${providerName}`);
  }
  return matches.metadata;
}

export interface ProviderModelsResult {
  provider: ProviderDetails;
  models: string[] | null;
  modelInfo: ModelInfo[] | null;
  error: string | null;
}

export function formatModelHint(info: ModelInfo): string {
  const parts: string[] = [];
  if (info.supports_reasoning) {
    parts.push('reasoning');
  }
  if (info.input_token_cost != null && info.output_token_cost != null) {
    const inputPerM = (info.input_token_cost * 1_000_000).toFixed(2);
    const outputPerM = (info.output_token_cost * 1_000_000).toFixed(2);
    parts.push(`$${inputPerM}/$${outputPerM} per 1M tokens`);
  }
  return parts.join(' · ');
}

export async function fetchModelsForProviders(
  activeProviders: ProviderDetails[]
): Promise<ProviderModelsResult[]> {
  const modelPromises = activeProviders.map(async (p) => {
    try {
      const response = await getProviderModelInfo({
        path: { name: p.name },
        throwOnError: true,
      });
      const infoList: ModelInfo[] = response.data || [];
      const models = infoList.map((m) => m.name);
      return { provider: p, models, modelInfo: infoList, error: null };
    } catch (e: unknown) {
      const errMsg = getErrorMessage(e);
      const errorMessage = `Failed to fetch models for ${p.name}${errMsg ? `: ${errMsg}` : ''}`;
      return {
        provider: p,
        models: null,
        modelInfo: null,
        error: errorMessage,
      };
    }
  });

  return await Promise.all(modelPromises);
}
