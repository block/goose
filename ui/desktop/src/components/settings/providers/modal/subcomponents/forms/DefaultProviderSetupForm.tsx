import React, { useEffect, useMemo, useState, useCallback } from 'react';
import { Input } from '../../../../../ui/input';
import { useConfig } from '../../../../../ConfigContext';
import { ProviderDetails, ConfigKey } from '../../../../../../api';

type ValidationErrors = Record<string, string>;

export interface ConfigInput {
  value?: string;
  serverHasValue: boolean;
}

interface DefaultProviderSetupFormProps {
  configValues: Record<string, ConfigInput>;
  setConfigValues: React.Dispatch<React.SetStateAction<Record<string, ConfigInput>>>;
  provider: ProviderDetails;
  validationErrors: ValidationErrors;
}

const envToPrettyName = (envVar: string) => {
  const wordReplacements: { [w: string]: string } = {
    Api: 'API',
    Aws: 'AWS',
    Gcp: 'GCP',
  };

  return envVar
    .toLowerCase()
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .map((word) => wordReplacements[word] || word)
    .join(' ')
    .trim();
};

export default function DefaultProviderSetupForm({
  configValues,
  setConfigValues,
  provider,
  validationErrors = {},
}: DefaultProviderSetupFormProps) {
  const parameters = useMemo(
    () => provider.metadata.config_keys || [],
    [provider.metadata.config_keys]
  );
  const [isLoading, setIsLoading] = useState(true);
  const { read } = useConfig();

  const loadConfigValues = useCallback(async () => {
    setIsLoading(true);
    const values: { [k: string]: ConfigInput } = {};

    for (const parameter of parameters) {
      const configKey = `${parameter.name}`;
      const configValue = (await read(configKey, parameter.secret || false)) as string;

      if (configValue) {
        if (parameter.secret) {
          values[parameter.name] = { serverHasValue: true };
        } else {
          values[parameter.name] = { value: configValue, serverHasValue: true };
        }
      } else if (parameter.default !== undefined && parameter.default !== null) {
        values[parameter.name] = { value: configValue, serverHasValue: false };
      }
    }

    setConfigValues((prev) => ({
      ...prev,
      ...values,
    }));
    setIsLoading(false);
  }, [parameters, read, setConfigValues]);

  useEffect(() => {
    loadConfigValues();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const parametersToRender = [...parameters];

  const getPlaceholder = (parameter: ConfigKey): string => {
    if (parameter.secret && configValues[parameter.name]?.serverHasValue) {
      return 'Leave blank to keep existing value';
    }

    if (parameter.default !== undefined && parameter.default !== null) {
      return parameter.default;
    }

    const name = parameter.name.toLowerCase();
    if (name.includes('api_key')) return 'Your API key';
    if (name.includes('api_url') || name.includes('host')) return 'https://api.example.com';
    if (name.includes('models')) return 'model-a, model-b';

    return parameter.name
      .replace(/_/g, ' ')
      .replace(/^./, (str) => str.toUpperCase())
      .trim();
  };

  const getFieldLabel = (parameter: ConfigKey) => {
    const name = parameter.name.toLowerCase();
    if (name.includes('api_key')) return 'API Key';
    if (name.includes('api_url') || name.includes('host')) return 'API Host';
    if (name.includes('models')) return 'Models';

    let parameter_name = parameter.name.toUpperCase();
    if (parameter_name.startsWith(provider.name.toUpperCase().replace('-', '_'))) {
      parameter_name = parameter_name.slice(provider.name.length + 1);
    }
    let pretty = envToPrettyName(parameter_name);
    return (
      <span>
        <span>{pretty}</span>
        <span className="text-sm font-light ml-2">({parameter.name})</span>
      </span>
    );
  };

  if (isLoading) {
    return <div className="text-center py-4">Loading configuration values...</div>;
  }

  return (
    <div className="mt-4 space-y-4">
      {parametersToRender.length === 0 ? (
        <div className="text-center text-gray-500">
          No configuration parameters for this provider.
        </div>
      ) : (
        parametersToRender.map((parameter) => (
          <div key={parameter.name}>
            <label className="block text-sm font-medium text-textStandard mb-1">
              {getFieldLabel(parameter)}
              {parameter.required && <span className="text-red-500 ml-1">*</span>}
            </label>
            <Input
              type={parameter.secret ? 'password' : 'text'}
              value={parameter.secret ? undefined : configValues[parameter.name]?.value || ''}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
                console.log(`Setting ${parameter.name} to:`, e.target.value);
                setConfigValues((prev) => {
                  const newValue = prev[parameter.name] || {
                    value: undefined,
                    serverHasValue: false,
                  };
                  newValue.value = e.target.value;
                  return {
                    ...prev,
                    [parameter.name]: newValue,
                  };
                });
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
