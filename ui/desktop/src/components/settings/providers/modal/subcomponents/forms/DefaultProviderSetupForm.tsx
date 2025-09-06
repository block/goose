import React, { useEffect, useMemo, useState, useCallback } from 'react';

import { SECRET_PRESENT_SENTINEL } from '../../../../../../utils/secretConstants';

import { Input } from '../../../../../ui/input';
import { useConfig } from '../../../../../ConfigContext'; // Adjust this import path as needed
import { ProviderDetails, ConfigKey } from '../../../../../../api';

type ValidationErrors = Record<string, string>;

interface DefaultProviderSetupFormProps {
  configValues: Record<string, string>;
  setConfigValues: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  provider: ProviderDetails;
  validationErrors: ValidationErrors;
  // Optional callback invoked when a field is edited so parent can clear errors
  onFieldChange?: (name: string, value: string) => void;
}

export default function DefaultProviderSetupForm({
  configValues,
  setConfigValues,
  provider,
  validationErrors = {},
  onFieldChange,
}: DefaultProviderSetupFormProps) {
  const parameters = useMemo(
    () => provider.metadata.config_keys || [],
    [provider.metadata.config_keys]
  );
  const [isLoading, setIsLoading] = useState(true);
  const { read } = useConfig();

  // Initialize values when the component mounts or provider changes
  const loadConfigValues = useCallback(async () => {
    // If there are no parameters, nothing to load
    if (parameters.length === 0) {
      setIsLoading(false);
      return;
    }

    setIsLoading(true);

    // Collect responses per parameter without relying on current configValues
    const responses: Record<string, string> = {};

    for (const parameter of parameters) {
      try {
        const configKey = `${parameter.name}`;
        const configResponse = await read(configKey, parameter.secret || false);

        if (configResponse) {
          responses[parameter.name] = parameter.secret
            ? SECRET_PRESENT_SENTINEL
            : String(configResponse);
        } else if (parameter.default !== undefined && parameter.default !== null) {
          responses[parameter.name] = String(parameter.default);
        }
      } catch (error) {
        console.error(`Failed to load config for ${parameter.name}:`, error);
        if (parameter.default !== undefined && parameter.default !== null) {
          responses[parameter.name] = String(parameter.default);
        }
      }
    }

    // Merge responses into state but do not overwrite user-entered values
    setConfigValues((prev) => {
      const merged = { ...prev };
      for (const k of Object.keys(responses)) {
        if (merged[k] === undefined || merged[k] === null || merged[k] === '') {
          merged[k] = responses[k];
        }
      }
      return merged;
    });

    setIsLoading(false);
  }, [parameters, read, setConfigValues]);

  // Load configuration once on mount (modal open). Parent will remount the component
  // when provider changes so a mount-time load is sufficient and avoids loops.
  useEffect(() => {
    loadConfigValues();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Show all parameters (required and optional)
  const visibleParameters = useMemo(() => parameters, [parameters]);

  // Helper function to generate appropriate placeholder text
  const getPlaceholder = (parameter: ConfigKey): string => {
    // If default is defined and not null, show it
    if (parameter.default !== undefined && parameter.default !== null) {
      return `Default: ${parameter.default}`;
    }

    const name = parameter.name.toLowerCase();
    if (name.includes('api_key')) return 'Your API key';
    if (name.includes('api_url') || name.includes('host')) return 'https://api.example.com';
    if (name.includes('models')) return 'model-a, model-b';

    return parameter.name
      .replace(/_/g, ' ')
      .replace(/([A-Z])/g, ' $1')
      .replace(/^./, (str) => str.toUpperCase())
      .trim();
  };

  // helper for custom labels
  const getFieldLabel = (parameter: ConfigKey): string => {
    const name = parameter.name.toLowerCase();
    if (name.includes('api_key')) return 'API Key';
    if (name.includes('api_url') || name.includes('host')) return 'API Host';
    if (name.includes('models')) return 'Models';

    return parameter.name
      .replace(/_/g, ' ')
      .replace(/([A-Z])/g, ' $1')
      .replace(/^./, (str) => str.toUpperCase())
      .trim();
  };

  const handleChange = (parameter: ConfigKey, value: string) => {
    setConfigValues((prev) => ({
      ...prev,
      [parameter.name]: value,
    }));

    // Let parent clear any validation errors for this field and any submission error
    if (onFieldChange) onFieldChange(parameter.name, value);
  };

  return (
    <div className="mt-4 space-y-4">
      {isLoading && (
        <div className="text-center py-2 text-sm text-textSubtle">
          Loading configuration values...
        </div>
      )}

      {visibleParameters.length === 0 ? (
        <div className="text-center text-gray-500">
          No configuration required for this provider.
        </div>
      ) : (
        visibleParameters.map((parameter) => (
          <div key={parameter.name}>
            <label className="block text-sm font-medium text-textStandard mb-1">
              {getFieldLabel(parameter)}
              {parameter.required && <span className="text-red-500 ml-1">*</span>}
            </label>
            <Input
              type={parameter.secret ? 'password' : 'text'}
              value={configValues[parameter.name] || ''}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                handleChange(parameter, e.target.value);
              }}
              placeholder={getPlaceholder(parameter)}
              className={`w-full h-14 px-4 font-regular rounded-lg shadow-none ${
                validationErrors[parameter.name]
                  ? 'border-2 border-red-500'
                  : 'border border-borderSubtle hover:border-borderStandard'
              } bg-background-default text-lg placeholder:text-textSubtle font-regular text-textStandard`}
              required={parameter.required}
            />
            {validationErrors[parameter.name] && (
              <p className="text-red-500 text-sm mt-1">{validationErrors[parameter.name]}</p>
            )}
          </div>
        ))
      )}
    </div>
  );
}
