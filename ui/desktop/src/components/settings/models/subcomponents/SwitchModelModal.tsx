import { useEffect, useState, useCallback } from 'react';
import { Bot, ExternalLink } from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../../ui/dialog';
import { Button } from '../../../ui/button';
import { QUICKSTART_GUIDE_URL } from '../../providers/modal/constants';
import { Input } from '../../../ui/input';
import { Select } from '../../../ui/Select';
import { useConfig } from '../../../ConfigContext';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import type { View } from '../../../../utils/navigationUtils';
import Model, { getProviderMetadata, fetchModelsForProviders, formatModelHint } from '../modelInterface';
import { getPredefinedModelsFromEnv, shouldShowPredefinedModels } from '../predefinedModelsUtils';
import { ProviderType } from '../../../../api';
import { trackModelChanged } from '../../../../utils/analytics';

const THINKING_LEVEL_OPTIONS = [
  { value: 'low', label: 'Low - Better latency, lighter reasoning' },
  { value: 'high', label: 'High - Deeper reasoning, higher latency' },
];

const VARIANT_OPTIONS = [
  { value: '', label: 'None - No extended thinking' },
  { value: 'low', label: 'Low - Lighter reasoning' },
  { value: 'medium', label: 'Medium - Balanced reasoning' },
  { value: 'high', label: 'High - Deeper reasoning' },
  { value: 'max', label: 'Max - Maximum reasoning depth' },
];

const PREFERRED_MODEL_PATTERNS = [
  /claude-sonnet-4/i,
  /claude-4/i,
  /gpt-4o(?!-mini)/i,
  /claude-3-5-sonnet/i,
  /claude-3\.5-sonnet/i,
  /gpt-4-turbo/i,
  /gpt-4(?!-|o)/i,
  /claude-3-opus/i,
  /claude-3-sonnet/i,
  /gemini-pro/i,
  /llama-3/i,
  /gpt-4o-mini/i,
  /claude-3-haiku/i,
  /gemini/i,
];

function findPreferredModel(
  models: { value: string; label: string; provider: string }[]
): string | null {
  if (models.length === 0) return null;

  const validModels = models.filter(
    (m) => m.value !== 'custom' && m.value !== '__loading__' && !m.value.startsWith('__')
  );

  if (validModels.length === 0) return null;

  for (const pattern of PREFERRED_MODEL_PATTERNS) {
    const match = validModels.find((m) => pattern.test(m.value));
    if (match) {
      return match.value;
    }
  }

  return validModels[0].value;
}

