import type {
  CanonicalModelInfoDto,
  CustomProviderCreateRequest_unstable,
  CustomProviderReadResponse_unstable,
  ProviderSecretDto,
  ProviderTemplateCatalogEntryDto,
  ProviderTemplateDto,
} from '@aaif/goose-sdk';
import type { ProviderDetails, ThinkingEffort, UpdateCustomProviderRequest } from '../api';
import { getAcpClient } from './acpConnection';

export type { CanonicalModelInfoDto, ProviderSecretDto };

function updateRequestToCreate(
  request: UpdateCustomProviderRequest
): CustomProviderCreateRequest_unstable {
  return {
    engine: request.engine,
    displayName: request.display_name,
    apiUrl: request.api_url,
    apiKey: request.api_key || null,
    models: request.models,
    supportsStreaming: request.supports_streaming ?? null,
    headers: request.headers ?? undefined,
    requiresAuth: request.requires_auth ?? true,
    catalogProviderId: request.catalog_provider_id ?? null,
    basePath: request.base_path ?? null,
    preservesThinking: request.preserves_thinking ?? null,
  };
}

export async function acpListProviderDetails(): Promise<ProviderDetails[]> {
  const client = await getAcpClient();
  const { entries } = await client.goose.providersList_unstable({});
  return entries.map((entry) => ({
    name: entry.providerId,
    is_configured: entry.configured,
    provider_type: entry.providerType as ProviderDetails['provider_type'],
    metadata: {
      name: entry.providerId,
      display_name: entry.providerName,
      description: entry.description,
      default_model: entry.defaultModel,
      model_doc_link: '',
      model_selection_hint: entry.modelSelectionHint ?? null,
      config_keys: entry.configKeys.map((key) => ({
        name: key.name,
        required: key.required,
        secret: key.secret,
        default: key.default ?? null,
        oauth_flow: key.oauthFlow ?? false,
        device_code_flow: key.deviceCodeFlow ?? false,
        primary: key.primary ?? false,
      })),
      known_models: entry.models.map((model) => ({
        name: model.id,
        context_limit: model.contextLimit ?? 0,
        reasoning: model.reasoning ?? undefined,
      })),
      setup_steps: entry.setupSteps,
    },
  }));
}

export async function acpListProviderModels(providerId: string) {
  const client = await getAcpClient();
  const { entries } = await client.goose.providersList_unstable({ providerIds: [providerId] });
  return entries.find((e) => e.providerId === providerId)?.models ?? [];
}

export async function acpListProviderCatalogEntries(
  format?: string
): Promise<ProviderTemplateCatalogEntryDto[]> {
  const client = await getAcpClient();
  const { providers } = await client.goose.providersCatalogList_unstable(format ? { format } : {});
  return providers;
}

export async function acpGetProviderTemplate(providerId: string): Promise<ProviderTemplateDto> {
  const client = await getAcpClient();
  const { template } = await client.goose.providersCatalogTemplate_unstable({ providerId });
  return template;
}

export async function acpGetCustomProvider(
  providerId: string
): Promise<CustomProviderReadResponse_unstable> {
  const client = await getAcpClient();
  return client.goose.providersCustomRead_unstable({ providerId });
}

export async function acpCreateCustomProviderFromRequest(
  request: UpdateCustomProviderRequest
): Promise<{ provider_name: string }> {
  const client = await getAcpClient();
  const response = await client.goose.providersCustomCreate_unstable(
    updateRequestToCreate(request)
  );
  return { provider_name: response.providerId };
}

export async function acpUpdateCustomProviderFromRequest(
  providerId: string,
  request: UpdateCustomProviderRequest
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.providersCustomUpdate_unstable({
    providerId,
    ...updateRequestToCreate(request),
  });
}

export async function acpDeleteCustomProvider(providerId: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.providersCustomDelete_unstable({ providerId });
}

export async function acpReadProviderConfig(providerId: string) {
  const client = await getAcpClient();
  const { fields } = await client.goose.providersConfigRead_unstable({ providerId });
  return fields;
}

export async function acpDeleteProviderConfig(providerId: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.providersConfigDelete_unstable({ providerId });
}

export async function acpSaveProviderConfig(
  providerId: string,
  fields: { key: string; value: string }[]
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.providersConfigSave_unstable({ providerId, fields });
}

export async function acpAuthenticateProvider(providerId: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.providersConfigAuthenticate_unstable({ providerId });
}

export async function acpListProviderSecrets(): Promise<ProviderSecretDto[]> {
  const client = await getAcpClient();
  const { secrets } = await client.goose.providersSecretsList_unstable({});
  return secrets;
}

export async function acpDeleteProviderSecret(id: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.providersSecretsDelete_unstable({ id });
}

export async function acpGetCanonicalModelInfo(
  provider: string,
  model: string
): Promise<CanonicalModelInfoDto | null> {
  const client = await getAcpClient();
  const { modelInfo } = await client.goose.providersCanonicalModelInfo_unstable({
    provider,
    model,
  });
  return modelInfo ?? null;
}

export async function acpReadDefaults(): Promise<{
  providerId: string | null;
  modelId: string | null;
}> {
  const client = await getAcpClient();
  const response = await client.goose.defaultsRead_unstable({});
  return {
    providerId: response.providerId ?? null,
    modelId: response.modelId ?? null,
  };
}

export async function acpSaveDefaults(providerId: string, modelId?: string | null): Promise<void> {
  const client = await getAcpClient();
  await client.goose.defaultsSave_unstable({ providerId, modelId: modelId ?? null });
}

export async function acpClearDefaults(): Promise<void> {
  const client = await getAcpClient();
  await client.goose.defaultsClear_unstable({});
}

export async function acpReadThinkingEffort(): Promise<ThinkingEffort | null> {
  const client = await getAcpClient();
  const response = await client.goose.preferencesRead_unstable({ keys: ['gooseThinkingEffort'] });
  const value = response.values.find((v) => v.key === 'gooseThinkingEffort')?.value;
  return typeof value === 'string' ? (value as ThinkingEffort) : null;
}

export async function acpSaveThinkingEffort(effort: ThinkingEffort): Promise<void> {
  const client = await getAcpClient();
  await client.goose.preferencesSave_unstable({
    values: [{ key: 'gooseThinkingEffort', value: effort }],
  });
}

/**
 * Switch the provider (and model) for an active session via ACP config options.
 *
 * Changing the provider on the server resets the session's model, so the model
 * is applied as a follow-up step when supplied.
 */
export async function acpSetSessionProviderModel(
  sessionId: string,
  providerId: string,
  modelId?: string | null
): Promise<void> {
  const client = await getAcpClient();
  await client.setSessionConfigOption({ sessionId, configId: 'provider', value: providerId });
  if (modelId) {
    await client.setSessionConfigOption({ sessionId, configId: 'model', value: modelId });
  }
}
