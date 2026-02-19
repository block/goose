import React, { useState, useEffect, useCallback } from 'react';
import { useForm } from '@tanstack/react-form';
import {
  Recipe,
  generateDeepLink,
  Parameter,
  encodeRecipe,
  stripEmptyExtensions,
} from '../../recipe';
import { Check, ExternalLink, Play, Save, X } from 'lucide-react';
import { Geese } from '../icons/Geese';
import Copy from '../icons/Copy';
import { ExtensionConfig } from '../ConfigContext';
import { Button } from '../ui/button';
import type { Settings } from '../../api';

import { RecipeFormFields } from './shared/RecipeFormFields';
import { RecipeFormData } from './shared/recipeFormSchema';
import { toastSuccess, toastError } from '../../toasts';
import { saveRecipe } from '../../recipe/recipe_management';
import { errorMessage } from '../../utils/conversionUtils';

interface CreateEditRecipeModalProps {
  isOpen: boolean;
  onClose: (wasSaved?: boolean) => void;
  recipe?: Recipe;
  isCreateMode?: boolean;
  recipeId?: string | null;
}

export default function CreateEditRecipeModal({
  isOpen,
  onClose,
  recipe,
  isCreateMode = false,
  recipeId,
}: CreateEditRecipeModalProps) {
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
        model: recipe.settings?.goose_model ?? undefined,
        provider: recipe.settings?.goose_provider ?? undefined,
        extensions: recipe.extensions || undefined,
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
      model: undefined,
      provider: undefined,
      extensions: undefined,
    };
  }, [recipe]);

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
  const [model, setModel] = useState(form.state.values.model);
  const [provider, setProvider] = useState(form.state.values.provider);
  const [extensions, setExtensions] = useState(form.state.values.extensions);

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
      setModel(form.state.values.model);
      setProvider(form.state.values.provider);
      setExtensions(form.state.values.extensions);
    });
  }, [form]);
  const [copied, setCopied] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Reset form when recipe changes
  useEffect(() => {
    if (recipe) {
      const newValues = getInitialValues();
      form.reset(newValues);
    }
  }, [recipe, form, getInitialValues]);

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

    const cleanedExtensions = extensions?.map(
      (extension: ExtensionConfig & { envs?: unknown; enabled?: boolean }) => {
        const { envs: _envs, enabled: _enabled, ...rest } = extension;
        return rest;
      }
    ) as ExtensionConfig[] | undefined;

    const mergedSettings: Settings = {
      ...(recipe?.settings || {}),
    };
    if (model !== undefined) {
      mergedSettings.goose_model = model || null;
    } else if ('goose_model' in mergedSettings) {
      delete mergedSettings.goose_model;
    }
    if (provider !== undefined) {
      mergedSettings.goose_provider = provider || null;
    } else if ('goose_provider' in mergedSettings) {
      delete mergedSettings.goose_provider;
    }
    const settings = Object.values(mergedSettings).some(
      (value) => value !== undefined && value !== null
    )
      ? mergedSettings
      : undefined;

    return {
      ...recipe,
      title,
      description,
      instructions,
      activities,
      prompt: prompt || undefined,
      parameters: formattedParameters,
      response: responseConfig,
      extensions: cleanedExtensions,
      settings,
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
    model,
    provider,
    extensions,
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
    model,
    provider,
    extensions,
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

      await saveRecipe(recipe, recipeId);

      onClose(true);

      toastSuccess({
        title: (recipe.title || '').trim(),
        msg: 'Recipe saved successfully',
      });
    } catch (error) {
      console.error('Failed to save recipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save recipe: ${errorMessage(error, 'Unknown error')}`,
        traceback: errorMessage(error),
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

      await saveRecipe(recipe, recipeId);

      // Close modal first
      onClose(true);

      // Encode the recipe as a deeplink before passing to the new window
      const encodedRecipe = await encodeRecipe(stripEmptyExtensions(recipe));
      window.electron.createChatWindow(
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        encodedRecipe
      );

      toastSuccess({
        title: recipe.title,
        msg: 'Recipe saved and launched successfully',
      });
    } catch (error) {
      console.error('Failed to save and run recipe:', error);

      toastError({
        title: 'Save and Run Failed',
        msg: `Failed to save and run recipe: ${errorMessage(error, 'Unknown error')}`,
        traceback: errorMessage(error),
      });
    } finally {
      setIsSaving(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[400] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-border-default rounded-lg w-[90vw] max-w-4xl h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-border-default">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-background-default rounded-full flex items-center justify-center">
              <Geese className="w-6 h-6 text-iconProminent" />
            </div>
            <div>
              <h1 className="text-xl font-medium text-text-default">
                {isCreateMode ? 'Create Recipe' : 'View/edit recipe'}
              </h1>
              <p className="text-text-muted text-sm">
                {isCreateMode
                  ? 'Create a new recipe to define agent behavior and capabilities for reusable chat sessions.'
                  : "You can edit the recipe below to change the agent's behavior in a new session."}{' '}
                <a
                  href="https://block.github.io/goose/docs/guides/recipes/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 text-blue-500 hover:text-blue-600 hover:underline"
                >
                  Learn more
                  <ExternalLink className="w-3 h-3" />
                </a>
              </p>
            </div>
          </div>
          <Button
            onClick={() => onClose(false)}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-background-muted rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4">
          <RecipeFormFields form={form} />

          {/* Deep Link Display */}
          {requiredFieldsAreFilled() && (
            <div className="w-full p-4 bg-background-muted rounded-lg mt-6">
              <div className="flex items-center justify-between mb-2">
                <div className="text-sm text-text-muted">
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
                  <span className="ml-1 text-sm text-text-muted">
                    {copied ? 'Copied!' : 'Copy'}
                  </span>
                </Button>
              </div>
              <div
                onClick={handleCopy}
                className="text-sm truncate font-mono cursor-pointer text-text-default"
              >
                {isGeneratingDeeplink
                  ? 'Generating deeplink...'
                  : deeplink || 'Click to generate deeplink'}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-6 border-t border-border-default">
          <Button
            onClick={() => onClose(false)}
            variant="ghost"
            className="px-4 py-2 text-text-muted rounded-lg hover:bg-background-muted transition-colors"
          >
            Close
          </Button>

          <div className="flex gap-3">
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
    </div>
  );
}
