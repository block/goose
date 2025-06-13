import { useState, useEffect } from 'react';
import { Recipe } from '../recipe';
import { Buffer } from 'buffer';
import { FullExtensionConfig } from '../extensions';
import { Geese } from './icons/Geese';
import Copy from './icons/Copy';
import { Check } from 'lucide-react';
import { useConfig } from './ConfigContext';
import { FixedExtensionEntry } from './ConfigContext';
import RecipeActivityEditor from './RecipeActivityEditor';
import RecipeInfoModal from './RecipeInfoModal';
import RecipeExpandableInfo from './RecipeExpandableInfo';
import { ScheduleFromRecipeModal } from './schedule/ScheduleFromRecipeModal';

interface RecipeEditorProps {
  config?: Recipe;
}

// Function to generate a deep link from a recipe
function generateDeepLink(recipe: Recipe): string {
  const configBase64 = Buffer.from(JSON.stringify(recipe)).toString('base64');
  return `goose://recipe?config=${configBase64}`;
}

export default function RecipeEditor({ config }: RecipeEditorProps) {
  const { getExtensions } = useConfig();
  const [recipeConfig] = useState<Recipe | undefined>(config);
  const [title, setTitle] = useState(config?.title || '');
  const [description, setDescription] = useState(config?.description || '');
  const [instructions, setInstructions] = useState(config?.instructions || '');
  const [prompt, setPrompt] = useState(config?.prompt || '');
  const [activities, setActivities] = useState<string[]>(config?.activities || []);
  const [extensionOptions, setExtensionOptions] = useState<FixedExtensionEntry[]>([]);
  const [extensionsLoaded, setExtensionsLoaded] = useState(false);
  const [copied, setCopied] = useState(false);
  const [isRecipeInfoModalOpen, setRecipeInfoModalOpen] = useState(false);
  const [isScheduleModalOpen, setIsScheduleModalOpen] = useState(false);
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

  const getCurrentConfig = (): Recipe => {
    console.log('Creating config with:', {
      selectedExtensions: recipeExtensions,
      availableExtensions: extensionOptions,
      recipeConfig,
    });

    const config = {
      ...recipeConfig,
      title,
      description,
      instructions,
      activities,
      prompt,
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
          <div className="flex flex-col space-y-2 pt-1">
            <button
              onClick={() => setIsScheduleModalOpen(true)}
              disabled={!requiredFieldsAreFilled()}
              className="w-full h-[60px] rounded-none border-t text-gray-900 dark:text-white hover:bg-gray-50 dark:border-gray-600 text-lg font-medium"
            >
              Create Schedule from Recipe
            </button>
            <button
              onClick={() => {
                localStorage.removeItem('recipe_editor_extensions');
                window.close();
              }}
              className="w-full p-3 text-textSubtle rounded-lg hover:bg-bgSubtle"
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
    </div>
  );
}
