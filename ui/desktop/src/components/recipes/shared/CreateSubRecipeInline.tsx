import { useState, useCallback } from 'react';
import { useForm } from '@tanstack/react-form';
import { X, Save, Loader2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { toastSuccess, toastError } from '../../../toasts';
import { saveRecipe, getStorageDirectory } from '../../../recipe/recipe_management';
import { Recipe } from '../../../recipe';
import { SubRecipeFormData } from './recipeFormSchema';
import { useEscapeKey } from '../../../hooks/useEscapeKey';
import KeyValueEditor from './KeyValueEditor';

interface CreateSubRecipeInlineProps {
  isOpen: boolean;
  onClose: () => void;
  onSubRecipeSaved: (subRecipe: SubRecipeFormData) => void;
}

export default function CreateSubRecipeInline({
  isOpen,
  onClose,
  onSubRecipeSaved,
}: CreateSubRecipeInlineProps) {
  useEscapeKey(isOpen, onClose);

  const form = useForm({
    defaultValues: {
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: [],
      parameters: [],
      jsonSchema: '',
      subRecipes: [],
    },
  });

  const [name, setName] = useState('');
  const [toolDescription, setToolDescription] = useState('');
  const [sequentialWhenRepeated, setSequentialWhenRepeated] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = useCallback(async () => {
    const formValues = form.state.values;

    if (
      !name.trim() ||
      !formValues.title.trim() ||
      !formValues.description.trim() ||
      !formValues.instructions.trim()
    ) {
      toastError({
        title: 'Validation Failed',
        msg: 'Name, title, recipe description, and instructions are required.',
      });
      return;
    }

    setIsSaving(true);
    try {
      const recipe: Recipe = {
        version: '1.0.0',
        title: formValues.title.trim(),
        description: formValues.description.trim(),
        instructions: formValues.instructions.trim(),
      };

      const sanitizedTitle = formValues.title
        .trim()
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '');

      const savedRecipeId = await saveRecipe(recipe, sanitizedTitle);

      const subRecipe: SubRecipeFormData = {
        name: name.trim(),
        path: `${getStorageDirectory(true)}/${savedRecipeId}.yaml`,
        description: toolDescription.trim() || undefined,
        sequential_when_repeated: sequentialWhenRepeated,
        values: Object.keys(values).length > 0 ? values : undefined,
      };

      toastSuccess({
        title: formValues.title.trim(),
        msg: 'Subrecipe created successfully',
      });

      onSubRecipeSaved(subRecipe);
      onClose();

      form.reset();
      setName('');
      setToolDescription('');
      setSequentialWhenRepeated(false);
      setValues({});
    } catch (error) {
      console.error('Failed to save subrecipe:', error);

      toastError({
        title: 'Save Failed',
        msg: `Failed to save subrecipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
      });
    } finally {
      setIsSaving(false);
    }
  }, [form, name, toolDescription, sequentialWhenRepeated, values, onSubRecipeSaved, onClose]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[500] flex items-center justify-center bg-black/50">
      <div className="bg-background-primary border border-borderSubtle rounded-lg w-[90vw] max-w-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-borderSubtle">
          <div>
            <h2 className="text-xl font-medium text-textProminent">Create New Subrecipe</h2>
            <p className="text-textSubtle text-sm">
              Create a simple recipe to use as a callable tool in your main recipe
            </p>
          </div>
          <Button
            onClick={onClose}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-bgSubtle rounded-lg transition-colors"
            aria-label="Close create subrecipe modal"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
          {/* Name Field */}
          <div>
            <label
              htmlFor="subrecipe-name"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Name <span className="text-text-danger">*</span>
            </label>
            <input
              id="subrecipe-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="e.g., security_scan"
            />
            <p className="text-xs text-text-muted mt-1">
              Unique identifier used to generate the tool name
            </p>
          </div>

          {/* Title Field */}
          <form.Field name="title">
            {(field) => (
              <div>
                <label
                  htmlFor="subrecipe-title"
                  className="block text-sm font-medium text-text-standard mb-2"
                >
                  Recipe Title <span className="text-text-danger">*</span>
                </label>
                <input
                  id="subrecipe-title"
                  type="text"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  onBlur={field.handleBlur}
                  className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring"
                  placeholder="e.g., Security Analysis Tool"
                />
              </div>
            )}
          </form.Field>

          {/* Recipe Description Field */}
          <form.Field name="description">
            {(field) => (
              <div>
                <label
                  htmlFor="recipe-description"
                  className="block text-sm font-medium text-text-standard mb-2"
                >
                  Recipe Description <span className="text-text-danger">*</span>
                </label>
                <input
                  id="recipe-description"
                  type="text"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  onBlur={field.handleBlur}
                  className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring"
                  placeholder="What this recipe does when executed"
                />
              </div>
            )}
          </form.Field>

          {/* Instructions Field */}
          <form.Field name="instructions">
            {(field) => (
              <div>
                <label
                  htmlFor="subrecipe-instructions"
                  className="block text-sm font-medium text-text-standard mb-2"
                >
                  Instructions <span className="text-text-danger">*</span>
                </label>
                <textarea
                  id="subrecipe-instructions"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  onBlur={field.handleBlur}
                  className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring resize-none font-mono text-sm"
                  placeholder="Instructions for the AI when this subrecipe is called..."
                  rows={8}
                />
              </div>
            )}
          </form.Field>

          {/* Tool Description Field */}
          <div>
            <label
              htmlFor="tool-description"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Tool Description
            </label>
            <textarea
              id="tool-description"
              value={toolDescription}
              onChange={(e) => setToolDescription(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring resize-none"
              placeholder="Optional description shown when this is called as a tool"
              rows={2}
            />
          </div>

          {/* Sequential When Repeated */}
          <div className="flex items-center gap-2">
            <input
              id="subrecipe-sequential"
              type="checkbox"
              checked={sequentialWhenRepeated}
              onChange={(e) => setSequentialWhenRepeated(e.target.checked)}
              className="w-4 h-4 border-border-subtle rounded focus:ring-2 focus:ring-ring"
            />
            <label htmlFor="subrecipe-sequential" className="text-sm text-text-standard">
              Sequential when repeated
            </label>
            <span className="text-xs text-text-muted">
              (Forces sequential execution of multiple instances)
            </span>
          </div>

          {/* Pre-configured Values */}
          <div>
            <label className="block text-sm font-medium text-text-standard mb-2">
              Pre-configured Values
            </label>
            <p className="text-xs text-text-muted mb-3">
              Optional parameter values that are always passed to the subrecipe
            </p>
            <KeyValueEditor values={values} onChange={setValues} />
          </div>
        </div>

        {/* Footer */}
        <div className="flex gap-3 p-6 border-t border-borderSubtle justify-end">
          <Button onClick={onClose} variant="outline">
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={
              !name.trim() ||
              !form.state.values.title.trim() ||
              !form.state.values.description.trim() ||
              !form.state.values.instructions.trim() ||
              isSaving
            }
            className="inline-flex items-center gap-2"
          >
            {isSaving ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Creating...
              </>
            ) : (
              <>
                <Save className="w-4 h-4" />
                Create & Add Subrecipe
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
