import React, { useEffect, useMemo, useState, useCallback } from 'react';
import { Input } from '../../../../../ui/input';
import { Select } from '../../../../../ui/Select';
import { useConfig } from '../../../../../ConfigContext';
import { ProviderDetails, ConfigKey } from '../../../../../../api';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../../../../../ui/collapsible';

type ValidationErrors = Record<string, string>;

type ConfigValue = string | { maskedValue: string };
export interface ConfigInput {
  serverValue?: ConfigValue;
  value?: string;
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
  const isAzureProvider = provider.name === 'azure_openai';
  const [azureAuthType, setAzureAuthType] = useState<string>('api_key');
  const [isLoading, setIsLoading] = useState(true);
  const [optionalExpanded, setOptionalExpanded] = useState(false);
  const { read } = useConfig();

  const handleAzureAuthTypeChange = (value: string) => {
    setAzureAuthType(value);
    setConfigValues((prev) => ({
      ...prev,
      AZURE_OPENAI_AUTH_TYPE: {
        ...(prev.AZURE_OPENAI_AUTH_TYPE || {}),
        value,
      },
    }));
  };

  const handleAzureEndpointChange = (value: string) => {
    setConfigValues((prev) => ({
      ...prev,
      AZURE_OPENAI_ENDPOINT: {
        ...(prev.AZURE_OPENAI_ENDPOINT || {}),
        value,
      },
    }));
  };

  const loadConfigValues = useCallback(async () => {
    setIsLoading(true);
    try {
      const values: { [k: string]: ConfigInput } = {};

      for (const parameter of parameters) {
        const configKey = `${parameter.name}`;
        const configValue = (await read(configKey, parameter.secret || false)) as ConfigValue;

        if (isAzureProvider && parameter.name === 'AZURE_OPENAI_AUTH_TYPE') {
          if (typeof configValue === 'string' && configValue) {
            setAzureAuthType(configValue);
          } else if (parameter.default !== undefined && parameter.default !== null) {
            setAzureAuthType(String(parameter.default));
          }
        }
 
        if (configValue) {
          values[parameter.name] = { serverValue: configValue };
        } else if (parameter.default !== undefined && parameter.default !== null) {
          values[parameter.name] = { value: parameter.default };
        }
      }

      setConfigValues((prev) => ({
        ...prev,
        ...values,
      }));
    } finally {
      setIsLoading(false);
    }
  }, [parameters, read, setConfigValues]);

