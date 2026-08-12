import { acpSaveProviderConfig } from '../../../../../../acp/providers';

/**
 * Submit provider configuration through ACP.
 *
 * The ACP server validates the supplied fields, persists config/secret values,
 * and triggers an inventory refresh in a single call, so no client-side
 * rollback is required.
 */
export const providerConfigSubmitHandler = async (
  provider: {
    name: string;
    metadata: {
      config_keys?: Array<{ name: string; default?: unknown }>;
    };
  },
  configValues: Record<string, string>
) => {
  const fields: { key: string; value: string }[] = [];
  for (const { name, default: defaultValue } of provider.metadata.config_keys ?? []) {
    const value = configValues[name] ?? defaultValue;
    if (value === undefined || value === null || value === '') {
      continue;
    }
    fields.push({ key: name, value: String(value) });
  }

  await acpSaveProviderConfig(provider.name, fields);
};
