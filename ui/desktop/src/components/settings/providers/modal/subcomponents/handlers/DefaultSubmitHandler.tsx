import { getProviderModels, readConfig } from '../../../../../../api';

/**
 * Standalone function to submit provider configuration
 * Useful for components that don't want to use the hook
 */
export const providerConfigSubmitHandler = async (
  upsertFn: (key: string, value: unknown, isSecret: boolean) => Promise<void>,
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

  // Save current config values for rollback on failure
  const previousConfigValues: Record<string, { value: unknown; isSecret: boolean }> = {};
  for (const param of parameters) {
    try {
      const currentValue = await readConfig({
        body: { key: param.name, is_secret: param.secret || false },
      });
      if (currentValue.data) {
        previousConfigValues[param.name] = {
          value: currentValue.data,
          isSecret: param.secret || false,
        };
      }
    } catch {
      // No previous value exists, that's fine
    }
  }

  const requiredParams = parameters.filter((param) => param.required);
  if (requiredParams.length === 0 && parameters.length > 0) {
    const allOptionalWithDefaults = parameters.every(
      (param) => !param.required && param.default !== undefined
    );
    if (allOptionalWithDefaults) {
      const promises: Promise<void>[] = [];

      for (const param of parameters) {
        if (param.default !== undefined) {
          const value =
            configValues[param.name] !== undefined ? configValues[param.name] : param.default;
          promises.push(upsertFn(param.name, value, param.secret === true));
        }
      }

      await Promise.all(promises);
      return;
    }
  }

  const upsertPromises = parameters.map(
    async (parameter: {
      name: string;
      required?: boolean;
      default?: unknown;
      secret?: boolean;
    }) => {
      if (!configValues[parameter.name] && !parameter.required) {
        return;
      }

      const value =
        configValues[parameter.name] !== undefined
          ? configValues[parameter.name]
          : parameter.default;

      if (value === undefined || value === null) {
        return;
      }

      const configKey = `${parameter.name}`;
      const isSecret = parameter.secret === true;

      await upsertFn(configKey, value, isSecret);
    }
  );

  await Promise.all(upsertPromises);

  // Test the provider configuration by attempting to list models
  // This validates that the provider is properly configured and reachable
  try {
    await getProviderModels({
      path: { name: provider.name },
      throwOnError: true,
    });
  } catch (error) {
    // Rollback to previous config values on failure
    const rollbackPromises: Promise<void>[] = [];
    for (const [key, { value, isSecret }] of Object.entries(previousConfigValues)) {
      rollbackPromises.push(upsertFn(key, value, isSecret));
    }
    await Promise.all(rollbackPromises);

    // Re-throw the error so the UI can show it
    throw error;
  }
};
