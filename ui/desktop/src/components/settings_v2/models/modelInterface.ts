import { ProviderDetails } from '../../../api';

export default interface Model {
  id?: number; // Make `id` optional to allow user-defined models
  name: string;
  provider: string;
  lastUsed?: string;
  alias?: string; // optional model display name
  subtext?: string; // goes below model name if not the provider
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