type SwitchModelModalProps = {
  sessionId: string | null;
  onClose: () => void;
  setView: (view: View) => void;
  onModelSelected?: (model: string) => void;
  initialProvider?: string | null;
  titleOverride?: string;
};
export const SwitchModelModal = ({
  sessionId,
  onClose,
  setView,
  onModelSelected,
  initialProvider,
  titleOverride,
}: SwitchModelModalProps) => {
  const { getProviders, read } = useConfig();
  const { changeModel, currentModel, currentProvider } = useModelAndProvider();
  const [providerOptions, setProviderOptions] = useState<{ value: string; label: string }[]>([]);
  type ModelOption = { value: string; label: string; provider: string; hint?: string; supportsReasoning?: boolean; isDisabled?: boolean };
  const [modelOptions, setModelOptions] = useState<{ options: ModelOption[] }[]>([]);
  const [provider, setProvider] = useState<string | null>(
    initialProvider || currentProvider || null
  );
  const [model, setModel] = useState<string>(currentModel || '');
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [validationErrors, setValidationErrors] = useState({
    provider: '',
    model: '',
  });
  const [isValid, setIsValid] = useState(true);
  const [attemptedSubmit, setAttemptedSubmit] = useState(false);
  const [usePredefinedModels] = useState(shouldShowPredefinedModels());
  const [selectedPredefinedModel, setSelectedPredefinedModel] = useState<Model | null>(null);
  const [predefinedModels, setPredefinedModels] = useState<Model[]>([]);
  const [loadingModels, setLoadingModels] = useState<boolean>(false);
  const [userClearedModel, setUserClearedModel] = useState(false);
  const [providerErrors, setProviderErrors] = useState<Record<string, string>>({});
  const [thinkingLevel, setThinkingLevel] = useState<string>('low');
  const [variant, setVariant] = useState<string>('');

  const modelName = usePredefinedModels ? selectedPredefinedModel?.name : model;
  const isGemini3Model = modelName?.toLowerCase().startsWith('gemini-3') ?? false;

  // Check if the selected model supports reasoning
  const isReasoningModel = (() => {
    if (!model) return false;
    const allOptions = modelOptions.flatMap((g) => g.options);
    const selected = allOptions.find((o) => o.value === model);
    return selected?.supportsReasoning ?? false;
  })();

  // Validate form data
  const validateForm = useCallback(() => {
    const errors = {
      provider: '',
      model: '',
    };
    let formIsValid = true;

    if (usePredefinedModels) {
      if (!selectedPredefinedModel) {
        errors.model = 'Please select a model';
        formIsValid = false;
      }
    } else {
      if (!provider) {
        errors.provider = 'Please select a provider';
        formIsValid = false;
      }

      if (!model) {
        errors.model = 'Please select or enter a model';
        formIsValid = false;
      }
    }

    setValidationErrors(errors);
    setIsValid(formIsValid);
    return formIsValid;
  }, [model, provider, usePredefinedModels, selectedPredefinedModel]);

  const handleClose = () => {
    onClose();
  };

  const handleSubmit = async () => {
    setAttemptedSubmit(true);
    const isFormValid = validateForm();

    if (isFormValid) {
      let modelObj: Model;

      if (usePredefinedModels && selectedPredefinedModel) {
        modelObj = selectedPredefinedModel;
      } else {
        const providerMetaData = await getProviderMetadata(provider || '', getProviders);
        const providerDisplayName = providerMetaData.display_name;
        modelObj = {
          name: model,
          provider: provider,
          subtext: providerDisplayName,
        } as Model;
      }

      if (isGemini3Model) {
        modelObj = {
          ...modelObj,
          request_params: { ...modelObj.request_params, thinking_level: thinkingLevel },
        };
      }

      if (isReasoningModel && variant) {
        modelObj = { ...modelObj, variant };
      }

      await changeModel(sessionId, modelObj);
      onModelSelected?.(modelObj.name);

      trackModelChanged(modelObj.provider || '', modelObj.name);

      onClose();
    }
  };

  // Re-validate when inputs change and after attempted submission
  useEffect(() => {
    if (attemptedSubmit) {
      validateForm();
    }
  }, [attemptedSubmit, validateForm]);

  useEffect(() => {
    // Load predefined models if enabled
    if (usePredefinedModels) {
      const models = getPredefinedModelsFromEnv();
      setPredefinedModels(models);

      // Initialize selected predefined model with current model
      (async () => {
        try {
          const currentModelName = (await read('GOOSE_MODEL', false)) as string;
          const matchingModel = models.find((model) => model.name === currentModelName);
          if (matchingModel) {
            setSelectedPredefinedModel(matchingModel);
          }
        } catch (error) {
          console.error('Failed to get current model for selection:', error);
        }
      })();
    }

    // Load providers for manual model selection
    (async () => {
      try {
        const providersResponse = await getProviders(false);
        const activeProviders = providersResponse.filter((provider) => provider.is_configured);
        // Create provider options and add "Use other provider" option
        setProviderOptions([
          ...activeProviders.map(({ metadata, name }) => ({
            value: name,
            label: metadata.display_name,
          })),
          {
            value: 'configure_providers',
            label: 'Use other provider',
          },
        ]);

        setLoadingModels(true);

        const results = await fetchModelsForProviders(activeProviders);

        // Process results and build grouped options
        const groupedOptions: {
          options: { value: string; label: string; provider: string; hint?: string; providerType: ProviderType }[];
        }[] = [];
        const errorMap: Record<string, string> = {};

        results.forEach(({ provider: p, models, modelInfo, error }) => {
          if (error) {
            errorMap[p.name] = error;
            return;
          }

          const modelList = models || [];
          const infoMap = new Map((modelInfo || []).map((info) => [info.name, info]));

          const options: {
            value: string;
            label: string;
            provider: string;
            hint?: string;
            supportsReasoning?: boolean;
            providerType: ProviderType;
          }[] = modelList.map((m) => {
            const info = infoMap.get(m);
            const hint = info ? formatModelHint(info) : undefined;
            return {
              value: m,
              label: m,
              provider: p.name,
              hint: hint || undefined,
              supportsReasoning: info?.supports_reasoning ?? false,
              providerType: p.provider_type,
            };
          });

          if (p.metadata.allows_unlisted_models && p.provider_type !== 'Custom') {
            options.push({
              value: 'custom',
              label: 'Enter a model not listed...',
              provider: p.name,
              providerType: p.provider_type,
            });
          }

          if (options.length > 0) {
            groupedOptions.push({ options });
          }
        });

        // Save provider errors to state
        setProviderErrors(errorMap);

        setModelOptions(groupedOptions);
        setOriginalModelOptions(groupedOptions);
      } catch (error: unknown) {
        console.error('Failed to query providers:', error);
      } finally {
        setLoadingModels(false);
      }
    })();
  }, [getProviders, usePredefinedModels, read]);

  const filteredModelOptions = provider
    ? modelOptions.filter((group) => group.options[0]?.provider === provider)
    : [];

  useEffect(() => {
    // Don't auto-select if user explicitly cleared the model
    if (!provider || loadingModels || model || isCustomModel || userClearedModel) return;

    const providerModels = modelOptions
      .filter((group) => group.options[0]?.provider === provider)
      .flatMap((group) => group.options);

    if (providerModels.length > 0) {
      const preferredModel = findPreferredModel(providerModels);
      if (preferredModel) {
        setModel(preferredModel);
      }
    }
  }, [provider, modelOptions, loadingModels, model, isCustomModel, userClearedModel]);

  // Handle model selection change
  const handleModelChange = (newValue: unknown) => {
    const selectedOption = newValue as { value: string; label: string; provider: string } | null;
    if (selectedOption?.value === 'custom') {
      setIsCustomModel(true);
      setModel('');
      setProvider(selectedOption.provider);
      setUserClearedModel(false);
    } else if (selectedOption === null) {
      // User cleared the selection
      setIsCustomModel(false);
      setModel('');
      setUserClearedModel(true);
    } else {
      setIsCustomModel(false);
      setModel(selectedOption?.value || '');
      setProvider(selectedOption?.provider || '');
      setUserClearedModel(false);
      setVariant('');
    }
  };

  // Store the original model options in state, initialized from modelOptions
  const [originalModelOptions, setOriginalModelOptions] =
    useState<{ options: { value: string; label: string; provider: string }[] }[]>(modelOptions);

  const handleInputChange = (inputValue: string) => {
    if (!provider) return;

    const trimmedInput = inputValue.trim();

    if (trimmedInput === '') {
      // Reset to original model options when input is cleared
      setModelOptions([...originalModelOptions]); // Create new array to ensure state update
      return;
    }

    // Filter through the original model options to find matches
    const matchingOptions = originalModelOptions
      .map((group) => ({
        options: group.options.filter(
          (option) =>
            option.value.toLowerCase().includes(trimmedInput.toLowerCase()) &&
            option.value !== 'custom' // Exclude the "Use custom model" option from search
        ),
      }))
      .filter((group) => group.options.length > 0);

    if (matchingOptions.length > 0) {
      // If we found matches in the existing options, show those
      setModelOptions(matchingOptions);
    } else {
      // If no matches, show the "Use: " option
      const customOption = [
        {
          options: [
            {
              value: trimmedInput,
              label: `Use: "${trimmedInput}"`,
              provider: provider,
            },
          ],
        },
      ];
      setModelOptions(customOption);
    }
  };

  return (
    <Dialog open={true} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Bot size={24} className="text-text-default" />
            {titleOverride || 'Switch models'}
          </DialogTitle>
          <DialogDescription>
            Select a provider and model to use for your conversations.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-4">
          {usePredefinedModels ? (
            <div className="w-full flex flex-col gap-4">
              <div className="flex justify-between items-center">
                <label className="text-sm font-medium text-text-default">Choose a model:</label>
              </div>

              <div className="space-y-2 max-h-64 overflow-y-auto">
                {predefinedModels.map((model) => (
                  <div key={model.id || model.name} className="group hover:cursor-pointer text-sm">
                    <div
                      className={`flex items-center justify-between text-text-default py-2 px-2 ${
                        selectedPredefinedModel?.name === model.name
                          ? 'bg-background-muted'
                          : 'bg-background-default hover:bg-background-muted'
                      } rounded-lg transition-all`}
                      onClick={() => setSelectedPredefinedModel(model)}
                    >
                      <div className="flex-1">
                        <div className="flex items-center justify-between">
                          <span className="text-text-default font-medium">
                            {model.alias || model.name}
                          </span>
                          {model.alias?.includes('recommended') && (
                            <span className="text-xs bg-background-muted text-text-default px-2 py-1 rounded-full border border-border-default ml-2">
                              Recommended
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-2 mt-[2px]">
                          <span className="text-xs text-text-muted">{model.subtext}</span>
                          <span className="text-xs text-text-muted">•</span>
                          <span className="text-xs text-text-muted">{model.provider}</span>
                        </div>
                      </div>

                      <div className="relative flex items-center ml-3">
                        <input
                          type="radio"
                          name="predefined-model"
                          value={model.name}
                          checked={selectedPredefinedModel?.name === model.name}
                          onChange={() => setSelectedPredefinedModel(model)}
                          className="peer sr-only"
                        />
                        <div
                          className="h-4 w-4 rounded-full border border-border-default
                                peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                                peer-checked:bg-white dark:peer-checked:bg-black
                                transition-all duration-200 ease-in-out group-hover:border-border-default"
                        ></div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              {attemptedSubmit && validationErrors.model && (
                <div className="text-red-500 text-sm mt-1">{validationErrors.model}</div>
              )}

              {isGemini3Model && (
                <div className="mt-2">
                  <label className="text-sm text-textSubtle mb-1 block">
                    Thinking Level
                    <span className="text-xs text-textMuted ml-2">(Gemini 3 models only)</span>
                  </label>
                  <Select
                    options={THINKING_LEVEL_OPTIONS}
                    value={THINKING_LEVEL_OPTIONS.find((o) => o.value === thinkingLevel)}
                    onChange={(newValue: unknown) => {
                      const option = newValue as { value: string; label: string } | null;
                      setThinkingLevel(option?.value || 'low');
                    }}
                    placeholder="Select thinking level"
                  />
                </div>
              )}
            </div>
          ) : (
            /* Manual Provider/Model Selection */
            <div className="w-full flex flex-col gap-4">
              <div>
                <Select
                  options={providerOptions}
                  value={providerOptions.find((option) => option.value === provider) || null}
                  onChange={(newValue: unknown) => {
                    const option = newValue as { value: string; label: string } | null;
                    if (option?.value === 'configure_providers') {
                      // Navigate to ConfigureProviders view
                      setView('ConfigureProviders');
                      onClose(); // Close the current modal
                    } else {
                      setProvider(option?.value || null);
                      setModel('');
                      setIsCustomModel(false);
                      setUserClearedModel(false);
                    }
                  }}
                  placeholder="Provider, type to search"
                  isClearable
                />
                {attemptedSubmit && validationErrors.provider && (
                  <div className="text-red-500 text-sm mt-1">{validationErrors.provider}</div>
                )}
              </div>

              {provider && (
                <>
                  {providerErrors[provider] ? (
                    /* Show error message when provider failed to connect */
                    <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-3">
                      <div className="flex items-start">
                        <div className="flex-1">
                          <h3 className="text-sm font-medium text-red-800 dark:text-red-200">
                            Could not contact provider
                          </h3>
                          <div className="mt-1 text-sm text-red-700 dark:text-red-300">
                            {providerErrors[provider]}
                          </div>
                          <div className="mt-2 text-xs text-red-600 dark:text-red-400">
                            Check your provider configuration in Settings → Providers
                          </div>
                        </div>
                      </div>
                    </div>
                  ) : !isCustomModel ? (
                    <div>
                      <Select
                        options={
                          loadingModels
                            ? []
                            : filteredModelOptions.length > 0
                              ? filteredModelOptions
                              : []
                        }
                        onChange={handleModelChange}
                        onInputChange={handleInputChange}
                        value={
                          loadingModels
                            ? { value: '', label: 'Loading models…', isDisabled: true }
                            : model
                              ? { value: model, label: model }
                              : null
                        }
                        placeholder="Select a model, type to search"
                        isClearable
                        isDisabled={loadingModels}
                        formatOptionLabel={(option: unknown) => {
                          const opt = option as ModelOption;
                          if (!opt.hint) return opt.label;
                          return (
                            <div className="flex justify-between items-center w-full">
                              <span>{opt.label}</span>
                              <span className="text-xs text-text-muted ml-2">{opt.hint}</span>
                            </div>
                          );
                        }}
                      />

                      {attemptedSubmit && validationErrors.model && (
                        <div className="text-red-500 text-sm mt-1">{validationErrors.model}</div>
                      )}
                    </div>
                  ) : (
                    <div className="flex flex-col gap-2">
                      <div className="flex justify-between">
                        <label className="text-sm text-text-muted">Custom model name</label>
                        <button
                          onClick={() => setIsCustomModel(false)}
                          className="text-sm text-text-muted"
                        >
                          Back to model list
                        </button>
                      </div>
                      <Input
                        className="border-2 px-4 py-5"
                        placeholder="Type model name here"
                        onChange={(event) => setModel(event.target.value)}
                        value={model}
                      />
                      {attemptedSubmit && validationErrors.model && (
                        <div className="text-red-500 text-sm mt-1">{validationErrors.model}</div>
                      )}
                    </div>
                  )}

                  {isGemini3Model && (
                    <div className="mt-2">
                      <label className="text-sm text-textSubtle mb-1 block">
                        Thinking Level
                        <span className="text-xs text-textMuted ml-2">(Gemini 3 models only)</span>
                      </label>
                      <Select
                        options={THINKING_LEVEL_OPTIONS}
                        value={THINKING_LEVEL_OPTIONS.find((o) => o.value === thinkingLevel)}
                        onChange={(newValue: unknown) => {
                          const option = newValue as { value: string; label: string } | null;
                          setThinkingLevel(option?.value || 'low');
                        }}
                        placeholder="Select thinking level"
                      />
                    </div>
                  )}

                  {isReasoningModel && !isGemini3Model && (
                    <div className="mt-2">
                      <label className="text-sm text-text-muted mb-1 block">
                        Reasoning Effort
                      </label>
                      <Select
                        options={VARIANT_OPTIONS}
                        value={VARIANT_OPTIONS.find((o) => o.value === variant) || VARIANT_OPTIONS[0]}
                        onChange={(newValue: unknown) => {
                          const option = newValue as { value: string; label: string } | null;
                          setVariant(option?.value || '');
                        }}
                        placeholder="Select reasoning effort"
                      />
                    </div>
                  )}
                </>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="pt-4 flex-col sm:flex-row gap-3">
          <a
            href={QUICKSTART_GUIDE_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center text-text-muted hover:text-text-default text-sm mr-auto"
          >
            <ExternalLink size={14} className="mr-1" />
            Quick start guide
          </a>
          <div className="flex gap-2">
            <Button variant="outline" onClick={handleClose} type="button">
              Cancel
            </Button>
            <Button onClick={handleSubmit} disabled={!isValid}>
              Select model
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
