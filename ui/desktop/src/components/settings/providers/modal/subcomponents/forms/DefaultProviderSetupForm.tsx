import React, { useEffect, useMemo, useState, useCallback } from 'react';
import { Input } from '../../../../../ui/input';
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  // Group parameters by their group field (must be before any early returns)
  const groupedParameters = useMemo(() => {
    const groups = new Map<string | undefined | null, ConfigKey[]>();

    parameters.forEach((p) => {
      const groupKey = p.group;
      if (!groups.has(groupKey)) {
        groups.set(groupKey, []);
      }
      groups.get(groupKey)!.push(p);
    });

    return groups;
  }, [parameters]);

  // Separate required fields and ungrouped optional fields (must be before any early returns)
  // Note: ungrouped params can be under either undefined or null
  const ungroupedParams = groupedParameters.get(undefined) || groupedParameters.get(null) || [];
  const aboveFoldParameters = ungroupedParams.filter((p) => p.required);

  // Determine which fields should show above the fold
  let finalAboveFoldParameters: ConfigKey[];

  if (aboveFoldParameters.length > 0) {
    // If there are required ungrouped fields, show them
    finalAboveFoldParameters = aboveFoldParameters;
  } else {
    // Otherwise, check if there are any required fields anywhere (including in groups)
    const allRequiredParams = parameters.filter((p) => p.required);

    if (allRequiredParams.length > 0) {
      // Show all required fields above the fold
      finalAboveFoldParameters = allRequiredParams;
    } else {
      // No required fields at all - show priority fields (API key, endpoint, etc.)
      const priorityFieldNames = ['API_KEY', 'ENDPOINT', 'HOST', 'URL', 'BASE_URL', 'BASE_PATH'];
      const priorityFields = ungroupedParams.filter((p) => {
        const upperName = p.name.toUpperCase();
        return priorityFieldNames.some(name => upperName.includes(name));
      });

      // If we found priority fields, use them. Otherwise, just show the first few ungrouped params
      if (priorityFields.length > 0) {
        finalAboveFoldParameters = priorityFields;
      } else if (ungroupedParams.length > 0) {
        // Fall back to showing first 3 ungrouped params
        finalAboveFoldParameters = ungroupedParams.slice(0, Math.min(3, ungroupedParams.length));
      } else {
        finalAboveFoldParameters = [];
      }
    }
  }

  // For below-fold parameters, exclude any that are now showing above the fold
  const finalAboveFoldParamNames = new Set(finalAboveFoldParameters.map(p => p.name));
  const belowFoldParameters = ungroupedParams.filter(
    (p) => !p.required && !finalAboveFoldParamNames.has(p.name)
  );

  // Get all grouped parameters (exclude undefined/null group AND exclude any shown above fold) (must be before any early returns)
  const groupedSections = Array.from(groupedParameters.entries())
    .filter(([groupName]) => groupName !== undefined && groupName !== null)
    .map(([groupName, params]) => [
      groupName,
      params.filter(p => !finalAboveFoldParamNames.has(p.name))
    ] as [string | undefined, typeof params])
    .filter(([, params]) => params.length > 0);

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

  // Calculate total count of all below-fold parameters (ungrouped + grouped)
  const totalBelowFoldCount = belowFoldParameters.length +
    groupedSections.reduce((sum, [, params]) => sum + params.length, 0);
  const expandCtaText = `${optionalExpanded ? 'Hide' : 'Show'} ${totalBelowFoldCount} options `;

  if (isLoading) {
    return <div className="text-center py-4">Loading configuration values...</div>;
  }

  function getRenderValue(parameter: ConfigKey): string | undefined {
    if (parameter.secret) {
      return undefined;
    }

    const entry = configValues[parameter.name];
    return entry?.value || (entry?.serverValue as string) || '';
  }

  const renderParametersList = (parameters: ConfigKey[]) => {
    return parameters.map((parameter) => (
      <div key={parameter.name}>
        <label className="block text-sm font-medium text-textStandard mb-1">
          {getFieldLabel(parameter)}
          {parameter.required && <span className="text-red-500 ml-1">*</span>}
        </label>
        <Input
          type="text"
          value={getRenderValue(parameter)}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
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
    ));
  };

  return (
    <div className="mt-4 space-y-4">
      {finalAboveFoldParameters.length === 0 && belowFoldParameters.length === 0 && groupedSections.length === 0 ? (
        <div className="text-center text-gray-500">
          No configuration parameters for this provider.
        </div>
      ) : (
        <div className="space-y-4">
          {/* Only render the above-fold section if there are parameters */}
          {finalAboveFoldParameters.length > 0 && (
            <div className="space-y-4">{renderParametersList(finalAboveFoldParameters)}</div>
          )}

          {/* Render all below-fold parameters in a single collapsible */}
          {totalBelowFoldCount > 0 && (
            <Collapsible
              open={optionalExpanded}
              onOpenChange={setOptionalExpanded}
              className="border-2 border-dashed border-secondary rounded-lg bg-secondary/10"
            >
              <CollapsibleTrigger className="px-3 py-2 w-full text-left">
                <div className="flex items-center">
                  <span className="text-sm">{expandCtaText}</span>
                  <span className="text-sm ml-2">{optionalExpanded ? '↑' : '↓'}</span>
                </div>
              </CollapsibleTrigger>
              <CollapsibleContent className="px-3 pb-3 space-y-4">
                {/* Render ungrouped optional parameters first */}
                {belowFoldParameters.length > 0 && renderParametersList(belowFoldParameters)}

                {/* Render grouped parameters with section headers */}
                {groupedSections.map(([groupName, groupParams]) => (
                  <div key={groupName} className="space-y-4">
                    <div className="pt-2 border-t border-secondary">
                      <h4 className="text-sm font-medium text-textStandard mb-2">{groupName}</h4>
                    </div>
                    {renderParametersList(groupParams)}
                  </div>
                ))}
              </CollapsibleContent>
            </Collapsible>
          )}
        </div>
      )}
    </div>
  );
}
