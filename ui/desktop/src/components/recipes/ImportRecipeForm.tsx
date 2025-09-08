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
import { RecipeNameField, recipeNameSchema } from './shared/RecipeNameField';
import { generateRecipeNameFromTitle } from './shared/recipeNameUtils';

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
    yamlFile: z
      .instanceof(File)
      .nullable()
      .refine((file) => {
        if (!file) return true;
        return file.size <= 1024 * 1024;
      }, 'File is too large, max size is 1MB'),
    recipeName: recipeNameSchema,
    global: z.boolean(),
  })
  .refine((data) => (data.deeplink && data.deeplink.trim()) || data.yamlFile, {
    message: 'Either of deeplink or YAML file are required',
    path: ['deeplink'],
  });

export default function ImportRecipeForm({ isOpen, onClose, onSuccess }: ImportRecipeFormProps) {
  const [importing, setImporting] = useState(false);

  // Handle Esc key for modal
  useEscapeKey(isOpen, onClose);

  // Function to parse deeplink and extract recipe
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

  const parseYamlFile = async (fileContent: string): Promise<Recipe> => {
    const parsed = yaml.parse(fileContent);

    if (!parsed) {
      throw new Error('YAML file is empty or contains invalid content');
    }

    // Handle both CLI format (flat structure) and Desktop format (nested under 'recipe' key)
    const recipe = parsed.recipe || parsed;

    return recipe as Recipe;
  };

  const importRecipeForm = useForm({
    defaultValues: {
      deeplink: '',
      yamlFile: null as File | null,
      recipeName: '',
      global: true,
    },
    validators: {
      onChange: importRecipeSchema,
    },
    onSubmit: async ({ value }) => {
      setImporting(true);
      try {
        let recipe: Recipe;

        // Parse recipe from either deeplink or YAML file
        if (value.deeplink && value.deeplink.trim()) {
          const parsedRecipe = await parseDeeplink(value.deeplink.trim());
          if (!parsedRecipe) {
            throw new Error('Invalid deeplink or recipe format');
          }
          recipe = parsedRecipe;
        } else {
          const fileContent = await value.yamlFile!.text();
          recipe = await parseYamlFile(fileContent);
        }

        await saveRecipe(recipe, {
          name: value.recipeName.trim(),
          global: value.global,
        });

        // Reset dialog state
        importRecipeForm.reset({
          deeplink: '',
          yamlFile: null,
          recipeName: '',
          global: true,
        });
        onClose();

        onSuccess();

        toastSuccess({
          title: value.recipeName.trim(),
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
    // Reset form to default values
    importRecipeForm.reset({
      deeplink: '',
      yamlFile: null,
      recipeName: '',
      global: true,
    });
    onClose();
  };

  // Store reference to recipe name field for programmatic updates
  let recipeNameFieldRef: { handleChange: (value: string) => void } | null = null;

  // Auto-generate recipe name when deeplink changes
  const handleDeeplinkChange = async (
    value: string,
    field: { handleChange: (value: string) => void }
  ) => {
    // Use the proper field change handler to trigger validation
    field.handleChange(value);

    if (value.trim()) {
      try {
        const recipe = await parseDeeplink(value.trim());
        if (recipe && recipe.title) {
          const suggestedName = generateRecipeNameFromTitle(recipe.title);

          // Use the recipe name field's handleChange method if available
          if (recipeNameFieldRef) {
            recipeNameFieldRef.handleChange(suggestedName);
          } else {
            importRecipeForm.setFieldValue('recipeName', suggestedName);
          }
        }
      } catch (error) {
        // Silently handle parsing errors during auto-suggest
        console.log('Could not parse deeplink for auto-suggest:', error);
      }
    } else {
      // Clear the recipe name when deeplink is empty
      if (recipeNameFieldRef) {
        recipeNameFieldRef.handleChange('');
      } else {
        importRecipeForm.setFieldValue('recipeName', '');
      }
    }
  };

  const handleYamlFileChange = async (file: File | undefined) => {
    importRecipeForm.setFieldValue('yamlFile', file || null);

    if (file) {
      try {
        const fileContent = await file.text();
        const recipe = await parseYamlFile(fileContent);
        if (recipe.title) {
          const suggestedName = generateRecipeNameFromTitle(recipe.title);

          // Use the recipe name field's handleChange method if available
          if (recipeNameFieldRef) {
            recipeNameFieldRef.handleChange(suggestedName);
          } else {
            importRecipeForm.setFieldValue('recipeName', suggestedName);
          }
        }
      } catch (error) {
        // Silently handle parsing errors during auto-suggest
        console.log('Could not parse YAML file for auto-suggest:', error);
      }
    } else {
      // Clear the recipe name when file is removed
      if (recipeNameFieldRef) {
        recipeNameFieldRef.handleChange('');
      } else {
        importRecipeForm.setFieldValue('recipeName', '');
      }
    }
  };

  if (!isOpen) return null;

  return (
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
                      const isDisabled = values.yamlFile !== null;

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

                  <importRecipeForm.Field name="yamlFile">
                    {(field) => {
                      const hasDeeplink = values.deeplink?.trim();
                      const isDisabled = !!hasDeeplink;

                      return (
                        <div className={isDisabled ? 'opacity-50' : ''}>
                          <label
                            htmlFor="import-yaml-file"
                            className="block text-sm font-medium text-text-standard mb-3"
                          >
                            Recipe YAML File
                          </label>
                          <div className="relative">
                            <Input
                              id="import-yaml-file"
                              type="file"
                              accept=".yaml,.yml"
                              disabled={isDisabled}
                              onChange={(e) => {
                                handleYamlFileChange(e.target.files?.[0]);
                              }}
                              onBlur={field.handleBlur}
                              className={`${field.state.meta.errors.length > 0 ? 'border-red-500' : ''} ${
                                isDisabled ? 'cursor-not-allowed' : ''
                              }`}
                            />
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
              Ensure you review contents of YAML files before adding them to your goose interface.
            </p>

            <importRecipeForm.Field name="recipeName">
              {(field) => {
                // Store reference to the field for programmatic updates
                recipeNameFieldRef = field;

                return (
                  <RecipeNameField
                    id="import-recipe-name"
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
            <importRecipeForm.Subscribe selector={(state) => [state.canSubmit, state.isSubmitting]}>
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
  );
}

// Export the button component for easy access
export function ImportRecipeButton({ onClick }: { onClick: () => void }) {
  return (
    <Button onClick={onClick} variant="default" size="sm" className="flex items-center gap-2">
      <Download className="w-4 h-4" />
      Import Recipe
    </Button>
  );
}
