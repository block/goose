import { getProviderModels, readConfig } from '../../../../../../api';
import type { GooseConfigUpdate, GooseConfigResponse } from '../../../../../../api';

/**
 * Standalone function to submit provider configuration
 * Useful for components that don't want to use the hook
 */
export const providerConfigSubmitHandler = async (
  updateFn: (patch: GooseConfigUpdate) => Promise<void>,
  provider: {
    name: string;
    metadata: {
      config_keys?: Array<{
        name: string;
        required?: boolean;
        default?: unknown;
        secret?: boolean;
      }>;
    };
  },
  configValues: Record<string, string>
) => {
  const parameters = provider.metadata.config_keys || [];

  // Save current NON-SECRET config values for rollback on failure
  // We skip secrets because readConfig returns null for secrets,
  // and upserting those null values would corrupt the actual secret
  let previousConfig: GooseConfigResponse = {};
  const nonSecretParams = parameters.filter((param) => !param.secret);

  if (nonSecretParams.length > 0) {
    try {
      const currentConfig = await readConfig();
      previousConfig = currentConfig.data ?? {};
    } catch {
      // No previous config, that's fine
    }
  }

  const requiredParams = parameters.filter((param) => param.required);
  if (requiredParams.length === 0 && parameters.length > 0) {
    const allOptionalWithDefaults = parameters.every(
      (param) => !param.required && param.default !== undefined
    );
    if (allOptionalWithDefaults) {
      const patch: Record<string, unknown> = {};
      for (const param of parameters) {
        if (param.default !== undefined) {
          patch[param.name] =
            configValues[param.name] !== undefined ? configValues[param.name] : param.default;
        }
      }
      await updateFn(patch as GooseConfigUpdate);
      return;
    }
  }

  const patch: Record<string, unknown> = {};
  for (const parameter of parameters) {
    if (!configValues[parameter.name] && !parameter.required) {
      continue;
    }

    const value =
      configValues[parameter.name] !== undefined
        ? configValues[parameter.name]
        : parameter.default;

    if (value === undefined || value === null) {
      continue;
    }

    patch[parameter.name] = value;
  }

  await updateFn(patch as GooseConfigUpdate);

  try {
    await getProviderModels({
      path: { name: provider.name },
      throwOnError: true,
    });
  } catch (error) {
    // Rollback non-secret params to their previous values
    const rollbackPatch: Record<string, unknown> = {};
    for (const param of nonSecretParams) {
      const key = param.name as keyof GooseConfigResponse;
      if (key in previousConfig) {
        rollbackPatch[param.name] = previousConfig[key];
      }
    }
    if (Object.keys(rollbackPatch).length > 0) {
      await updateFn(rollbackPatch as GooseConfigUpdate);
    }

    throw error;
  }
};
