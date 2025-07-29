import { useState, useEffect, useCallback } from 'react';
import { Recipe, generateDeepLink } from '../recipe';
import { Parameter } from '../recipe/index';
import { FullExtensionConfig } from '../extensions';
import { Geese } from './icons/Geese';
import Copy from './icons/Copy';
import { Check, Save, Calendar, X } from 'lucide-react';
import { useConfig } from './ConfigContext';
import { FixedExtensionEntry } from './ConfigContext';
import RecipeActivityEditor from './RecipeActivityEditor';
import RecipeInfoModal from './RecipeInfoModal';
import RecipeExpandableInfo from './RecipeExpandableInfo';
import { ScheduleFromRecipeModal } from './schedule/ScheduleFromRecipeModal';
import ParameterInput from './parameter/ParameterInput';
import { saveRecipe, generateRecipeFilename } from '../recipe/recipeStorage';
import { toastSuccess, toastError } from '../toasts';
import { Button } from './ui/button';

interface ViewRecipeModalProps {
  isOpen: boolean;
  onClose: () => void;
  config: Recipe;
}

export default function ViewRecipeModal({ isOpen, onClose, config }: ViewRecipeModalProps) {
  const { getExtensions } = useConfig();
  const [recipeConfig] = useState<Recipe | undefined>(config);
  const [title, setTitle] = useState(config?.title || '');
  const [description, setDescription] = useState(config?.description || '');
  const [instructions, setInstructions] = useState(config?.instructions || '');
  const [prompt, setPrompt] = useState(config?.prompt || '');
  const [activities, setActivities] = useState<string[]>(config?.activities || []);
  const [parameters, setParameters] = useState<Parameter[]>(config?.parameters || []);

  const [extensionOptions, setExtensionOptions] = useState<FixedExtensionEntry[]>([]);
  const [extensionsLoaded, setExtensionsLoaded] = useState(false);
  const [copied, setCopied] = useState(false);
  const [isRecipeInfoModalOpen, setRecipeInfoModalOpen] = useState(false);
  const [isScheduleModalOpen, setIsScheduleModalOpen] = useState(false);
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [saveRecipeName, setSaveRecipeName] = useState('');
  const [saveGlobal, setSaveGlobal] = useState(true);
  const [saving, setSaving] = useState(false);
  const [recipeInfoModelProps, setRecipeInfoModelProps] = useState<{
    label: string;
    value: string;
    setValue: (value: string) => void;
  } | null>(null);

  // Initialize selected extensions for the recipe from config
  const [recipeExtensions] = useState<string[]>(() => {
    if (config?.extensions) {
      return config.extensions.map((ext) => ext.name);
    }
    return [];
  });

  // Reset form when config changes
  useEffect(() => {
    if (config) {
      setTitle(config.title || '');
      setDescription(config.description || '');
      setInstructions(config.instructions || '');
      setPrompt(config.prompt || '');
      setActivities(config.activities || []);
      setParameters(config.parameters || []);
    }
  }, [config]);

  // Load extensions when modal opens
  useEffect(() => {
    if (isOpen && !extensionsLoaded) {
      const loadExtensions = async () => {
        try {
          const extensions = await getExtensions(false);
          console.log('Loading extensions for recipe modal');

          if (extensions && extensions.length > 0) {
            const initializedExtensions = extensions.map((ext) => ({
              ...ext,
              enabled: recipeExtensions.includes(ext.name),
            }));

            setExtensionOptions(initializedExtensions);
            setExtensionsLoaded(true);
          }
        } catch (error) {
          console.error('Failed to load extensions:', error);
        }
      };
      loadExtensions();
    }
  }, [isOpen, getExtensions, recipeExtensions, extensionsLoaded]);

  // Auto-detect new parameters from instructions and prompt
  // This adds new parameters without overwriting existing ones
  useEffect(() => {
    const instructionsParams = parseParametersFromInstructions(instructions);
    const promptParams = parseParametersFromInstructions(prompt);

    // Combine all detected parameters, ensuring no duplicates by key
    const detectedParamsMap = new Map<string, Parameter>();

    // Add instruction parameters
    instructionsParams.forEach((param) => {
      detectedParamsMap.set(param.key, param);
    });

    // Add prompt parameters (won't overwrite existing keys)
    promptParams.forEach((param) => {
      if (!detectedParamsMap.has(param.key)) {
        detectedParamsMap.set(param.key, param);
      }
    });

    const existingParamKeys = new Set(parameters.map((param) => param.key));

    // Only add parameters that don't already exist
    const newParams = Array.from(detectedParamsMap.values()).filter(
      (detectedParam) => !existingParamKeys.has(detectedParam.key)
    );

    if (newParams.length > 0) {
      setParameters((prev) => [...prev, ...newParams]);
    }
  }, [instructions, prompt, parameters]);

  const getCurrentConfig = useCallback((): Recipe => {
    // Transform the internal parameters state into the desired output format.
    const formattedParameters = parameters.map((param) => {
      const formattedParam: Parameter = {
        key: param.key,
        input_type: param.input_type || 'string',
        requirement: param.requirement,
        description: param.description,
      };

      // Add the 'default' key ONLY if the parameter is optional and has a default value.
      if (param.requirement === 'optional' && param.default) {
        formattedParam.default = param.default;
      }

      // Add options for select input type
      if (param.input_type === 'select' && param.options) {
        formattedParam.options = param.options.filter((opt) => opt.trim() !== ''); // Filter empty options when saving
      }

      return formattedParam;
    });

    const updatedConfig = {
      ...recipeConfig,
      title,
      description,
      instructions,
      activities,
      prompt,
      parameters: formattedParameters,
      extensions: recipeExtensions
        .map((name) => {
          const extension = extensionOptions.find((e) => e.name === name);
          if (!extension) return null;

          // Create a clean copy of the extension configuration
          const { enabled: _enabled, ...cleanExtension } = extension;
          // Remove legacy envs which could potentially include secrets
          if ('envs' in cleanExtension) {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const { envs: _envs, ...finalExtension } = cleanExtension as any;
            return finalExtension;
          }
          return cleanExtension;
        })
        .filter(Boolean) as FullExtensionConfig[],
    };

    return updatedConfig;
  }, [
    recipeConfig,
    title,
    description,
    instructions,
    activities,
    prompt,
    parameters,
    recipeExtensions,
    extensionOptions,
  ]);

  const [errors, setErrors] = useState<{
    title?: string;
    description?: string;
    instructions?: string;
  }>({});

  const requiredFieldsAreFilled = () => {
    return title.trim() && description.trim() && instructions.trim();
  };

  const validateForm = () => {
    const newErrors: { title?: string; description?: string; instructions?: string } = {};
    if (!title.trim()) {
      newErrors.title = 'Title is required';
    }
    if (!description.trim()) {
      newErrors.description = 'Description is required';
    }
    if (!instructions.trim()) {
      newErrors.instructions = 'Instructions are required';
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleParameterChange = (name: string, value: Partial<Parameter>) => {
    setParameters((prev) =>
      prev.map((param) => (param.key === name ? { ...param, ...value } : param))
    );
  };

  const [deeplink, setDeeplink] = useState('');
  const [isGeneratingDeeplink, setIsGeneratingDeeplink] = useState(false);

  // Generate deeplink whenever recipe configuration changes
  useEffect(() => {
    let isCancelled = false;

    const generateLink = async () => {
      if (!title.trim() || !description.trim() || !instructions.trim()) {
        setDeeplink('');
        return;
      }

      setIsGeneratingDeeplink(true);
      try {
        const currentConfig = getCurrentConfig();
        const link = await generateDeepLink(currentConfig);
        if (!isCancelled) {
          setDeeplink(link);
        }
      } catch (error) {
        console.error('Failed to generate deeplink:', error);
        if (!isCancelled) {
          setDeeplink('Error generating deeplink');
        }
      } finally {
        if (!isCancelled) {
          setIsGeneratingDeeplink(false);
        }
      }
    };

    generateLink();

    return () => {
      isCancelled = true;
    };
  }, [
    title,
    description,
    instructions,
    prompt,
    activities,
    parameters,
    recipeExtensions,
    getCurrentConfig,
  ]);

  const handleCopy = () => {
    if (!deeplink || isGeneratingDeeplink || deeplink === 'Error generating deeplink') {
      return;
    }

    navigator.clipboard
      .writeText(deeplink)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      })
      .catch((err) => {
        console.error('Failed to copy the text:', err);
      });
  };

  const handleSaveRecipe = async () => {
    if (!saveRecipeName.trim()) {
      return;
    }

    setSaving(true);
    try {
      const currentRecipe = getCurrentConfig();

      if (!currentRecipe.title || !currentRecipe.description || !currentRecipe.instructions) {
        throw new Error('Invalid recipe configuration: missing required fields');
      }

      await saveRecipe(currentRecipe, {
        name: saveRecipeName.trim(),
        global: saveGlobal,
      });

      // Reset dialog state
      setShowSaveDialog(false);
      setSaveRecipeName('');

      toastSuccess({
        title: saveRecipeName.trim(),
        msg: `Recipe saved successfully`,
      });
    } catch (error) {
      console.error('Failed to save recipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSaving(false);
    }
  };

  const handleSaveRecipeClick = () => {
    if (!validateForm()) {
      return;
    }

    const currentRecipe = getCurrentConfig();
    // Generate a suggested name from the recipe title
    const suggestedName = generateRecipeFilename(currentRecipe);
    setSaveRecipeName(suggestedName);
    setShowSaveDialog(true);
  };

  const onClickEditTextArea = ({
    label,
    value,
    setValue,
  }: {
    label: string;
    value: string;
    setValue: (value: string) => void;
  }) => {
    setRecipeInfoModalOpen(true);
    setRecipeInfoModelProps({
      label,
      value,
      setValue,
    });
  };

  function parseParametersFromInstructions(instructions: string): Parameter[] {
    const regex = /\{\{(.*?)\}\}/g;
    const matches = [...instructions.matchAll(regex)];

    return matches.map((match) => {
      return {
        key: match[1].trim(),
        description: `Enter value for ${match[1].trim()}`,
        requirement: 'required',
        input_type: 'string',
      };
    });
  }

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[400] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-borderSubtle rounded-lg w-[90vw] max-w-4xl h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-borderSubtle">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-background-default rounded-full flex items-center justify-center">
              <Geese className="w-6 h-6 text-iconProminent" />
            </div>
            <div>
              <h1 className="text-xl font-medium text-textProminent">View/edit current recipe</h1>
              <p className="text-textSubtle text-sm">
                You can edit the recipe below to change the agent's behavior in a new session.
              </p>
            </div>
          </div>
          <Button
            onClick={onClose}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-bgSubtle rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4">
          <div className="space-y-6">
            <div className="pb-6 border-b border-borderSubtle">
              <label htmlFor="title" className="block text-md text-textProminent mb-2 font-bold">
                Title <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={title}
                onChange={(e) => {
                  setTitle(e.target.value);
                  if (errors.title) {
                    setErrors({ ...errors, title: undefined });
                  }
                }}
                className={`w-full p-3 border rounded-lg bg-background-default text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent ${
                  errors.title ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Agent Recipe Title (required)"
              />
              {errors.title && <div className="text-red-500 text-sm mt-1">{errors.title}</div>}
            </div>

            <div className="pb-6 border-b border-borderSubtle">
              <label
                htmlFor="description"
                className="block text-md text-textProminent mb-2 font-bold"
              >
                Description <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={description}
                onChange={(e) => {
                  setDescription(e.target.value);
                  if (errors.description) {
                    setErrors({ ...errors, description: undefined });
                  }
                }}
                className={`w-full p-3 border rounded-lg bg-background-default text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent ${
                  errors.description ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Description (required)"
              />
              {errors.description && (
                <div className="text-red-500 text-sm mt-1">{errors.description}</div>
              )}
            </div>

            <div className="pb-6 border-b border-borderSubtle">
              <RecipeExpandableInfo
                infoLabel="Instructions"
                infoValue={instructions}
                required={true}
                onClickEdit={() =>
                  onClickEditTextArea({
                    label: 'Instructions',
                    value: instructions,
                    setValue: setInstructions,
                  })
                }
              />
              {errors.instructions && (
                <div className="text-red-500 text-sm mt-1">{errors.instructions}</div>
              )}
            </div>

            {parameters.map((parameter: Parameter) => (
              <ParameterInput
                key={parameter.key}
                parameter={parameter}
                onChange={(name, value) => handleParameterChange(name, value)}
              />
            ))}

            <div className="pb-6 border-b border-borderSubtle">
              <RecipeExpandableInfo
                infoLabel="Initial Prompt"
                infoValue={prompt}
                required={false}
                onClickEdit={() =>
                  onClickEditTextArea({
                    label: 'Initial Prompt',
                    value: prompt,
                    setValue: setPrompt,
                  })
                }
              />
            </div>

            <div className="pb-6 border-b border-borderSubtle">
              <RecipeActivityEditor activities={activities} setActivities={setActivities} />
            </div>

            {/* Deep Link Display */}
            <div className="w-full p-4 bg-bgSubtle rounded-lg">
              {!requiredFieldsAreFilled() ? (
                <div className="text-sm text-textSubtle">
                  Fill in required fields to generate link
                </div>
              ) : (
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm text-textSubtle">
                    Copy this link to share with friends or paste directly in Chrome to open
                  </div>
                  <Button
                    onClick={() => validateForm() && handleCopy()}
                    variant="ghost"
                    size="sm"
                    disabled={
                      !deeplink || isGeneratingDeeplink || deeplink === 'Error generating deeplink'
                    }
                    className="ml-4 p-2 hover:bg-background-default rounded-lg transition-colors flex items-center disabled:opacity-50 disabled:hover:bg-transparent"
                  >
                    {copied ? (
                      <Check className="w-4 h-4 text-green-500" />
                    ) : (
                      <Copy className="w-4 h-4 text-iconSubtle" />
                    )}
                    <span className="ml-1 text-sm text-textSubtle">
                      {copied ? 'Copied!' : 'Copy'}
                    </span>
                  </Button>
                </div>
              )}
              {requiredFieldsAreFilled() && (
                <div
                  onClick={() => validateForm() && handleCopy()}
                  className={`text-sm truncate font-mono cursor-pointer ${!title.trim() || !description.trim() ? 'text-textDisabled' : 'text-textStandard'}`}
                >
                  {isGeneratingDeeplink
                    ? 'Generating deeplink...'
                    : deeplink || 'Click to generate deeplink'}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-6 border-t border-borderSubtle">
          <Button
            onClick={onClose}
            variant="ghost"
            className="px-4 py-2 text-textSubtle rounded-lg hover:bg-bgSubtle transition-colors"
          >
            Close
          </Button>

          <div className="flex gap-3">
            <button
              onClick={handleSaveRecipeClick}
              disabled={!requiredFieldsAreFilled() || saving}
              className="inline-flex items-center justify-center gap-2 px-4 py-2 bg-bgStandard text-textStandard border border-borderStandard rounded-lg hover:bg-bgSubtle transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Save className="w-4 h-4" />
              {saving ? 'Saving...' : 'Save Recipe'}
            </button>
            <Button
              onClick={() => setIsScheduleModalOpen(true)}
              disabled={!requiredFieldsAreFilled()}
              variant="outline"
              size="default"
              className="inline-flex items-center justify-center gap-2 px-4 py-2 bg-textProminent text-bgApp rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Calendar className="w-4 h-4" />
              Create Schedule
            </Button>
          </div>
        </div>
      </div>

      <RecipeInfoModal
        infoLabel={recipeInfoModelProps?.label}
        originalValue={recipeInfoModelProps?.value}
        isOpen={isRecipeInfoModalOpen}
        onClose={() => setRecipeInfoModalOpen(false)}
        onSaveValue={recipeInfoModelProps?.setValue}
      />

      <ScheduleFromRecipeModal
        isOpen={isScheduleModalOpen}
        onClose={() => setIsScheduleModalOpen(false)}
        recipe={getCurrentConfig()}
        onCreateSchedule={(deepLink) => {
          // Open the schedules view with the deep link pre-filled
          window.electron.createChatWindow(
            undefined,
            undefined,
            undefined,
            undefined,
            undefined,
            'schedules'
          );
          // Store the deep link in localStorage for the schedules view to pick up
          localStorage.setItem('pendingScheduleDeepLink', deepLink);
        }}
      />

      {/* Save Recipe Dialog */}
      {showSaveDialog && (
        <div className="fixed inset-0 z-[500] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-borderSubtle rounded-lg p-6 w-96 max-w-[90vw]">
            <h3 className="text-lg font-medium text-textProminent mb-4">Save Recipe</h3>

            <div className="space-y-4">
              <div>
                <label
                  htmlFor="recipe-name"
                  className="block text-sm font-medium text-textStandard mb-2"
                >
                  Recipe Name
                </label>
                <input
                  id="recipe-name"
                  type="text"
                  value={saveRecipeName}
                  onChange={(e) => setSaveRecipeName(e.target.value)}
                  className="w-full p-3 border border-borderSubtle rounded-lg bg-background-default text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
                  placeholder="Enter recipe name"
                  autoFocus
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-textStandard mb-2">
                  Save Location
                </label>
                <div className="space-y-2">
                  <label className="flex items-center">
                    <input
                      type="radio"
                      name="save-location"
                      checked={saveGlobal}
                      onChange={() => setSaveGlobal(true)}
                      className="mr-2"
                    />
                    <span className="text-sm text-textStandard">
                      Global - Available across all Goose sessions
                    </span>
                  </label>
                  <label className="flex items-center">
                    <input
                      type="radio"
                      name="save-location"
                      checked={!saveGlobal}
                      onChange={() => setSaveGlobal(false)}
                      className="mr-2"
                    />
                    <span className="text-sm text-textStandard">
                      Directory - Available in the working directory
                    </span>
                  </label>
                </div>
              </div>
            </div>

            <div className="flex justify-end space-x-3 mt-6">
              <button
                onClick={() => {
                  setShowSaveDialog(false);
                  setSaveRecipeName('');
                }}
                className="px-4 py-2 text-textSubtle hover:text-textStandard transition-colors"
                disabled={saving}
              >
                Cancel
              </button>
              <button
                onClick={handleSaveRecipe}
                disabled={!saveRecipeName.trim() || saving}
                className="px-4 py-2 bg-textProminent text-bgApp rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {saving ? 'Saving...' : 'Save Recipe'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
