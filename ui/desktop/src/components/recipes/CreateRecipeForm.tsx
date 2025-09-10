import { useState, useEffect } from 'react';
import { useForm } from '@tanstack/react-form';
import { z } from 'zod';
import { FileText } from 'lucide-react';
import { Button } from '../ui/button';
import { Recipe } from '../../recipe';
import { saveRecipe } from '../../recipe/recipeStorage';
import { toastSuccess, toastError } from '../../toasts';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { RecipeNameField, recipeNameSchema } from './shared/RecipeNameField';
import { generateRecipeNameFromTitle } from './shared/recipeNameUtils';
import { validateJsonSchema, getValidationErrorMessages } from '../../recipe/validation';

interface CreateRecipeFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

// Define Zod schema for the entire form
const createRecipeSchema = z.object({
  title: z.string().min(3, 'Title must be at least 3 characters'),
  description: z.string().min(10, 'Description must be at least 10 characters'),
  instructions: z.string().min(20, 'Instructions must be at least 20 characters'),
  prompt: z.string(),
  activities: z.string(),
  jsonSchema: z.string(),
  recipeName: recipeNameSchema,
  global: z.boolean(),
});

export default function CreateRecipeForm({ isOpen, onClose, onSuccess }: CreateRecipeFormProps) {
  const [creating, setCreating] = useState(false);

  // Handle Esc key for modal
  useEscapeKey(isOpen, onClose);

  const createRecipeForm = useForm({
    defaultValues: {
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: '',
      jsonSchema: '',
      recipeName: '',
      global: true,
    },
    validators: {
      onChange: createRecipeSchema,
    },
    onSubmit: async ({ value }) => {
      setCreating(true);
      try {
        // Parse activities from comma-separated string
        const activities = value.activities
          .split(',')
          .map((activity) => activity.trim())
          .filter((activity) => activity.length > 0);

        // Parse and validate JSON schema if provided
        let jsonSchemaObj = undefined;
        if (value.jsonSchema && value.jsonSchema.trim()) {
          try {
            jsonSchemaObj = JSON.parse(value.jsonSchema.trim());
            // Validate the JSON schema syntax
            const validationResult = validateJsonSchema(jsonSchemaObj);
            if (!validationResult.success) {
              const errorMessages = getValidationErrorMessages(validationResult.errors);
              throw new Error(`Invalid JSON schema: ${errorMessages.join(', ')}`);
            }
          } catch (error) {
            throw new Error(
              `JSON Schema parsing error: ${error instanceof Error ? error.message : 'Invalid JSON'}`
            );
          }
        }

        // Create the recipe object
        const recipe: Recipe = {
          title: value.title.trim(),
          description: value.description.trim(),
          instructions: value.instructions.trim(),
          prompt: value.prompt.trim() || undefined,
          activities: activities.length > 0 ? activities : undefined,
          response: jsonSchemaObj ? { json_schema: jsonSchemaObj } : undefined,
        };

        await saveRecipe(recipe, {
          name: value.recipeName.trim(),
          global: value.global,
        });

        // Reset dialog state
        createRecipeForm.reset({
          title: '',
          description: '',
          instructions: '',
          prompt: '',
          activities: '',
          jsonSchema: '',
          recipeName: '',
          global: true,
        });
        onClose();

        onSuccess();

        toastSuccess({
          title: value.recipeName.trim(),
          msg: 'Recipe created successfully',
        });
      } catch (error) {
        console.error('Failed to create recipe:', error);

        toastError({
          title: 'Create Failed',
          msg: `Failed to create recipe: ${error instanceof Error ? error.message : 'Unknown error'}`,
          traceback: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setCreating(false);
      }
    },
  });

  // Set default example values when the modal opens
  useEffect(() => {
    if (isOpen) {
      // Set example values like the original did
      createRecipeForm.setFieldValue('title', 'Python Development Assistant');
      createRecipeForm.setFieldValue(
        'description',
        'A helpful assistant for Python development tasks including coding, debugging, and code review.'
      );
      createRecipeForm.setFieldValue(
        'instructions',
        `You are an expert Python developer assistant. Help users with:

1. Writing clean, efficient Python code
2. Debugging and troubleshooting issues
3. Code review and optimization suggestions
4. Best practices and design patterns
5. Testing and documentation

Always provide clear explanations and working code examples.

Parameters you can use:
- {{project_type}}: The type of Python project (web, data science, CLI, etc.)
- {{python_version}}: Target Python version`
      );
      createRecipeForm.setFieldValue(
        'prompt',
        'What Python development task can I help you with today?'
      );
      createRecipeForm.setFieldValue('activities', 'coding, debugging, testing, documentation');
      createRecipeForm.setFieldValue('recipeName', 'python-development-assistant');
      createRecipeForm.setFieldValue('global', true);
    }
  }, [isOpen, createRecipeForm]);

  const handleClose = () => {
    // Reset form to default values
    createRecipeForm.reset({
      title: '',
      description: '',
      instructions: '',
      prompt: '',
      activities: '',
      jsonSchema: '',
      recipeName: '',
      global: true,
    });
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[300] flex items-center justify-center bg-black/50">
      <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[700px] max-w-[90vw] max-h-[90vh] overflow-y-auto">
        <h3 className="text-lg font-medium text-text-standard mb-4">Create New Recipe</h3>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            e.stopPropagation();
            createRecipeForm.handleSubmit();
          }}
        >
          <div className="space-y-4">
            <createRecipeForm.Field name="title">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-title"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Title <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="create-title"
                    type="text"
                    value={field.state.value}
                    onChange={(e) => {
                      const value = e.target.value;
                      field.handleChange(value);
                      // Auto-generate recipe name when title changes
                      if (value.trim()) {
                        const suggestedName = generateRecipeNameFromTitle(value);
                        createRecipeForm.setFieldValue('recipeName', suggestedName);
                      }
                    }}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Recipe title"
                    autoFocus
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="description">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-description"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Description <span className="text-red-500">*</span>
                  </label>
                  <input
                    id="create-description"
                    type="text"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Brief description of what this recipe does"
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="instructions">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-instructions"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Instructions <span className="text-red-500">*</span>
                  </label>
                  <textarea
                    id="create-instructions"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder="Detailed instructions for the AI agent..."
                    rows={8}
                  />
                  <p className="text-xs text-text-muted mt-1">
                    Use {`{{parameter_name}}`} to define parameters that users can fill in
                  </p>
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="prompt">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-prompt"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Initial Prompt (Optional)
                  </label>
                  <textarea
                    id="create-prompt"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                    placeholder="First message to send when the recipe starts..."
                    rows={3}
                  />
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="activities">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-activities"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Activities (Optional)
                  </label>
                  <input
                    id="create-activities"
                    type="text"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className="w-full p-3 border border-border-subtle rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500"
                    placeholder="coding, debugging, testing, documentation (comma-separated)"
                  />
                  <p className="text-xs text-text-muted mt-1">
                    Comma-separated list of activities this recipe helps with
                  </p>
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="jsonSchema">
              {(field) => (
                <div>
                  <label
                    htmlFor="create-json-schema"
                    className="block text-sm font-medium text-text-standard mb-2"
                  >
                    Response JSON Schema (Optional)
                  </label>
                  <textarea
                    id="create-json-schema"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm ${
                      field.state.meta.errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
                    }`}
                    placeholder={`{
  "type": "object",
  "properties": {
    "result": {
      "type": "string",
      "description": "The main result"
    }
  },
  "required": ["result"]
}`}
                    rows={6}
                  />
                  <p className="text-xs text-text-muted mt-1">
                    Define the expected structure of the AI's response using JSON Schema format
                  </p>
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-red-500 text-sm mt-1">
                      {typeof field.state.meta.errors[0] === 'string'
                        ? field.state.meta.errors[0]
                        : field.state.meta.errors[0]?.message || String(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="recipeName">
              {(field) => (
                <RecipeNameField
                  id="create-recipe-name"
                  value={field.state.value}
                  onChange={field.handleChange}
                  onBlur={field.handleBlur}
                  errors={field.state.meta.errors.map((error) =>
                    typeof error === 'string' ? error : error?.message || String(error)
                  )}
                />
              )}
            </createRecipeForm.Field>

            <createRecipeForm.Field name="global">
              {(field) => (
                <div>
                  <label className="block text-sm font-medium text-text-standard mb-2">
                    Save Location
                  </label>
                  <div className="space-y-2">
                    <label className="flex items-center">
                      <input
                        type="radio"
                        name="create-save-location"
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
                        name="create-save-location"
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
            </createRecipeForm.Field>
          </div>

          <div className="flex justify-end space-x-3 mt-6">
            <Button type="button" onClick={handleClose} variant="ghost" disabled={creating}>
              Cancel
            </Button>
            <createRecipeForm.Subscribe
              selector={(state) => [state.canSubmit, state.isSubmitting, state.isValid]}
            >
              {([canSubmit, isSubmitting, isValid]) => {
                // Debug logging to see what's happening
                console.log('Form state:', { canSubmit, isSubmitting, isValid });

                return (
                  <Button
                    type="submit"
                    disabled={!canSubmit || creating || isSubmitting}
                    variant="default"
                  >
                    {creating || isSubmitting ? 'Creating...' : 'Create Recipe'}
                  </Button>
                );
              }}
            </createRecipeForm.Subscribe>
          </div>
        </form>
      </div>
    </div>
  );
}

// Export the button component for easy access
export function CreateRecipeButton({ onClick }: { onClick: () => void }) {
  return (
    <Button onClick={onClick} variant="outline" size="sm" className="flex items-center gap-2">
      <FileText className="w-4 h-4" />
      Create Recipe
    </Button>
  );
}
