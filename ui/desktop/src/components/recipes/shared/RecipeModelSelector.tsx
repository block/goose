import { useEffect, useState, useCallback } from 'react';
import { Select } from '../../ui/Select';
import { Input } from '../../ui/input';
import { useConfig } from '../../ConfigContext';
import { fetchModelsForProviders } from '../../settings/models/modelInterface';
import { useLocalization } from '../../../contexts/LocalizationContext';

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
  const { t } = useLocalization();
  const { getProviders } = useConfig();
  const [providerOptions, setProviderOptions] = useState<{ value: string; label: string }[]>([]);
  const [modelOptions, setModelOptions] = useState<
    { options: { value: string; label: string; provider: string }[] }[]
  >([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [fetchError, setFetchError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        setFetchError(null);
        const providersResponse = await getProviders(false);
        const activeProviders = providersResponse.filter((provider) => provider.is_configured);

        setProviderOptions([
          { value: '', label: t('recipes.modelSelector.useDefaultProvider') },
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

          options.push({
            value: `__custom__:${p.name}`,
            label: t('recipes.modelSelector.enterModelNotListed'),
            provider: p.name,
          });

          if (options.length > 0) {
            groupedOptions.push({ options });
          }
        });

        setModelOptions(groupedOptions);
      } catch (error) {
        console.error('Failed to load providers:', error);
        setFetchError(t('recipes.modelSelector.fetchError'));
      } finally {
        setLoadingModels(false);
      }
    })();
  }, [getProviders, t]);

  useEffect(() => {
    if (!loadingModels && selectedModel && selectedProvider) {
      const allModels = modelOptions.flatMap((group) => group.options);
      const modelExists = allModels.some(
        (opt) => opt.value === selectedModel && opt.provider === selectedProvider
      );
      if (!modelExists) {
        setIsCustomModel(true);
      }
    }
  }, [loadingModels, modelOptions, selectedModel, selectedProvider]);

  const filteredModelOptions = selectedProvider
    ? modelOptions.filter((group) => group.options[0]?.provider === selectedProvider)
    : [];

  const handleProviderChange = useCallback(
    (newValue: unknown) => {
      const option = newValue as { value: string; label: string } | null;
      const providerValue = option?.value || undefined;
      onProviderChange(providerValue === '' ? undefined : providerValue);
      onModelChange(undefined);
      setIsCustomModel(false);
    },
    [onProviderChange, onModelChange]
  );

  const handleModelChange = useCallback(
    (newValue: unknown) => {
      const option = newValue as { value: string; label: string; provider: string } | null;
      if (option?.value.startsWith('__custom__:')) {
        setIsCustomModel(true);
        onModelChange(undefined);
      } else {
        setIsCustomModel(false);
        onModelChange(option?.value || undefined);
      }
    },
    [onModelChange]
  );

  return (
    <div className="space-y-4">
      {fetchError && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-700">
          {fetchError}
        </div>
      )}
      <div>
        <label className="block text-sm font-medium text-textStandard mb-2">
          {t('recipes.modelSelector.providerLabel')}
        </label>
        <p className="text-xs text-textSubtle mb-2">
          {t('recipes.modelSelector.providerHint')}
        </p>
        <Select
          options={providerOptions}
          value={
            selectedProvider
              ? providerOptions.find((opt) => opt.value === selectedProvider) || null
              : providerOptions.find((opt) => opt.value === '') || null
          }
          onChange={handleProviderChange}
          placeholder={t('recipes.modelSelector.selectProvider')}
          isClearable
        />
      </div>

      <div>
        <div className="flex justify-between items-center mb-2">
          <label className="block text-sm font-medium text-textStandard">
            {t('recipes.modelSelector.modelLabel')}
          </label>
          {isCustomModel && (
            <button
              onClick={() => {
                setIsCustomModel(false);
                onModelChange(undefined);
              }}
              className="text-xs text-textSubtle hover:underline"
              type="button"
            >
              {t('recipes.modelSelector.backToModelList')}
            </button>
          )}
        </div>
        <p className="text-xs text-textSubtle mb-2">
          {t('recipes.modelSelector.modelHint')}
        </p>
        {isCustomModel ? (
          <Input
            type="text"
            placeholder={t('recipes.modelSelector.customModelPlaceholder')}
            value={selectedModel || ''}
            onChange={(e) => onModelChange(e.target.value || undefined)}
          />
        ) : (
          <Select
            options={loadingModels ? [] : filteredModelOptions}
            value={
              loadingModels
                ? { value: '', label: t('recipes.modelSelector.loadingModels'), isDisabled: true }
                : selectedModel
                  ? { value: selectedModel, label: selectedModel }
                  : null
            }
            onChange={handleModelChange}
            placeholder={t('recipes.modelSelector.selectModel')}
            isClearable
            isDisabled={loadingModels}
          />
        )}
      </div>
    </div>
  );
};
