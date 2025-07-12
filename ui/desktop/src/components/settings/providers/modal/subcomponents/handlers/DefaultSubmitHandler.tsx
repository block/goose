/**
 * Standalone function to submit provider configuration
 * Useful for components that don't want to use the hook
 */
export const DefaultSubmitHandler = async (
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
  configValues: Record<string, unknown>
) => {
  const parameters = provider.metadata.config_keys || [];

  if (parameters.length === 0) {
    const configKey = `${provider.name}_configured`;
    return upsertFn(configKey, true, false);
  }

  const requiredParams = parameters.filter((param) => param.required);
  if (requiredParams.length === 0 && parameters.length > 0) {
    const allOptionalWithDefaults = parameters.every(
      (param) => !param.required && param.default !== undefined
    );
    if (allOptionalWithDefaults) {
      const promises: Promise<void>[] = [];
      const configKey = `${provider.name}_configured`;
      promises.push(upsertFn(configKey, true, false));

      for (const param of parameters) {
        if (param.default !== undefined) {
          const value =
            configValues[param.name] !== undefined ? configValues[param.name] : param.default;
          promises.push(upsertFn(param.name, value, param.secret === true));
        }
      }

      return Promise.all(promises);
    }
  }

  const upsertPromises = parameters.map(
    (parameter: { name: string; required?: boolean; default?: unknown; secret?: boolean }) => {
      if (!configValues[parameter.name] && !parameter.required) {
        return Promise.resolve();
      }

      const value =
        configValues[parameter.name] !== undefined
          ? configValues[parameter.name]
          : parameter.default;

      if (value === undefined || value === null) {
        return Promise.resolve();
      }

      const configKey = `${parameter.name}`;

      const isSecret = parameter.secret === true;

      return upsertFn(configKey, value, isSecret);
    }
  );

  return Promise.all(upsertPromises);
};
