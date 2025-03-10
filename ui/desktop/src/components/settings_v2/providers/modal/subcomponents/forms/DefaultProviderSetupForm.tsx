import React, { useEffect, useMemo } from 'react';
import { Input } from '../../../../../ui/input';

interface DefaultProviderSetupFormProps {
  configValues: Record<string, any>;
  setConfigValues: React.Dispatch<React.SetStateAction<Record<string, any>>>;
  provider: any;
}

export default function DefaultProviderSetupForm({
  configValues,
  setConfigValues,
  provider,
}: DefaultProviderSetupFormProps) {
  const parameters = provider.metadata.config_keys || [];

  // Initialize default values when the component mounts or provider changes
  useEffect(() => {
    const defaultValues = {};
    parameters.forEach((parameter) => {
      if (
        parameter.required &&
        parameter.default !== undefined &&
        parameter.default !== null &&
        !configValues[parameter.name]
      ) {
        defaultValues[parameter.name] = parameter.default;
      }
    });

    // Only update if there are default values to add
    if (Object.keys(defaultValues).length > 0) {
      setConfigValues((prev) => ({
        ...prev,
        ...defaultValues,
      }));
    }
  }, [provider.name, parameters, setConfigValues, configValues]);

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

  return (
    <div className="mt-4 space-y-4">
      {requiredParameters.length === 0 ? (
        <div className="text-center text-gray-500">
          No required configuration for this provider.
        </div>
      ) : (
        requiredParameters.map((parameter) => (
          <div key={parameter.name}>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {parameter.name}
              <span className="text-red-500 ml-1">*</span>
            </label>
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
              className="w-full h-14 px-4 font-regular rounded-lg border shadow-none border-gray-300 bg-white text-lg placeholder:text-gray-400 font-regular text-gray-900"
              required={true}
            />
          </div>
        ))
      )}
    </div>
  );
}
