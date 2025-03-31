import React, { useEffect, useMemo, useState } from 'react';
import { Input } from '../../../../../ui/input';
import { useConfig } from '../../../../../ConfigContext'; // Adjust this import path as needed

interface DefaultProviderSetupFormProps {
  configValues: Record<string, any>;
  setConfigValues: React.Dispatch<React.SetStateAction<Record<string, any>>>;
  provider: any;
  validationErrors: any;
}

export default function DefaultProviderSetupForm({
  configValues,
  setConfigValues,
  provider,
  validationErrors,
}: DefaultProviderSetupFormProps) {
  const parameters = provider.metadata.config_keys || [];
  const [isLoading, setIsLoading] = useState(true);
  const { read } = useConfig();

  // Initialize values when the component mounts or provider changes
  useEffect(() => {
    const loadConfigValues = async () => {
      setIsLoading(true);
      const newValues = { ...configValues };

      // Try to load actual values from config for each parameter that is not secret
      for (const parameter of parameters) {
        if (parameter.required) {
          try {
            // Check if there's a stored value in the config system
            const configKey = `${parameter.name}`;
            const configResponse = await read(configKey, parameter.secret || false);

            if (configResponse) {
              // Use the value from the config provider
              newValues[parameter.name] = configResponse;
            } else if (
              parameter.default !== undefined &&
              parameter.default !== null &&
              !configValues[parameter.name]
            ) {
              // Fall back to default value if no config value exists
              newValues[parameter.name] = parameter.default;
            }
          } catch (error) {
            console.error(`Failed to load config for ${parameter.name}:`, error);
            // Fall back to default if read operation fails
            if (
              parameter.default !== undefined &&
              parameter.default !== null &&
              !configValues[parameter.name]
            ) {
              newValues[parameter.name] = parameter.default;
            }
          }
        }
      }

      // Update state with loaded values
      setConfigValues((prev) => ({
        ...prev,
        ...newValues,
      }));
      setIsLoading(false);
    };

    loadConfigValues();
  }, [provider.name, parameters, setConfigValues, read]);

  // Filter parameters to only show required ones
  const requiredParameters = useMemo(() => {
    return parameters.filter((param) => param.required === true);
  }, [parameters]);

  // Helper function to generate appropriate placeholder text
  const getPlaceholder = (parameter) => {
    // If default is defined and not null, show it
    if (parameter.default !== undefined && parameter.default !== null) {
      return `Default: ${parameter.default}`;
    }

    // Otherwise, use the parameter name as a hint
    return parameter.name.toUpperCase();
  };

  if (isLoading) {
    return <div className="text-center py-4">Loading configuration values...</div>;
  }

  return (
    <div className="mt-4 space-y-4">
      {requiredParameters.length === 0 ? (
        <div className="text-center text-gray-500">
          No required configuration for this provider.
        </div>
      ) : (
        requiredParameters.map((parameter) => (
          <div key={parameter.name}>
            <label className="block text-sm font-medium text-gray-700 mb-1">{parameter.name}</label>
            <Input
              type={parameter.secret ? 'password' : 'text'}
              value={configValues[parameter.name] || ''}
              onChange={(e) =>
                setConfigValues((prev) => ({
                  ...prev,
                  [parameter.name]: e.target.value,
                }))
              }
              placeholder={getPlaceholder(parameter)}
              className={`w-full h-14 px-4 font-regular rounded-lg border shadow-none ${
                validationErrors[parameter.name] ? 'border-red-500' : 'border-gray-300'
              } bg-white text-lg placeholder:text-gray-400 font-regular text-gray-900`}
              required={true}
            />
          </div>
        ))
      )}
    </div>
  );
}
