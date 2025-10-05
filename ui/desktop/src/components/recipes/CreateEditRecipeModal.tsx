import React, { useState, useEffect, useCallback } from 'react';
import { useForm } from '@tanstack/react-form';
import { Recipe, generateDeepLink, Parameter } from '../../recipe';
import { Geese } from '../icons/Geese';
import Copy from '../icons/Copy';
import { Check, Save, Calendar, X, Play } from 'lucide-react';
import { ExtensionConfig, useConfig } from '../ConfigContext';
import { FixedExtensionEntry } from '../ConfigContext';
import { ScheduleFromRecipeModal } from '../schedule/ScheduleFromRecipeModal';
import { Button } from '../ui/button';

import { RecipeFormFields } from './shared/RecipeFormFields';
import { RecipeFormData } from './shared/recipeFormSchema';
import { saveRecipe, generateRecipeFilename } from '../../recipe/recipeStorage';
import { toastSuccess, toastError } from '../../toasts';

interface CreateEditRecipeModalProps {
  isOpen: boolean;
  onClose: (wasSaved?: boolean) => void;
  recipe?: Recipe;
  recipeName?: string;
  isCreateMode?: boolean;
}

export default function CreateEditRecipeModal({
  isOpen,
  onClose,
  recipe,
  recipeName: initialRecipeName,
  isCreateMode = false,
}: CreateEditRecipeModalProps) {
  const { getExtensions } = useConfig();

  const getInitialValues = React.useCallback((): RecipeFormData => {
    if (recipe) {
      return {
        title: recipe.title || '',
        description: recipe.description || '',
        instructions: recipe.instructions || '',
        prompt: recipe.prompt || '',
        activities: recipe.activities || [],
        parameters: recipe.parameters || [],
        jsonSchema: recipe.response?.json_schema
          ? JSON.stringify(recipe.response.json_schema, null, 2)
          : '',
        recipeName: initialRecipeName || '',
        global: true,
      };
    }
    return {
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: [],
      parameters: [],
      jsonSchema: '',
      recipeName: '',
      global: true,
    };
  }, [recipe, initialRecipeName]);

  const form = useForm({
    defaultValues: getInitialValues(),
  });

  // Helper functions to get values from form - using state to trigger re-renders
  const [title, setTitle] = useState(form.state.values.title);
  const [description, setDescription] = useState(form.state.values.description);
  const [instructions, setInstructions] = useState(form.state.values.instructions);
  const [prompt, setPrompt] = useState(form.state.values.prompt);
  const [activities, setActivities] = useState(form.state.values.activities);
  const [parameters, setParameters] = useState(form.state.values.parameters);
  const [jsonSchema, setJsonSchema] = useState(form.state.values.jsonSchema);

  // Subscribe to form changes to update local state
  useEffect(() => {
    return form.store.subscribe(() => {
      setTitle(form.state.values.title);
      setDescription(form.state.values.description);
      setInstructions(form.state.values.instructions);
      setPrompt(form.state.values.prompt);
      setActivities(form.state.values.activities);
      setParameters(form.state.values.parameters);
      setJsonSchema(form.state.values.jsonSchema);
      setRecipeName(form.state.values.recipeName);
      setGlobal(form.state.values.global);
    });
  }, [form]);
  const [extensionOptions, setExtensionOptions] = useState<FixedExtensionEntry[]>([]);
  const [extensionsLoaded, setExtensionsLoaded] = useState(false);
  const [copied, setCopied] = useState(false);
  const [isScheduleModalOpen, setIsScheduleModalOpen] = useState(false);
  const [recipeName, setRecipeName] = useState(form.state.values.recipeName);
  const [global, setGlobal] = useState(form.state.values.global);
  const [isSaving, setIsSaving] = useState(false);

  // Initialize selected extensions for the recipe
  const [recipeExtensions] = useState<string[]>(() => {
    if (recipe?.extensions) {
      return recipe.extensions.map((ext) => ext.name);
    }
    return [];
  });

  // Reset form when recipe changes
  useEffect(() => {
    if (recipe) {
      const newValues = getInitialValues();
      form.reset(newValues);
    }
  }, [recipe, form, getInitialValues]);

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

  const getCurrentRecipe = useCallback((): Recipe => {
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

    // Parse response schema if provided
    let responseConfig = undefined;
    if (jsonSchema && jsonSchema.trim()) {
      try {
        const parsedSchema = JSON.parse(jsonSchema);
        responseConfig = { json_schema: parsedSchema };
      } catch (error) {
        console.warn('Invalid JSON schema provided:', error);
        // If JSON is invalid, don't include response config
      }
    }

    return {
      ...recipe,
      title,
      description,
      instructions,
      activities,
      prompt: prompt || undefined,
      parameters: formattedParameters,
      response: responseConfig,
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
        .filter(Boolean) as ExtensionConfig[],
    };
  }, [
    recipe,
    title,
    description,
    instructions,
    activities,
    prompt,
    parameters,
    jsonSchema,
    recipeExtensions,
    extensionOptions,
  ]);

  const requiredFieldsAreFilled = () => {
    return title.trim() && description.trim() && (instructions.trim() || (prompt || '').trim());
  };

  const validateForm = () => {
    const basicValidation =
      title.trim() && description.trim() && (instructions.trim() || (prompt || '').trim());

    // If JSON schema is provided, it must be valid
    if (jsonSchema && jsonSchema.trim()) {
      try {
        JSON.parse(jsonSchema);
      } catch {
        return false; // Invalid JSON schema fails validation
      }
    }

    return basicValidation;
  };

  const [deeplink, setDeeplink] = useState('');
  const [isGeneratingDeeplink, setIsGeneratingDeeplink] = useState(false);

  // Generate deeplink whenever recipe configuration changes
  useEffect(() => {
    let isCancelled = false;

    const generateLink = async () => {
      if (
        !title.trim() ||
        !description.trim() ||
        (!instructions.trim() && !(prompt || '').trim())
      ) {
        setDeeplink('');
        return;
      }

      setIsGeneratingDeeplink(true);
      try {
        const currentRecipe = getCurrentRecipe();
        const link = await generateDeepLink(currentRecipe);
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
    jsonSchema,
    recipeExtensions,
    getCurrentRecipe,
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

  const handleSaveRecipeClick = async () => {
    if (!validateForm()) {
      toastError({
        title: 'Validation Failed',
        msg: 'Please fill in all required fields and ensure JSON schema is valid.',
      });
      return;
    }

    setIsSaving(true);
    try {
      const recipe = getCurrentRecipe();

      await saveRecipe(recipe, {
        name: (recipeName || '').trim(),
        global: global,
      });

      onClose(true);

      toastSuccess({
        title: (recipeName || '').trim(),
        msg: 'Recipe saved successfully',
      });
    } catch (error) {
      console.error('Failed to save recipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveAndRunRecipeClick = async () => {
    if (!validateForm()) {
      toastError({
        title: 'Validation Failed',
        msg: 'Please fill in all required fields and ensure JSON schema is valid.',
      });
      return;
    }

    setIsSaving(true);
    try {
      const recipe = getCurrentRecipe();
      const recipeName = generateRecipeFilename(recipe);

      await saveRecipe(recipe, {
        name: recipeName,
        global: true,
      });

      // Close modal first
      onClose(true);

      // Open recipe in a new window instead of navigating in the same window
      window.electron.createChatWindow(undefined, undefined, undefined, undefined, recipe);

      toastSuccess({
        title: recipeName,
        msg: 'Recipe saved and launched successfully',
      });
    } catch (error) {
      console.error('Failed to save and run recipe:', error);

      toastError({
        title: 'Save and Run Failed',
        msg: `Failed to save and run recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsSaving(false);
    }
  };

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
              <h1 className="text-xl font-medium text-textProminent">
                {isCreateMode ? 'Create Recipe' : 'View/edit recipe'}
              </h1>
              <p className="text-textSubtle text-sm">
                {isCreateMode
                  ? 'Create a new recipe to define agent behavior and capabilities.'
                  : "You can edit the recipe below to change the agent's behavior in a new session."}
              </p>
            </div>
          </div>
          <Button
            onClick={() => onClose(false)}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-bgSubtle rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4">
          <RecipeFormFields form={form} />

          {/* Deep Link Display */}
          {requiredFieldsAreFilled() && (
            <div className="w-full p-4 bg-bgSubtle rounded-lg mt-6">
              <div className="flex items-center justify-between mb-2">
                <div className="text-sm text-textSubtle">
                  Copy this link to share with friends or paste directly in Chrome to open
                </div>
                <Button
                  onClick={handleCopy}
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
              <div
                onClick={handleCopy}
                className="text-sm truncate font-mono cursor-pointer text-textStandard"
              >
                {isGeneratingDeeplink
                  ? 'Generating deeplink...'
                  : deeplink || 'Click to generate deeplink'}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-6 border-t border-borderSubtle">
          <Button
            onClick={() => onClose(false)}
            variant="ghost"
            className="px-4 py-2 text-textSubtle rounded-lg hover:bg-bgSubtle transition-colors"
          >
            Close
          </Button>

          <div className="flex gap-3">
            <Button
              onClick={() => setIsScheduleModalOpen(true)}
              disabled={!requiredFieldsAreFilled()}
              variant="outline"
              size="default"
              className="inline-flex items-center justify-center gap-2 px-4 py-2"
            >
              <Calendar className="w-4 h-4" />
              Create Schedule
            </Button>
            <Button
              onClick={handleSaveRecipeClick}
              disabled={!requiredFieldsAreFilled() || isSaving}
              variant="outline"
              size="default"
              className="inline-flex items-center justify-center gap-2 px-4 py-2"
            >
              <Save className="w-4 h-4" />
              {isSaving ? 'Saving...' : 'Save Recipe'}
            </Button>
            <Button
              onClick={handleSaveAndRunRecipeClick}
              disabled={!requiredFieldsAreFilled() || isSaving}
              variant="default"
              size="default"
              className="inline-flex items-center justify-center gap-2 px-4 py-2"
            >
              <Play className="w-4 h-4" />
              {isSaving ? 'Saving...' : 'Save & Run Recipe'}
            </Button>
          </div>
        </div>
      </div>

      <ScheduleFromRecipeModal
        isOpen={isScheduleModalOpen}
        onClose={() => setIsScheduleModalOpen(false)}
        recipe={getCurrentRecipe()}
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
