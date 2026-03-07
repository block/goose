import { useMemo, useState } from 'react';
import { useForm } from '@tanstack/react-form';
import { z } from 'zod';
import { Download } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Recipe, parseDeeplink, parseRecipeFromFile } from '../../recipe';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { getRecipeJsonSchema } from '../../recipe/validation';
import { saveRecipe } from '../../recipe/recipe_management';
import { errorMessage } from '../../utils/conversionUtils';
import { useLocalization } from '../../contexts/LocalizationContext';

interface ImportRecipeFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function ImportRecipeForm({ isOpen, onClose, onSuccess }: ImportRecipeFormProps) {
  const { t } = useLocalization();
  const [importing, setImporting] = useState(false);
  const [showSchemaModal, setShowSchemaModal] = useState(false);

  useEscapeKey(isOpen, onClose);

  const importRecipeSchema = useMemo(
    () =>
      z
        .object({
          deeplink: z
            .string()
            .refine(
              (value) => !value || value.trim().startsWith('goose://recipe?config='),
              t('recipes.import.validation.invalidDeeplinkFormat')
            ),
          recipeUploadFile: z
            .instanceof(File)
            .nullable()
            .refine((file) => {
              if (!file) return true;
              return file.size <= 1024 * 1024;
            }, t('recipes.import.validation.fileTooLarge')),
        })
        .refine((data) => (data.deeplink && data.deeplink.trim()) || data.recipeUploadFile, {
          message: t('recipes.import.validation.deeplinkOrFileRequired'),
          path: ['deeplink'],
        }),
    [t]
  );

  const importRecipeForm = useForm({
    defaultValues: {
      deeplink: '',
      recipeUploadFile: null as File | null,
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
            throw new Error(t('recipes.import.validation.invalidDeeplinkOrRecipe'));
          }
          recipe = parsedRecipe;
        } else {
          const fileContent = await value.recipeUploadFile!.text();
          recipe = await parseRecipeFromFile(fileContent);
        }

        await saveRecipe(recipe, null);

        // Reset dialog state
        importRecipeForm.reset({
          deeplink: '',
          recipeUploadFile: null,
        });
        onClose();

        onSuccess();

        toastSuccess({
          title: recipe.title.trim(),
          msg: t('recipes.import.importSuccess'),
        });
      } catch (error) {
        console.error('Failed to import recipe:', error);

        toastError({
          title: t('recipes.import.importFailedTitle'),
          msg: t('recipes.import.importFailedMessage', {
            error: errorMessage(error, t('common.labels.unknown')),
          }),
          traceback: errorMessage(error),
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
    });
    onClose();
  };

  const handleDeeplinkChange = async (
    value: string,
    field: { handleChange: (value: string) => void }
  ) => {
    field.handleChange(value);

    if (value.trim()) {
      try {
        await parseDeeplink(value.trim());
      } catch (error) {
        toastError({
          title: t('recipes.import.invalidDeeplinkTitle'),
          msg: t('recipes.import.invalidDeeplinkMessage', {
            error: errorMessage(error, t('common.labels.unknown')),
          }),
        });
      }
    }
  };

  const handleRecipeUploadChange = async (file: File | undefined) => {
    importRecipeForm.setFieldValue('recipeUploadFile', file || null);

    if (file) {
      try {
        const fileContent = await file.text();
        await parseRecipeFromFile(fileContent);
      } catch (error) {
        toastError({
          title: t('recipes.import.invalidFileTitle'),
          msg: errorMessage(error, t('common.labels.unknown')),
        });
      }
    }
  };

  if (!isOpen) return null;

  return (
    <>
      <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
        <div className="bg-background-primary border border-border-primary rounded-lg p-6 w-[500px] max-w-[90vw]">
          <h3 className="text-lg font-medium text-text-primary mb-4">
            {t('recipes.import.title')}
          </h3>

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
                              className="block text-sm font-medium text-text-primary mb-2"
                            >
                              {t('recipes.import.deeplinkLabel')}
                            </label>
                            <textarea
                              id="import-deeplink"
                              value={field.state.value}
                              onChange={(e) => handleDeeplinkChange(e.target.value, field)}
                              onBlur={field.handleBlur}
                              disabled={isDisabled}
                              className={`w-full p-3 border rounded-lg bg-background-primary text-text-primary focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none ${
                                field.state.meta.errors.length > 0
                                  ? 'border-red-500'
                                  : 'border-border-primary'
                              } ${isDisabled ? 'cursor-not-allowed bg-gray-40 text-gray-300' : ''}`}
                              placeholder={t('recipes.import.deeplinkPlaceholder')}
                              rows={3}
                              autoFocus={!isDisabled}
                            />
                            <p
                              className={`text-xs mt-1 ${isDisabled ? 'text-gray-300' : 'text-text-secondary'}`}
                            >
                              {t('recipes.import.deeplinkHint')}
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
                        <div className="w-full border-t border-border-primary" />
                      </div>
                      <div className="relative flex justify-center text-sm">
                        <span className="px-3 bg-background-primary text-text-secondary font-medium">
                          {t('recipes.import.or')}
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
                              className="block text-sm font-medium text-text-primary mb-3"
                            >
                              {t('recipes.import.fileLabel')}
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
                                className={`text-xs mt-1 ${isDisabled ? 'text-gray-300' : 'text-text-secondary'}`}
                              >
                                {t('recipes.import.fileHint')}
                              </p>
                              <button
                                type="button"
                                onClick={() => setShowSchemaModal(true)}
                                className="text-xs text-blue-500 hover:text-blue-700 underline"
                                disabled={isDisabled}
                              >
                                {t('recipes.import.example')}
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

              <p className="text-xs text-text-secondary">
                {t('recipes.import.reviewHint')}
              </p>
            </div>

            <div className="flex justify-end space-x-3 mt-6">
              <Button type="button" onClick={handleClose} variant="ghost" disabled={importing}>
                {t('common.actions.cancel')}
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
                    {importing || isSubmitting
                      ? t('recipes.import.importing')
                      : t('recipes.import.button')}
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
          <div className="bg-background-primary border border-border-primary rounded-lg p-6 w-[800px] max-w-[90vw] max-h-[80vh] flex flex-col">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-medium text-text-primary">
                {t('recipes.import.structureTitle')}
              </h3>
              <button
                type="button"
                onClick={() => setShowSchemaModal(false)}
                className="text-text-secondary hover:text-text-primary"
              >
                ✕
              </button>
            </div>
            <p className="mt-4 text-blue-700 text-sm">
              {t('recipes.import.structureDescription')}
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
  const { t } = useLocalization();

  return (
    <Button onClick={onClick} variant="default" size="sm" className="flex items-center gap-2">
      <Download className="w-4 h-4" />
      {t('recipes.import.button')}
    </Button>
  );
}
