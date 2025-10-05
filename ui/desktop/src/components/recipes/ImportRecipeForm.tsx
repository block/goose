import { useState } from 'react';
import { useForm } from '@tanstack/react-form';
import { z } from 'zod';
import { Download } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Recipe, decodeRecipe } from '../../recipe';
import { saveRecipe } from '../../recipe/recipeStorage';
import * as yaml from 'yaml';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { RecipeTitleField } from './shared/RecipeTitleField';
import { listSavedRecipes } from '../../recipe/recipeStorage';
import {
  validateRecipe,
  getValidationErrorMessages,
  getRecipeJsonSchema,
} from '../../recipe/validation';

interface ImportRecipeFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

// Define Zod schema for the import form
const importRecipeSchema = z
  .object({
    deeplink: z
      .string()
      .refine(
        (value) => !value || value.trim().startsWith('goose://recipe?config='),
        'Invalid deeplink format. Expected: goose://recipe?config=...'
      ),
    recipeUploadFile: z
      .instanceof(File)
      .nullable()
      .refine((file) => {
        if (!file) return true;
        return file.size <= 1024 * 1024;
      }, 'File is too large, max size is 1MB'),
    recipeTitle: z
      .string()
      .min(1, 'Recipe title is required')
      .max(100, 'Recipe title must be 100 characters or less')
      .refine((title) => title.trim().length > 0, 'Recipe title cannot be empty')
      .refine(
        (title) => /^[^<>:"/\\|?*]+$/.test(title.trim()),
        'Recipe title contains invalid characters (< > : " / \\ | ? *)'
      ),
    global: z.boolean(),
  })
  .refine((data) => (data.deeplink && data.deeplink.trim()) || data.recipeUploadFile, {
    message: 'Either of deeplink or recipe file are required',
    path: ['deeplink'],
  });

export default function ImportRecipeForm({ isOpen, onClose, onSuccess }: ImportRecipeFormProps) {
  const [importing, setImporting] = useState(false);
  const [showSchemaModal, setShowSchemaModal] = useState(false);

  useEscapeKey(isOpen, onClose);

  const parseDeeplink = async (deeplink: string): Promise<Recipe | null> => {
    try {
      const cleanLink = deeplink.trim();

      if (!cleanLink.startsWith('goose://recipe?config=')) {
        throw new Error('Invalid deeplink format. Expected: goose://recipe?config=...');
      }

      const recipeEncoded = cleanLink.replace('goose://recipe?config=', '');

      if (!recipeEncoded) {
        throw new Error('No recipe configuration found in deeplink');
      }
      const recipe = await decodeRecipe(recipeEncoded);

      if (!recipe.title || !recipe.description) {
        throw new Error('Recipe is missing required fields (title, description)');
      }

      if (!recipe.instructions && !recipe.prompt) {
        throw new Error('Recipe must have either instructions or prompt');
      }

      return recipe;
    } catch (error) {
      console.error('Failed to parse deeplink:', error);
      return null;
    }
  };

  const parseRecipeUploadFile = async (fileContent: string, fileName: string): Promise<Recipe> => {
    const isJsonFile = fileName.toLowerCase().endsWith('.json');
    let parsed;

    try {
      if (isJsonFile) {
        parsed = JSON.parse(fileContent);
      } else {
        parsed = yaml.parse(fileContent);
      }
    } catch (error) {
      throw new Error(
        `Failed to parse ${isJsonFile ? 'JSON' : 'YAML'} file: ${error instanceof Error ? error.message : 'Invalid format'}`
      );
    }

    if (!parsed) {
      throw new Error(`${isJsonFile ? 'JSON' : 'YAML'} file is empty or contains invalid content`);
    }

    // Handle both CLI format (flat structure) and Desktop format (nested under 'recipe' key)
    const recipe = parsed.recipe || parsed;

    return recipe as Recipe;
  };

  const validateTitleUniqueness = async (title: string): Promise<string | undefined> => {
    if (!title.trim()) return undefined;

    try {
      const existingRecipes = await listSavedRecipes();
      const titleExists = existingRecipes.some(
        (recipe) => recipe.recipe.title?.toLowerCase() === title.toLowerCase()
      );

      if (titleExists) {
        return `A recipe with the same title already exists`;
      }
    } catch (error) {
      console.warn('Failed to validate title uniqueness:', error);
    }

    return undefined;
  };

  const importRecipeForm = useForm({
    defaultValues: {
      deeplink: '',
      recipeUploadFile: null as File | null,
      recipeTitle: '',
      global: true,
    },
    validators: {
      onChange: importRecipeSchema,
    },
    onSubmit: async ({ value }) => {
      setImporting(true);
      try {
        let recipe: Recipe;

        // Parse recipe from either deeplink or recipe file
        if (value.deeplink && value.deeplink.trim()) {
          const parsedRecipe = await parseDeeplink(value.deeplink.trim());
          if (!parsedRecipe) {
            throw new Error('Invalid deeplink or recipe format');
          }
          recipe = parsedRecipe;
        } else {
          const fileContent = await value.recipeUploadFile!.text();
          recipe = await parseRecipeUploadFile(fileContent, value.recipeUploadFile!.name);
        }

        recipe.title = value.recipeTitle.trim();

        const titleValidationError = await validateTitleUniqueness(value.recipeTitle.trim());
        if (titleValidationError) {
          throw new Error(titleValidationError);
        }

        const validationResult = validateRecipe(recipe);
        if (!validationResult.success) {
          const errorMessages = getValidationErrorMessages(validationResult.errors);
          throw new Error(`Recipe validation failed: ${errorMessages.join(', ')}`);
        }

        await saveRecipe(recipe, {
          name: '',
          title: value.recipeTitle.trim(),
          global: value.global,
        });

        // Reset dialog state
        importRecipeForm.reset({
          deeplink: '',
          recipeUploadFile: null,
          recipeTitle: '',
          global: true,
        });
        onClose();

        onSuccess();

        toastSuccess({
          title: value.recipeTitle.trim(),
          msg: 'Recipe imported successfully',
        });
      } catch (error) {
        console.error('Failed to import recipe:', error);

        toastError({
          title: 'Import Failed',
          msg: `Failed to import recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
          traceback: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setImporting(false);
      }
    },
  });

  const handleClose = () => {
    importRecipeForm.reset({
      deeplink: '',
      recipeUploadFile: null,
      recipeTitle: '',
      global: true,
    });
    onClose();
  };

  let recipeTitleFieldRef: { handleChange: (value: string) => void } | null = null;

  const handleDeeplinkChange = async (
    value: string,
    field: { handleChange: (value: string) => void }
  ) => {
    field.handleChange(value);

    if (value.trim()) {
      try {
        const recipe = await parseDeeplink(value.trim());
        if (recipe && recipe.title) {
          if (recipeTitleFieldRef) {
            recipeTitleFieldRef.handleChange(recipe.title);
          } else {
            importRecipeForm.setFieldValue('recipeTitle', recipe.title);
          }
        }
      } catch (error) {
        toastError({
          title: 'Invalid Deeplink',
          msg: `The deeplink format is invalid: ${error instanceof Error ? error.message : 'Unknown error'}`,
        });
      }
    } else {
      if (recipeTitleFieldRef) {
        recipeTitleFieldRef.handleChange('');
      } else {
        importRecipeForm.setFieldValue('recipeTitle', '');
      }
    }
  };

  const handleRecipeUploadChange = async (file: File | undefined) => {
    importRecipeForm.setFieldValue('recipeUploadFile', file || null);

    if (file) {
      try {
        const fileContent = await file.text();
        const recipe = await parseRecipeUploadFile(fileContent, file.name);
        if (recipe.title) {
          if (recipeTitleFieldRef) {
            recipeTitleFieldRef.handleChange(recipe.title);
          } else {
            importRecipeForm.setFieldValue('recipeTitle', recipe.title);
          }
        }
      } catch (error) {
        toastError({
          title: 'Invalid Recipe File',
          msg: `The recipe file format is invalid: ${error instanceof Error ? error.message : 'Unknown error'}`,
        });
      }
    } else {
      if (recipeTitleFieldRef) {
        recipeTitleFieldRef.handleChange('');
      } else {
        importRecipeForm.setFieldValue('recipeTitle', '');
      }
    }
  };

  if (!isOpen) return null;

  return (
    <>
      <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
        <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[500px] max-w-[90vw]">
          <h3 className="text-lg font-medium text-text-standard mb-4">Import Recipe</h3>

          <form
            onSubmit={(e) => {
              e.preventDefault();
              e.stopPropagation();
              importRecipeForm.handleSubmit();
            }}
          >
            <div className="space-y-4">
              <importRecipeForm.Subscribe selector={(state) => state.values}>
                {(values) => (
                  <>
                    <importRecipeForm.Field name="deeplink">
                      {(field) => {
                        const isDisabled = values.recipeUploadFile !== null;

                        return (
                          <div className={isDisabled ? 'opacity-50' : ''}>
                            <label
                              htmlFor="import-deeplink"
                              className="block text-sm font-medium text-text-standard mb-2"
                            >
                              Recipe Deeplink
                            </label>
                            <textarea
                              id="import-deeplink"
                              value={field.state.value}
                              onChange={(e) => handleDeeplinkChange(e.target.value, field)}
                              onBlur={field.handleBlur}
                              disabled={isDisabled}
                              className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none ${
                                field.state.meta.errors.length > 0
                                  ? 'border-red-500'
                                  : 'border-border-subtle'
                              } ${isDisabled ? 'cursor-not-allowed bg-gray-40 text-gray-300' : ''}`}
                              placeholder="Paste your goose://recipe?config=... deeplink here"
                              rows={3}
                              autoFocus={!isDisabled}
                            />
                            <p
                              className={`text-xs mt-1 ${isDisabled ? 'text-gray-300' : 'text-text-muted'}`}
                            >
                              Paste a recipe deeplink starting with "goose://recipe?config="
                            </p>
                            {field.state.meta.errors.length > 0 && (
                              <p className="text-red-500 text-sm mt-1">
                                {typeof field.state.meta.errors[0] === 'string'
                                  ? field.state.meta.errors[0]
                                  : field.state.meta.errors[0]?.message ||
                                    String(field.state.meta.errors[0])}
                              </p>
                            )}
                          </div>
                        );
                      }}
                    </importRecipeForm.Field>

                    <div className="relative">
                      <div className="absolute inset-0 flex items-center">
                        <div className="w-full border-t border-border-subtle" />
                      </div>
                      <div className="relative flex justify-center text-sm">
                        <span className="px-3 bg-background-default text-text-muted font-medium">
                          OR
                        </span>
                      </div>
                    </div>

                    <importRecipeForm.Field name="recipeUploadFile">
                      {(field) => {
                        const hasDeeplink = values.deeplink?.trim();
                        const isDisabled = !!hasDeeplink;

                        return (
                          <div className={isDisabled ? 'opacity-50' : ''}>
                            <label
                              htmlFor="import-recipe-file"
                              className="block text-sm font-medium text-text-standard mb-3"
                            >
                              Recipe File
                            </label>
                            <div className="relative">
                              <Input
                                id="import-recipe-file"
                                type="file"
                                accept=".yaml,.yml,.json"
                                disabled={isDisabled}
                                onChange={(e) => {
                                  handleRecipeUploadChange(e.target.files?.[0]);
                                }}
                                onBlur={field.handleBlur}
                                className={`file:pt-1 ${field.state.meta.errors.length > 0 ? 'border-red-500' : ''} ${
                                  isDisabled ? 'cursor-not-allowed' : ''
                                }`}
                              />
                            </div>
                            <div className="flex items-center justify-between">
                              <p
                                className={`text-xs mt-1 ${isDisabled ? 'text-gray-300' : 'text-text-muted'}`}
                              >
                                Upload a YAML or JSON file containing the recipe structure
                              </p>
                              <button
                                type="button"
                                onClick={() => setShowSchemaModal(true)}
                                className="text-xs text-blue-500 hover:text-blue-700 underline"
                                disabled={isDisabled}
                              >
                                example
                              </button>
                            </div>
                            {field.state.meta.errors.length > 0 && (
                              <p className="text-red-500 text-sm mt-1">
                                {typeof field.state.meta.errors[0] === 'string'
                                  ? field.state.meta.errors[0]
                                  : field.state.meta.errors[0]?.message ||
                                    String(field.state.meta.errors[0])}
                              </p>
                            )}
                          </div>
                        );
                      }}
                    </importRecipeForm.Field>
                  </>
                )}
              </importRecipeForm.Subscribe>

              <p className="text-xs text-text-muted">
                Ensure you review contents of recipe files before adding them to your goose
                interface.
              </p>

              <importRecipeForm.Field name="recipeTitle">
                {(field) => {
                  // Store reference to the field for programmatic updates
                  recipeTitleFieldRef = field;

                  return (
                    <RecipeTitleField
                      id="import-recipe-title"
                      value={field.state.value}
                      onChange={field.handleChange}
                      onBlur={field.handleBlur}
                      errors={field.state.meta.errors.map((error) =>
                        typeof error === 'string' ? error : error?.message || String(error)
                      )}
                    />
                  );
                }}
              </importRecipeForm.Field>

              <importRecipeForm.Field name="global">
                {(field) => (
                  <div>
                    <label className="block text-sm font-medium text-text-standard mb-2">
                      Save Location
                    </label>
                    <div className="space-y-2">
                      <label className="flex items-center">
                        <input
                          type="radio"
                          name="import-save-location"
                          checked={field.state.value === true}
                          onChange={() => field.handleChange(true)}
                          className="mr-2"
                        />
                        <span className="text-sm text-text-standard">
                          Global - Available across all Goose sessions
                        </span>
                      </label>
                      <label className="flex items-center">
                        <input
                          type="radio"
                          name="import-save-location"
                          checked={field.state.value === false}
                          onChange={() => field.handleChange(false)}
                          className="mr-2"
                        />
                        <span className="text-sm text-text-standard">
                          Directory - Available in the working directory
                        </span>
                      </label>
                    </div>
                  </div>
                )}
              </importRecipeForm.Field>
            </div>

            <div className="flex justify-end space-x-3 mt-6">
              <Button type="button" onClick={handleClose} variant="ghost" disabled={importing}>
                Cancel
              </Button>
              <importRecipeForm.Subscribe
                selector={(state) => [state.canSubmit, state.isSubmitting]}
              >
                {([canSubmit, isSubmitting]) => (
                  <Button
                    type="submit"
                    disabled={!canSubmit || importing || isSubmitting}
                    variant="default"
                  >
                    {importing || isSubmitting ? 'Importing...' : 'Import Recipe'}
                  </Button>
                )}
              </importRecipeForm.Subscribe>
            </div>
          </form>
        </div>
      </div>

      {/* Schema Modal */}
      {showSchemaModal && (
        <div className="fixed inset-0 z-[400] flex items-center justify-center bg-black/50">
          <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[800px] max-w-[90vw] max-h-[80vh] flex flex-col">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-medium text-text-standard">Expected Recipe Structure</h3>
              <button
                type="button"
                onClick={() => setShowSchemaModal(false)}
                className="text-text-muted hover:text-text-standard"
              >
                ✕
              </button>
            </div>
            <p className="mt-4 text-blue-700 text-sm">
              Your YAML or JSON file should follow this structure. Required fields are: title,
              description, and either instructions or prompt.
            </p>
            <div className="flex-1 overflow-auto">
              <pre className="text-xs bg-whitedark:bg-gray-800 p-4 rounded overflow-auto whitespace-pre font-mono">
                {JSON.stringify(getRecipeJsonSchema(), null, 2)}
              </pre>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export function ImportRecipeButton({ onClick }: { onClick: () => void }) {
  return (
    <Button onClick={onClick} variant="default" size="sm" className="flex items-center gap-2">
      <Download className="w-4 h-4" />
      Import Recipe
    </Button>
  );
}
