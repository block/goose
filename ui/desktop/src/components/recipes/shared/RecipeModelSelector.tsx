import { useEffect, useState, useCallback } from 'react';
import { Select } from '../../ui/Select';
import { useConfig } from '../../ConfigContext';
import { fetchModelsForProviders } from '../../settings/models/modelInterface';

interface RecipeModelSelectorProps {
  selectedProvider?: string;
  selectedModel?: string;
  onProviderChange: (provider: string | undefined) => void;
  onModelChange: (model: string | undefined) => void;
}

export const RecipeModelSelector = ({
  selectedProvider,
  selectedModel,
  onProviderChange,
  onModelChange,
}: RecipeModelSelectorProps) => {
  const { getProviders } = useConfig();
  const [providerOptions, setProviderOptions] = useState<{ value: string; label: string }[]>([]);
  const [modelOptions, setModelOptions] = useState<
    { options: { value: string; label: string; provider: string }[] }[]
  >([]);
  const [loadingModels, setLoadingModels] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const providersResponse = await getProviders(false);
        const activeProviders = providersResponse.filter((provider) => provider.is_configured);

        setProviderOptions([
          { value: '', label: 'Use default provider' },
          ...activeProviders.map(({ metadata, name }) => ({
            value: name,
            label: metadata.display_name,
          })),
        ]);

        setLoadingModels(true);
        const results = await fetchModelsForProviders(activeProviders);

        const groupedOptions: {
          options: { value: string; label: string; provider: string }[];
        }[] = [];

        results.forEach(({ provider: p, models, error }) => {
          if (error) {
            return;
          }

          const modelList = models || [];
          const options = modelList.map((m) => ({
            value: m,
            label: m,
            provider: p.name,
          }));

          if (p.metadata.allows_unlisted_models) {
            options.push({
              value: 'custom',
              label: 'Enter a model not listed...',
              provider: p.name,
            });
          }

          if (options.length > 0) {
            groupedOptions.push({ options });
          }
        });

        setModelOptions(groupedOptions);
      } catch (error) {
        console.error('Failed to load providers:', error);
      } finally {
        setLoadingModels(false);
      }
    })();
  }, [getProviders]);

  const filteredModelOptions = selectedProvider
    ? modelOptions.filter((group) => group.options[0]?.provider === selectedProvider)
    : [];

  const handleProviderChange = useCallback(
    (newValue: unknown) => {
      const option = newValue as { value: string; label: string } | null;
      const providerValue = option?.value || undefined;
      onProviderChange(providerValue === '' ? undefined : providerValue);
      onModelChange(undefined);
    },
    [onProviderChange, onModelChange]
  );

  const handleModelChange = useCallback(
    (newValue: unknown) => {
      const option = newValue as { value: string; label: string } | null;
      onModelChange(option?.value || undefined);
    },
    [onModelChange]
  );

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-textStandard mb-2">
          Provider (Optional)
        </label>
        <p className="text-xs text-textSubtle mb-2">
          Leave empty to use the default provider configured in settings
        </p>
        <Select
          options={providerOptions}
          value={
            selectedProvider
              ? providerOptions.find((opt) => opt.value === selectedProvider) || null
              : providerOptions.find((opt) => opt.value === '') || null
          }
          onChange={handleProviderChange}
          placeholder="Select provider"
          isClearable
        />
      </div>

      {selectedProvider && (
        <div>
          <label className="block text-sm font-medium text-textStandard mb-2">
            Model (Optional)
          </label>
          <p className="text-xs text-textSubtle mb-2">
            Leave empty to use the default model for the selected provider
          </p>
          <Select
            options={loadingModels ? [] : filteredModelOptions}
            value={
              loadingModels
                ? { value: '', label: 'Loading models…', isDisabled: true }
                : selectedModel
                  ? { value: selectedModel, label: selectedModel }
                  : null
            }
            onChange={handleModelChange}
            placeholder="Select a model"
            isClearable
            isDisabled={loadingModels}
          />
        </div>
      )}
    </div>
  );
};
