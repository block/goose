import { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { Parameter } from '../recipe/index';

import { Buffer } from 'buffer';
import { FullExtensionConfig } from '../extensions';
import { Geese } from './icons/Geese';
import Copy from './icons/Copy';
import { Check, Save, Calendar } from 'lucide-react';
import { useConfig } from './ConfigContext';
import { FixedExtensionEntry } from './ConfigContext';
import RecipeActivityEditor from './RecipeActivityEditor';
import RecipeInfoModal from './RecipeInfoModal';
import RecipeExpandableInfo from './RecipeExpandableInfo';
import { ScheduleFromRecipeModal } from './schedule/ScheduleFromRecipeModal';
import ParameterInput from './parameter/ParameterInput';
import { saveRecipe, generateRecipeFilename } from '../recipe/recipeStorage';
import { toastSuccess, toastError } from '../toasts';

interface RecipeEditorProps {
  config?: Recipe;
}

// Function to generate a deep link from a recipe
function generateDeepLink(recipe: Recipe): string {
  const configBase64 = Buffer.from(JSON.stringify(recipe)).toString('base64');
  const urlSafe = encodeURIComponent(configBase64);
  return `goose://recipe?config=${urlSafe}`;
}

export default function RecipeEditor({ config }: RecipeEditorProps) {
  const { getExtensions } = useConfig();
  const [recipeConfig] = useState<Recipe | undefined>(config);
  const [title, setTitle] = useState(config?.title || '');
  const [description, setDescription] = useState(config?.description || '');
  const [instructions, setInstructions] = useState(config?.instructions || '');
  const [prompt, setPrompt] = useState(config?.prompt || '');
  const [activities, setActivities] = useState<string[]>(config?.activities || []);
  const [parameters, setParameters] = useState<Parameter[]>(
    parseParametersFromInstructions(instructions)
  );

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

  // Initialize selected extensions for the recipe from config or localStorage
  const [recipeExtensions] = useState<string[]>(() => {
    // First try to get from localStorage
    const stored = localStorage.getItem('recipe_editor_extensions');
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        return Array.isArray(parsed) ? parsed : [];
      } catch (e) {
        console.error('Failed to parse localStorage recipe extensions:', e);
        return [];
      }
    }
    // Fall back to config if available, using extension names
    const exts: string[] = [];
    return exts;
  });
  // Section visibility state
  const [activeSection, _] = useState<'none' | 'activities' | 'instructions' | 'extensions'>(
    'none'
  );

  // Load extensions when component mounts and when switching to extensions section
  useEffect(() => {
    if (activeSection === 'extensions' && !extensionsLoaded) {
      const loadExtensions = async () => {
        try {
          const extensions = await getExtensions(false); // force refresh to get latest
          console.log('Loading extensions for recipe editor');

          if (extensions && extensions.length > 0) {
            // Map the extensions with the current selection state from recipeExtensions
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
  }, [activeSection, getExtensions, recipeExtensions, extensionsLoaded]);

  // Effect for updating extension options when recipeExtensions change
  useEffect(() => {
    if (extensionsLoaded && extensionOptions.length > 0) {
      const updatedOptions = extensionOptions.map((ext) => ({
        ...ext,
        enabled: recipeExtensions.includes(ext.name),
      }));
      setExtensionOptions(updatedOptions);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recipeExtensions, extensionsLoaded]);

  // Use effect to set parameters whenever instructions or prompt changes
  useEffect(() => {
    const instructionsParams = parseParametersFromInstructions(instructions);
    const promptParams = parseParametersFromInstructions(prompt);

    // Combine parameters, ensuring no duplicates by key
    const allParams = [...instructionsParams];
    promptParams.forEach((promptParam) => {
      if (!allParams.some((param) => param.key === promptParam.key)) {
        allParams.push(promptParam);
      }
    });

    setParameters(allParams);
  }, [instructions, prompt]);

  const getCurrentConfig = (): Recipe => {
    // Transform the internal parameters state into the desired output format.
    const formattedParameters = parameters.map((param) => {
      const formattedParam: Parameter = {
        key: param.key,
        input_type: 'string',
        requirement: param.requirement,
        description: param.description,
      };

      // Add the 'default' key ONLY if the parameter is optional and has a default value.
      if (param.requirement === 'optional' && param.default) {
        // Note: `default` is a reserved keyword in JS, but assigning it as a property key like this is valid.
        formattedParam.default = param.default;
      }

      return formattedParam;
    });

    const config = {
      ...recipeConfig,
      title,
      description,
      instructions,
      activities,
      prompt,
      // Use the newly formatted parameters array in the final config object.
      parameters: formattedParameters,
      extensions: recipeExtensions
        .map((name) => {
          const extension = extensionOptions.find((e) => e.name === name);
          console.log('Looking for extension:', name, 'Found:', extension);
          if (!extension) return null;

          // Create a clean copy of the extension configuration
          const { enabled: _enabled, ...cleanExtension } = extension;
          // Remove legacy envs which could potentially include secrets
          // env_keys will work but rely on the end user having setup those keys themselves
          if ('envs' in cleanExtension) {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const { envs: _envs, ...finalExtension } = cleanExtension as any;
            return finalExtension;
          }
          return cleanExtension;
        })
        .filter(Boolean) as FullExtensionConfig[],
    };
    console.log('Final config extensions:', config.extensions);

    return config;
  };

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

  const deeplink = generateDeepLink(getCurrentConfig());

  const handleCopy = () => {
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
  // Reset extensionsLoaded when section changes away from extensions
  useEffect(() => {
    if (activeSection !== 'extensions') {
      setExtensionsLoaded(false);
    }
  }, [activeSection]);
  const page_title = config?.title ? 'View/edit current recipe' : 'Create an agent recipe';
  const subtitle = config?.title
    ? "You can edit the recipe below to change the agent's behavior in a new session."
    : 'Your custom agent recipe can be shared with others. Fill in the sections below to create!';

  function parseParametersFromInstructions(instructions: string): Parameter[] {
    const regex = /\{\{(.*?)\}\}/g;
    const matches = [...instructions.matchAll(regex)];

    return matches.map((match) => {
      return {
        key: match[1].trim(),
        description: `Enter value for ${match[1].trim()}`,
        requirement: 'required',
        input_type: 'string', // Default to string; can be changed based on requirements
      };
    });
  }

  return (
    <div className="flex flex-col w-full h-screen bg-bgApp max-w-3xl mx-auto">
      {activeSection === 'none' && (
        <div className="flex flex-col items-center mb-2 px-6 pt-10">
          <div className="w-16 h-16 bg-bgApp rounded-full flex items-center justify-center mb-4">
            <Geese className="w-12 h-12 text-iconProminent" />
          </div>
          <h1 className="text-2xl font-medium text-center text-textProminent">{page_title}</h1>
          <p className="text-textSubtle text-center mt-2 text-sm">{subtitle}</p>
        </div>
      )}
      <div className="flex-1 overflow-y-auto px-6">
        <div className="flex flex-col">
          <h2 className="text-lg font-medium mb-2 text-textProminent">Agent Recipe Details</h2>
        </div>
        <div className="space-y-2 py-2">
          <div className="pb-6 border-b-2 border-borderSubtle">
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
              className={`w-full p-3 border rounded-lg bg-bgApp text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent ${
                errors.title ? 'border-red-500' : 'border-borderSubtle'
              }`}
              placeholder="Agent Recipe Title (required)"
            />
            {errors.title && <div className="text-red-500 text-sm mt-1">{errors.title}</div>}
          </div>
          <div className="pt-3 pb-6 border-b-2 border-borderSubtle">
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
              className={`w-full p-3 border rounded-lg bg-bgApp text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent ${
                errors.description ? 'border-red-500' : 'border-borderSubtle'
              }`}
              placeholder="Description (required)"
            />
            {errors.description && (
              <div className="text-red-500 text-sm mt-1">{errors.description}</div>
            )}
          </div>
          <div className="pt-3 pb-6 border-b-2 border-borderSubtle">
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
          <div className="pt-3 pb-6 border-b-2 border-borderSubtle">
            <RecipeExpandableInfo
              infoLabel="Initial Prompt"
              infoValue={prompt}
              required={false}
              onClickEdit={() =>
                onClickEditTextArea({ label: 'Initial Prompt', value: prompt, setValue: setPrompt })
              }
            />
          </div>
          <div className="pt-3 pb-6">
            <RecipeActivityEditor activities={activities} setActivities={setActivities} />
          </div>

          {/* Deep Link Display */}
          <div className="w-full p-4 bg-bgSubtle rounded-lg">
            {!requiredFieldsAreFilled() ? (
              <div className="text-sm text-textSubtle text-xs text-textSubtle">
                Fill in required fields to generate link
              </div>
            ) : (
              <div className="flex items-center justify-between mb-2">
                <div className="text-sm text-textSubtle text-xs text-textSubtle">
                  Copy this link to share with friends or paste directly in Chrome to open
                </div>
                <button
                  onClick={() => validateForm() && handleCopy()}
                  className="ml-4 p-2 hover:bg-bgApp rounded-lg transition-colors flex items-center disabled:opacity-50 disabled:hover:bg-transparent"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-green-500" />
                  ) : (
                    <Copy className="w-4 h-4 text-iconSubtle" />
                  )}
                  <span className="ml-1 text-sm text-textSubtle">
                    {copied ? 'Copied!' : 'Copy'}
                  </span>
                </button>
              </div>
            )}
            {requiredFieldsAreFilled() && (
              <div
                onClick={() => validateForm() && handleCopy()}
                className={`text-sm truncate dark:text-white font-mono ${!title.trim() || !description.trim() ? 'text-textDisabled' : 'text-textStandard'}`}
              >
                {deeplink}
              </div>
            )}
          </div>
          {/* Action Buttons */}
          <div className="flex flex-col space-y-3 pt-4">
            <div className="flex gap-3">
              <button
                onClick={handleSaveRecipeClick}
                disabled={!requiredFieldsAreFilled() || saving}
                className="flex-1 inline-flex items-center justify-center gap-2 px-4 py-3 bg-bgStandard text-textStandard border border-borderStandard rounded-lg hover:bg-bgSubtle transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Save className="w-4 h-4" />
                {saving ? 'Saving...' : 'Save Recipe'}
              </button>
              <button
                onClick={() => setIsScheduleModalOpen(true)}
                disabled={!requiredFieldsAreFilled()}
                className="flex-1 inline-flex items-center justify-center gap-2 px-4 py-3 bg-textProminent text-bgApp rounded-lg hover:bg-opacity-90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Calendar className="w-4 h-4" />
                Create Schedule
              </button>
            </div>
            <button
              onClick={() => {
                localStorage.removeItem('recipe_editor_extensions');
                window.close();
              }}
              className="w-full p-3 text-textSubtle rounded-lg hover:bg-bgSubtle transition-colors"
            >
              Close
            </button>
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
        <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black bg-opacity-50">
          <div className="bg-bgApp border border-borderSubtle rounded-lg p-6 w-96 max-w-[90vw]">
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
                  className="w-full p-3 border border-borderSubtle rounded-lg bg-bgApp text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
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
