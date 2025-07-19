/**
 * Standalone function to submit provider configuration
 * Useful for components that don't want to use the hook
 */
export const DefaultSubmitHandler = async (
  upsertFn: (key: string, value: unknown, isSecret: boolean) => Promise<void>,
  provider: {
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

  const upsertPromises = parameters.map(
    (parameter: { name: string; required?: boolean; default?: unknown; secret?: boolean }) => {
      // Skip parameters that don't have a value and aren't required
      if (!configValues[parameter.name] && !parameter.required) {
        return Promise.resolve();
      }

      // For required parameters with no value, use the default if available
      const value =
        configValues[parameter.name] !== undefined
          ? configValues[parameter.name]
          : parameter.default;

      // Skip if there's still no value
      if (value === undefined || value === null) {
        return Promise.resolve();
      }

      // Create the provider-specific config key
      const configKey = `${parameter.name}`;

      // Explicitly define is_secret as a boolean (true/false)
      const isSecret = parameter.secret === true;

      // Pass the is_secret flag from the parameter definition
      return upsertFn(configKey, value, isSecret);
    }
  );

  // For providers with no required configuration, save optional parameters with defaults
  // This ensures something gets saved to mark the provider as configured
  if (parameters.length > 0 && parameters.every(p => !p.required)) {
    parameters.forEach((parameter) => {
      if (parameter.default !== undefined && parameter.default !== null) {
        const configKey = `${parameter.name}`;
        const isSecret = parameter.secret === true;
        upsertPromises.push(upsertFn(configKey, parameter.default, isSecret));
      }
    });
  }

  // Wait for all upsert operations to complete
  return Promise.all(upsertPromises);
};
