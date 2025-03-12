import { useConfig } from '../../../../../ConfigContext';
import React from 'react';

/**
 * Custom hook for provider configuration submission
 * Returns a submit handler function and submission state
 */
export const useDefaultSubmit = () => {
  const { upsert } = useConfig();
  const [isSubmitting, setIsSubmitting] = React.useState(false);
  const [error, setError] = React.useState(null);
  const [isSuccess, setIsSuccess] = React.useState(false);

  /**
   * Submit handler for provider configuration
   * @param {Object} provider - The provider object with metadata
   * @param {Object} configValues - The form values to be submitted
   * @param {Function} onSuccess - Optional callback for successful submission
   */
  const handleSubmit = async (provider, configValues, onSuccess) => {
    setIsSubmitting(true);
    setError(null);
    setIsSuccess(false);

    try {
      const parameters = provider.metadata.config_keys || [];

      // Create an array of promises for all the upsert operations
      const upsertPromises = parameters.map((parameter) => {
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
        // Format: provider.{provider_name}.{parameter_name}
        const configKey = `provider.${provider.name}.${parameter.name}`;

        // Pass the is_secret flag from the parameter definition
        return upsert(configKey, value, parameter.secret || false);
      });

      // Wait for all upsert operations to complete
      await Promise.all(upsertPromises);

      setIsSuccess(true);

      // Call the success callback if provided
      if (onSuccess) {
        onSuccess();
      }
    } catch (err) {
      console.error('Failed to save provider configuration:', err);
      setError('Failed to save configuration. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return {
    handleSubmit,
    isSubmitting,
    error,
    isSuccess,
  };
};

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
