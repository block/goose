import {
  type ChangeEvent,
  type Dispatch,
  type SetStateAction,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import type { ConfigKey, ProviderDetails } from '../../../../../../api';
import { useConfig } from '../../../../../ConfigContext';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../../../../../ui/collapsible';
import { Input } from '../../../../../ui/input';

type ValidationErrors = Record<string, string>;

type ConfigValue = string | { maskedValue: string };

export interface ConfigInput {
  serverValue?: ConfigValue;
  value?: string;
}

interface DefaultProviderSetupFormProps {
  configValues: Record<string, ConfigInput>;
  setConfigValues: Dispatch<SetStateAction<Record<string, ConfigInput>>>;
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
  const [optionalExpanded, setOptionalExpanded] = useState(false);
  const { read } = useConfig();

  const loadConfigValues = useCallback(async () => {
    setIsLoading(true);
    try {
      const values: { [k: string]: ConfigInput } = {};

      for (const parameter of parameters) {
        const configKey = `${parameter.name}`;
        const configValue = (await read(configKey, parameter.secret || false)) as ConfigValue;

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
  }, [loadConfigValues]);

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

    let parameterName = parameter.name.toUpperCase();
    if (parameterName.startsWith(provider.name.toUpperCase().replace('-', '_'))) {
      parameterName = parameterName.slice(provider.name.length + 1);
    }
    const pretty = envToPrettyName(parameterName);
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

    // Important: if the user has already entered something (including an empty string),
    // always prefer `value` and never fall back to `serverValue`.
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
      const inputId = `config-${parameter.name}`;

      return (
        <div key={parameter.name}>
          <label className="block text-sm font-medium text-textStandard mb-1" htmlFor={inputId}>
            {getFieldLabel(parameter)}
            {parameter.required && <span className="text-red-500 ml-1">*</span>}
          </label>
          <Input
            id={inputId}
            type="text"
            value={getRenderValue(parameter)}
            onChange={(e: ChangeEvent<HTMLInputElement>) => {
              setConfigValues((prev) => {
                const newValue = { ...(prev[parameter.name] || {}), value: e.target.value };
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