  useEffect(() => {
    loadConfigValues();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Azure-specific UX: after a short human delay, normalize the endpoint field.
  // Cases:
  //  a) value == endpoint (no path, no api-version): do nothing
  //  b) value == endpoint + path: set Endpoint to endpoint only
  //  c) value == endpoint + path + api-version: set Endpoint to endpoint only and
  //     update API Version from the query parameter
  useEffect(() => {
    if (!isAzureProvider) {
      return;
    }

    const entry = configValues['AZURE_OPENAI_ENDPOINT'];
    const rawValue =
      ((entry?.value as string | undefined) ??
        (typeof entry?.serverValue === 'string'
          ? (entry.serverValue as string)
          : undefined)) ||
      '';
    const trimmed = rawValue.trim();

    if (!trimmed) {
      // User cleared the field or hasn't entered anything meaningful yet
      return;
    }

    const timer = setTimeout(() => {
      try {
        const normalized = trimmed.startsWith('http://') || trimmed.startsWith('https://')
          ? trimmed
          : `https://${trimmed}`;

        const url = new URL(normalized);
        const origin = url.origin; // scheme + host + optional port
        const hasPath = url.pathname && url.pathname !== '/';
        const apiVersion = url.searchParams.get('api-version');
        const hasApiVersion = !!apiVersion;

        // Case a) value == endpoint: no path and no api-version → do nothing
        if (!hasPath && !hasApiVersion) {
          return;
        }

        setConfigValues((prev) => {
          const next: Record<string, ConfigInput> = { ...prev };
          let changed = false;

          const prevEndpointEntry = prev.AZURE_OPENAI_ENDPOINT;
          const prevEndpointValue =
            ((prevEndpointEntry?.value as string | undefined) ??
              (typeof prevEndpointEntry?.serverValue === 'string'
                ? (prevEndpointEntry.serverValue as string)
                : undefined)) ||
            '';

          // Case b/c: value had a path → set endpoint field to origin only
          if (hasPath && origin && prevEndpointValue !== origin) {
            next.AZURE_OPENAI_ENDPOINT = {
              ...(prevEndpointEntry || {}),
              value: origin,
            };
            changed = true;
          }

          // Case c: if api-version is present in the URL, update API Version field
          if (hasApiVersion && apiVersion) {
            const prevApiEntry = prev.AZURE_OPENAI_API_VERSION;
            const prevApiValue =
              ((prevApiEntry?.value as string | undefined) ??
                (typeof prevApiEntry?.serverValue === 'string'
                  ? (prevApiEntry.serverValue as string)
                  : undefined)) ||
              '';

            if (prevApiValue !== apiVersion) {
              next.AZURE_OPENAI_API_VERSION = {
                ...(prevApiEntry || {}),
                value: apiVersion,
              };
              changed = true;
            }
          }

          return changed ? next : prev;
        });
      } catch {
        // Ignore parse errors; user may still be typing an incomplete URL or has pasted invalid text.
        // We avoid being intrusive in these cases.
      }
    }, 800); // ~0.8s debounce to allow human typing

    return () => clearTimeout(timer);
  }, [isAzureProvider, configValues, setConfigValues]);

  const getPlaceholder = (parameter: ConfigKey): string => {
    if (parameter.secret) {
      const serverValue = configValues[parameter.name]?.serverValue;
      if (typeof serverValue === 'object' && 'maskedValue' in serverValue) {
        return serverValue.maskedValue;
      }
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

  function getRenderValue(parameter: ConfigKey): string | undefined {
    if (parameter.secret) {
      return undefined;
    }

    const entry = configValues[parameter.name];

    // Important: si l'utilisateur a déjà saisi quelque chose (y compris une chaîne vide),
    // on respecte toujours `value` et on ne retombe jamais sur `serverValue`.
    if (entry && 'value' in entry && entry.value !== undefined) {
      return entry.value ?? '';
    }

    if (typeof entry?.serverValue === 'string') {
      return entry.serverValue as string;
    }

    return '';
  }
 
  const renderParametersList = (parameters: ConfigKey[]) => {
    return parameters.map((parameter) => {
      if (isAzureProvider && parameter.name === 'AZURE_OPENAI_AUTH_TYPE') {
        return null;
      }

      if (
        isAzureProvider &&
        parameter.name === 'AZURE_OPENAI_API_KEY' &&
        azureAuthType === 'entra_id'
      ) {
        return null;
      }

      const isAzureEndpointField =
        isAzureProvider && parameter.name === 'AZURE_OPENAI_ENDPOINT';

      return (
        <div key={parameter.name}>
          <label className="block text-sm font-medium text-textStandard mb-1">
            {getFieldLabel(parameter)}
            {parameter.required && <span className="text-red-500 ml-1">*</span>}
          </label>
          <Input
            type="text"
            value={getRenderValue(parameter)}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
              if (isAzureEndpointField) {
                handleAzureEndpointChange(e.target.value);
              } else {
                setConfigValues((prev) => {
                  const newValue = { ...(prev[parameter.name] || {}), value: e.target.value };
                  return {
                    ...prev,
                    [parameter.name]: newValue,
                  };
                });
              }
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
      );
    });
  };

  let aboveFoldParameters = parameters.filter((p) => p.required);
  let belowFoldParameters = parameters.filter((p) => !p.required);
  if (aboveFoldParameters.length === 0) {
    aboveFoldParameters = belowFoldParameters;
    belowFoldParameters = [];
  }

  const expandCtaText = `${optionalExpanded ? 'Hide' : 'Show'} ${belowFoldParameters.length} options `;
 
  return (
    <div className="mt-4 space-y-4">
      {aboveFoldParameters.length === 0 && belowFoldParameters.length === 0 ? (
        <div className="text-center text-gray-500">
          No configuration parameters for this provider.
        </div>
      ) : (
        <div className="space-y-4">
          {isAzureProvider && (
            <div className="space-y-2">
              <label className="block text-sm font-medium text-textStandard mb-1">
                Authentication Type
              </label>
              <Select
                options={[
                  { value: 'api_key', label: 'Key Authentication' },
                  { value: 'entra_id', label: 'Entra ID Authentication' },
                ]}
                value={{
                  value: azureAuthType,
                  label:
                    azureAuthType === 'entra_id'
                      ? 'Entra ID Authentication'
                      : 'Key Authentication',
                }}
                onChange={(option: unknown) => {
                  const selectedOption = option as { value: string; label: string } | null;
                  if (selectedOption) {
                    handleAzureAuthTypeChange(selectedOption.value);
                  }
                }}
                isSearchable={false}
              />
              <p className="text-xs text-textSubtle">
                {azureAuthType === 'entra_id'
                  ? 'Azure OpenAI will use your Azure Entra ID / default credentials (for example via az login).'
                  : 'Azure OpenAI will use an API key stored securely in Goose configuration.'}
              </p>
            </div>
          )}
          <div>{renderParametersList(aboveFoldParameters)}</div>
          {belowFoldParameters.length > 0 && (
            <Collapsible
              open={optionalExpanded}
              onOpenChange={setOptionalExpanded}
              className="my-4 border-2 border-dashed border-secondary rounded-lg bg-secondary/10"
            >
              <CollapsibleTrigger className="m-3 w-full">
                <div>
                  <span className="text-sm">{expandCtaText}</span>
                  <span className="text-sm">{optionalExpanded ? '↑' : '↓'}</span>
                </div>
              </CollapsibleTrigger>
              <CollapsibleContent className="mx-3 mb-3">
                {renderParametersList(belowFoldParameters)}
              </CollapsibleContent>
            </Collapsible>
          )}
        </div>
      )}
    </div>
  );
}
