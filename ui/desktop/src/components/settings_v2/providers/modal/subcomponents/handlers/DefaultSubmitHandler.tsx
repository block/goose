/**
 * Standalone function to submit provider configuration
 * Useful for components that don't want to use the hook
 */
export const DefaultSubmitHandler = async (upsertFn, provider, configValues) => {
  const parameters = provider.metadata.config_keys || [];

  const upsertPromises = parameters.map((parameter) => {
    // Skip parameters that don't have a value and aren't required
    if (!configValues[parameter.name] && !parameter.required) {
      return Promise.resolve();
    }

    // For required parameters with no value, use the default if available
    const value =
      configValues[parameter.name] !== undefined ? configValues[parameter.name] : parameter.default;

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
  });

  // Wait for all upsert operations to complete
  return Promise.all(upsertPromises);
};
